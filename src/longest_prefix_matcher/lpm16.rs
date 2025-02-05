use rustc_hash::FxHashMap;
use serde::{Serialize, Deserialize};
use bucket_fn::Linear;
use cacheline_ef::CachelineEfVec;
use ptr_hash::{PtrHash, PtrHashParams};
use ptr_hash::*;
type PH<Key, BF> = PtrHash<Key, BF, CachelineEfVec, hash::FxHash, Vec<u8>>;

const N_INLINE_SUFFIXES: usize = 3;

const MASKS: [u64; 9] = [
    0x0000000000000000, // 0 bytes
    0x00000000000000FF, // 1 byte
    0x000000000000FFFF, // 2 bytes
    0x0000000000FFFFFF, // 3 bytes
    0x00000000FFFFFFFF, // 4 bytes
    0x000000FFFFFFFFFF, // 5 bytes
    0x0000FFFFFFFFFFFF, // 6 bytes
    0x00FFFFFFFFFFFFFF, // 7 bytes
    0xFFFFFFFFFFFFFFFF, // 8 bytes
];

#[derive(Serialize, Deserialize)]
pub struct LongestPrefixMatcher<V> {
    dictionary: FxHashMap<(u64, u8), V>, 
    buckets: FxHashMap<u64, Vec<(u64, u8, V)>>, 
}

impl<V> LongestPrefixMatcher<V> 
where 
    V: Copy + Into<usize> + std::default::Default,
{   
    pub fn new() -> Self {
        Self {
            dictionary: FxHashMap::default(),
            buckets: FxHashMap::default(),
        }
    }

    #[inline]
    pub fn insert(&mut self, data: &[u8], id: V) {
        let length = data.len();

        if length <= 8 {
            let value = bytes_to_u64_le(data, length);
            self.dictionary.insert((value, length as u8), id);
        }
        else {
            let prefix = bytes_to_u64_le(data, 8);
            let suffix_len = length - 8;
            let suffix = bytes_to_u64_le(&data[8..], suffix_len);
            let bucket = self.buckets.entry(prefix).or_default();
            bucket.push((suffix, suffix_len as u8, id));
            bucket.sort_unstable_by(|&a, &b| b.1.cmp(&a.1));
        }
    }

    #[inline]
    pub fn find_longest_match(&self, data: &[u8]) -> Option<(V, usize)> {
        // Long match handling
        if data.len() > 8 {
            let suffix_len = data.len().min(16) - 8;
            let prefix = bytes_to_u64_le(&data, 8);
            let suffix = bytes_to_u64_le(&data[8..], suffix_len);
            
            if let Some(bucket) = self.buckets.get(&prefix) {
                for &(entry_suffix, entry_suffix_len, entry_id) in bucket {
                    if is_prefix(suffix, entry_suffix, suffix_len, entry_suffix_len as usize) {
                        return Some((entry_id, 8 + entry_suffix_len as usize));
                    }
                }
            }
        }

        // Short match handling
        let mut prefix = bytes_to_u64_le(&data, 8);
        for length in (1..=8.min(data.len())).rev() {
            prefix = prefix & MASKS[length];
            if let Some(&id) = self.dictionary.get(&(prefix, length as u8)) {
                return Some((id, length));
            }
        }

        unreachable!("A match is guaranteed to be found before this is reached.");
    }

    pub fn finalize(&self) -> StaticLongestPrefixMatcher<V> {
        let mut long_dictionary = FxHashMap::default();
        let mut buckets = Vec::new();

        for (&prefix, bucket) in self.buckets.iter() {
            let (answer_id, answer_length) = self.find_longest_match(&prefix.to_le_bytes()).unwrap();
            let offset = buckets.len() as u16;
            let mut n_suffixes: u16 = 0;
            let mut inline_suffixes: [(u64, u8, V); N_INLINE_SUFFIXES] = [(0, 0, V::default()); N_INLINE_SUFFIXES];
            
            for i in 0..N_INLINE_SUFFIXES.min(bucket.len()) {
                let suffix = bucket[i].0;
                let len = bucket[i].1;
                let id = bucket[i].2;
                inline_suffixes[i] = (suffix, len, id);
                n_suffixes += 1;
            }

            for &(suffix, len, id) in bucket.iter().skip(N_INLINE_SUFFIXES) {
                buckets.push((suffix, len, id));
                n_suffixes += 1;
            }
 
            let info_long_match = InfoLongMatch {
                prefix,
                answer_id,
                answer_length: answer_length as u8,
                n_suffixes,
                inline_suffixes,
                offset,
            };

            long_dictionary.insert(prefix, info_long_match);
        }

        let mut short_dictionary = FxHashMap::default();
        for (&(prefix, length), &id) in self.dictionary.iter() {
            if length == 8 {
                if long_dictionary.contains_key(&prefix) {
                    continue;
                }

                let info_long_match = InfoLongMatch {
                    prefix,
                    answer_id: id,
                    answer_length: length,
                    n_suffixes: 0,
                    inline_suffixes: [(0, 0, V::default()); N_INLINE_SUFFIXES],
                    offset: 0,
                };

                long_dictionary.insert(prefix, info_long_match);

                continue;
            }
            short_dictionary.insert((prefix, length), id);
        }

        let prefixes = long_dictionary.keys().copied().collect::<Vec<_>>();
        let mphf = PH::<_, Linear>::new(&prefixes, PtrHashParams::default());
        let max = prefixes.iter()
            .map(|prefix| mphf.index(prefix))
            .fold(0, |acc, idx| acc.max(idx));

        let mut long_info = vec![InfoLongMatch::default(); max as usize + 1];
        for (prefix, &p) in long_dictionary.iter() {
            let index = mphf.index(prefix) as usize;
            long_info[index] = p;
        }

        StaticLongestPrefixMatcher {
            short_dictionary,
            long_dictionary: mphf,
            long_info,
            buckets,
        }
    }
}

