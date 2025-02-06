use rustc_hash::FxHashMap;
use serde::{Serialize, Deserialize};
use bucket_fn::Linear;
use cacheline_ef::CachelineEfVec;
use ptr_hash::{PtrHash, PtrHashParams};
use ptr_hash::*;
type PH<Key, BF> = PtrHash<Key, BF, CachelineEfVec, hash::FxHash, Vec<u8>>;

const N_INLINE_SUFFIXES: usize = 4;
const MAX_BUCKET_SIZE: usize = 128;

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
    pub fn insert(&mut self, data: &[u8], id: V) -> bool {
        let length = data.len();

        if length <= 8 {
            let value = bytes_to_u64_le(data, length);
            self.dictionary.insert((value, length as u8), id);
            return true;
        }
        else {
            let prefix = bytes_to_u64_le(data, 8);
            let bucket = self.buckets.entry(prefix).or_default();

            if bucket.len() > MAX_BUCKET_SIZE {
                return false;
            }

            let suffix_len = length - 8;
            let suffix = bytes_to_u64_le(&data[8..], suffix_len);
            
            bucket.push((suffix, suffix_len as u8, id));
            bucket.sort_unstable_by(|&a, &b| b.1.cmp(&a.1));
            return true;
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
        let mut long_buckets = Vec::new();
        
        for (&prefix, bucket) in self.buckets.iter() {
            let (answer_id, answer_length) = self.find_longest_match(&prefix.to_le_bytes()).unwrap();
            let offset = long_buckets.len() as u16;
            let mut n_suffixes: u16 = 0;
            
            let mut inline_suffixes: [u64; N_INLINE_SUFFIXES] = [0; N_INLINE_SUFFIXES];
            let mut inline_lengths: [u8; N_INLINE_SUFFIXES] = [0; N_INLINE_SUFFIXES];
            let mut inline_ids: [V; N_INLINE_SUFFIXES] = [V::default(); N_INLINE_SUFFIXES];

            for i in 0..N_INLINE_SUFFIXES.min(bucket.len()) {
                inline_suffixes[i] = bucket[i].0;
                inline_lengths[i] = bucket[i].1;
                inline_ids[i] = bucket[i].2;
                n_suffixes += 1;
            }

            for &(suffix, len, id) in bucket.iter().skip(N_INLINE_SUFFIXES) {
                long_buckets.push((suffix, len, id));
                n_suffixes += 1;
            }
 
            let info_long_match = LongMatchInfo {
                prefix,
                answer_id,
                answer_length: answer_length as u8,
                n_suffixes,
                inline_suffixes,
                inline_lengths,
                inline_ids,
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

                let info_long_match = LongMatchInfo {
                    prefix,
                    answer_id: id,
                    answer_length: length,
                    n_suffixes: 0,
                    inline_suffixes: [0; N_INLINE_SUFFIXES],
                    inline_lengths: [0; N_INLINE_SUFFIXES],
                    inline_ids: [V::default(); N_INLINE_SUFFIXES],
                    offset: 0,
                };

                long_dictionary.insert(prefix, info_long_match);

                continue;
            }

            short_dictionary.insert((prefix, length), id);
        }

        let prefixes = long_dictionary.keys().copied().collect::<Vec<_>>();
        let long_phf = PH::<_, Linear>::new(&prefixes, PtrHashParams::default());
        let max = prefixes.iter()
            .map(|prefix| long_phf.index(prefix))
            .fold(0, |acc, idx| acc.max(idx));

        let mut long_info = vec![LongMatchInfo::default(); max as usize + 1];
        for (prefix, &p) in long_dictionary.iter() {
            let index = long_phf.index(prefix) as usize;
            long_info[index] = p;
        }

        StaticLongestPrefixMatcher {
            short_dictionary,
            long_phf,
            long_info,
            long_buckets,
        }
    }
}

#[repr(align(64))] // Ensure 64-byte alignment
#[derive(Default, Copy, Clone)]
struct LongMatchInfo<V>
where
    V: Copy + Default + Into<usize>,
{
    pub prefix: u64,
    pub inline_suffixes: [u64; N_INLINE_SUFFIXES],
    pub inline_lengths: [u8; N_INLINE_SUFFIXES],
    pub inline_ids: [V; N_INLINE_SUFFIXES],
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
    long_phf: PH<u64, Linear>,
    long_info: Vec<LongMatchInfo<V>>,
    long_buckets: Vec<(u64, u8, V)>,
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
        let index = self.long_phf.index(&prefix);

        if index >= self.long_info.len() || prefix != self.long_info[index].prefix {
            return None;
        }

        let long_info = &self.long_info[index];

        for i in 0..N_INLINE_SUFFIXES.min(long_info.n_suffixes as usize) {
            let inline_suffix = long_info.inline_suffixes[i as usize];
            let inline_id = long_info.inline_ids[i as usize];
            let inline_len = long_info.inline_lengths[i as usize] as usize;
            if is_prefix(suffix, inline_suffix, suffix_len, inline_len) {
                return Some((inline_id, 8 + inline_len));
            }
        }

        if long_info.n_suffixes as usize > N_INLINE_SUFFIXES {
            let start = long_info.offset as usize;
            let end = start + long_info.n_suffixes as usize - N_INLINE_SUFFIXES;

            for i in start..end {
                let item = &self.long_buckets[i];
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