#[repr(align(64))] // Ensure 64-byte alignment
#[derive(Default, Copy, Clone)]
struct InfoLongMatch<V>
where
    V: Copy + Default + Into<usize>,
{
    pub prefix: u64,
    pub inline_suffixes: [(u64, u8, V); N_INLINE_SUFFIXES],
    pub n_suffixes: u16,
    pub offset: u16, 
    pub answer_id: V,
    pub answer_length: u8,
}

pub struct StaticLongestPrefixMatcher<V>
where
    V: Copy + Default + Into<usize>,
{
    short_dictionary: FxHashMap<(u64, u8), V>,
    long_dictionary: PH<u64, Linear>,
    long_info: Vec<InfoLongMatch<V>>,
    buckets: Vec<(u64, u8, V)>,
}

impl<V> StaticLongestPrefixMatcher<V>
where
    V: Copy + Default + Into<usize>,
{
    #[inline]
    pub fn find_longest_match(&self, data: &[u8]) -> Option<(V, usize)> {
        // Long match handling
        if data.len() >= 8 {
            let suffix_len = data.len().min(16) - 8;
            let prefix = bytes_to_u64_le(&data, 8);
            let suffix = bytes_to_u64_le(&data[8..], suffix_len);

            let long_answer = self.compute_long_answer(prefix, suffix, suffix_len);
            if long_answer.is_some() {
                return long_answer;
            }
        }

        // Short match handling
        let mut prefix = bytes_to_u64_le(&data, 8);
        for length in (1..=7.min(data.len())).rev() {
            prefix = prefix & MASKS[length];
            if let Some(&id) = self.short_dictionary.get(&(prefix, length as u8)) {
                return Some((id, length));
            }
        }

        unreachable!("A match is guaranteed to be found before this is reached.");
    }

    #[inline]
    pub fn compute_long_answer(&self, prefix: u64, suffix: u64, suffix_len: usize) -> Option<(V, usize)> {
        let index = self.long_dictionary.index(&prefix);
        let long_info = &self.long_info[index];

        if prefix != long_info.prefix {
            return None;
        }

        for i in 0..N_INLINE_SUFFIXES.min(long_info.n_suffixes as usize) {
            let inline_suffix = &long_info.inline_suffixes[i as usize];
            if is_prefix(suffix, inline_suffix.0, suffix_len, inline_suffix.1 as usize) {
                return Some((inline_suffix.2, 8 + inline_suffix.1 as usize));
            }
        }

        if long_info.n_suffixes as usize > N_INLINE_SUFFIXES {
            let start = long_info.offset as usize;
            let end = start + long_info.n_suffixes as usize - N_INLINE_SUFFIXES;

            for i in start..end {
                let item = &self.buckets[i];
                if is_prefix(suffix, item.0, suffix_len, item.1 as usize) {
                    return Some((item.2, 8 + item.1 as usize));
                }
            }
        }

        return Some((long_info.answer_id, long_info.answer_length as usize));
    }
}

#[inline(always)]
fn bytes_to_u64_le(bytes: &[u8], len: usize) -> u64 {
    let ptr = bytes.as_ptr();
    let value = unsafe {
        *(ptr as *const u64)
    };

    value & MASKS[len]
}

#[inline(always)]
fn is_prefix(text: u64, prefix: u64, text_size: usize, prefix_size: usize) -> bool {
    prefix_size <= text_size && shared_prefix_size(text, prefix) >= prefix_size
}

#[inline(always)]
fn shared_prefix_size(a: u64, b: u64) -> usize {
    ((a ^ b).trailing_zeros() >> 3) as usize
}
