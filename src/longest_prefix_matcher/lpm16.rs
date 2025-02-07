use rustc_hash::FxHashMap;
use serde::{Serialize, Deserialize};
use bucket_fn::Linear;
use cacheline_ef::CachelineEfVec;
use ptr_hash::{PtrHash, PtrHashParams};
use ptr_hash::*;
type PH<Key, BF> = PtrHash<Key, BF, CachelineEfVec, hash::FxHash, Vec<u8>>;

const N_INLINE_SUFFIXES_LONG: usize = 4;
const N_INLINE_SUFFIXES_MEDIUM: usize = 7;
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
        // Entries of length [9, 16]
        let mut long_dictionary = FxHashMap::default();
        let mut long_buckets = Vec::new();
        for (&prefix, bucket) in self.buckets.iter() {
            let (answer_id, answer_length) = self.find_longest_match(&prefix.to_le_bytes()).unwrap();
            let offset = long_buckets.len() as u16;
            let mut n_suffixes: u16 = 0;
            
            let mut inline_suffixes: [u64; N_INLINE_SUFFIXES_LONG] = [0; N_INLINE_SUFFIXES_LONG];
            let mut inline_lengths: [u8; N_INLINE_SUFFIXES_LONG] = [0; N_INLINE_SUFFIXES_LONG];
            let mut inline_ids: [V; N_INLINE_SUFFIXES_LONG] = [V::default(); N_INLINE_SUFFIXES_LONG];

            for i in 0..N_INLINE_SUFFIXES_LONG.min(bucket.len()) {
                inline_suffixes[i] = bucket[i].0;
                inline_lengths[i] = bucket[i].1;
                inline_ids[i] = bucket[i].2;
                n_suffixes += 1;
            }

            for &(suffix, len, id) in bucket.iter().skip(N_INLINE_SUFFIXES_LONG) {
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

        // Entries of length [1, 3]: explicitly store answers
        let mut short_answer: Vec<(V, u8)> = vec![(V::default(), 0); 1 << 24];
        for prefix in 0u64..(1 << 24) {
            let prefix_len = ((64 - prefix.leading_zeros() + 7) / 8).max(1) as usize;
            let prefix_slice = &prefix.to_le_bytes()[0..prefix_len];
            if let Some((id, len)) = self.find_longest_match(prefix_slice){
                short_answer[prefix as usize] = (id, len as u8);
            }
        }

        // Entries of length [4, 8]
        let mut self_medium_dictionary: FxHashMap<u64, Vec<(u32, u8, V)>> = FxHashMap::default();
        for (&(prefix, length), &id) in self.dictionary.iter() {
            if length == 8 {
                // Entries of length 8 are inserted in `long_dictionary`
                if long_dictionary.contains_key(&prefix) {
                    continue;
                }

                let info_long_match = LongMatchInfo {
                    prefix,
                    answer_id: id,
                    answer_length: length,
                    n_suffixes: 0,
                    inline_suffixes: [0; N_INLINE_SUFFIXES_LONG],
                    inline_lengths: [0; N_INLINE_SUFFIXES_LONG],
                    inline_ids: [V::default(); N_INLINE_SUFFIXES_LONG],
                    offset: 0,
                };

                long_dictionary.insert(prefix, info_long_match);
            }
            else if length >= 4 {
                // Entries of length [4, 7] are inserted in `self_medium_dictionary`
                let key = prefix & MASKS[4];
                let suffix = (prefix >> 32) as u32;
                let suffix_len = length - 4;
                self_medium_dictionary.entry(key).or_insert_with(Vec::new).push((suffix, suffix_len, id)); 
            }
        }

        let mut medium_dictionary = FxHashMap::default();
        let mut medium_buckets = Vec::new();
        for (&prefix, bucket) in self_medium_dictionary.iter_mut() {
            bucket.sort_unstable_by(|&a, &b| b.1.cmp(&a.1));

            let (answer_id, answer_length) = self.find_longest_match(&prefix.to_le_bytes()[0..4]).unwrap();
            let offset = medium_buckets.len() as u16;
            let mut n_suffixes: u16 = 0;
            
            let mut inline_suffixes: [u32; N_INLINE_SUFFIXES_MEDIUM] = [0; N_INLINE_SUFFIXES_MEDIUM];
            let mut inline_lengths: [u8; N_INLINE_SUFFIXES_MEDIUM] = [0; N_INLINE_SUFFIXES_MEDIUM];
            let mut inline_ids: [V; N_INLINE_SUFFIXES_MEDIUM] = [V::default(); N_INLINE_SUFFIXES_MEDIUM];

            for i in 0..N_INLINE_SUFFIXES_MEDIUM.min(bucket.len()) {
                inline_suffixes[i] = bucket[i].0;
                inline_lengths[i] = bucket[i].1;
                inline_ids[i] = bucket[i].2;
                n_suffixes += 1;
            }

            for &(suffix, len, id) in bucket.iter().skip(N_INLINE_SUFFIXES_MEDIUM) {
                medium_buckets.push((suffix, len, id));
                n_suffixes += 1;
            }
 
            let info_medium_match = MediumMatchInfo {
                prefix: prefix as u32,
                answer_id,
                answer_length: answer_length as u8,
                n_suffixes,
                inline_suffixes,
                inline_lengths,
                inline_ids,
                offset,
            };

            medium_dictionary.insert(prefix, info_medium_match);
        }

        let medium_prefixes = medium_dictionary.keys().copied().collect::<Vec<_>>();
        let medium_phf = PH::<_, Linear>::new(&medium_prefixes, PtrHashParams::default());
        let medium_max = medium_prefixes.iter()
            .map(|prefix| medium_phf.index(prefix))
            .fold(0, |acc, idx| acc.max(idx));

        let mut medium_info = vec![MediumMatchInfo::default(); medium_max as usize + 1];
        for (prefix, &p) in medium_dictionary.iter() {
            let index = medium_phf.index(prefix) as usize;
            medium_info[index] = p;
        }

        let long_prefixes = long_dictionary.keys().copied().collect::<Vec<_>>();
        let long_phf = PH::<_, Linear>::new(&long_prefixes, PtrHashParams::default());
        let long_max = long_prefixes.iter()
            .map(|prefix| long_phf.index(prefix))
            .fold(0, |acc, idx| acc.max(idx));

        let mut long_info = vec![LongMatchInfo::default(); long_max as usize + 1];
        for (prefix, &p) in long_dictionary.iter() {
            let index = long_phf.index(prefix) as usize;
            long_info[index] = p;
        }

        StaticLongestPrefixMatcher {
            short_answer,
            long_phf,
            long_info,
            long_buckets,
            medium_phf,
            medium_info,
            medium_buckets
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
    pub inline_suffixes: [u64; N_INLINE_SUFFIXES_LONG],
    pub inline_lengths: [u8; N_INLINE_SUFFIXES_LONG],
    pub inline_ids: [V; N_INLINE_SUFFIXES_LONG],
    pub n_suffixes: u16,
    pub offset: u16,
    pub answer_id: V,
    pub answer_length: u8,
}

#[repr(align(64))] // Ensure 64-byte alignment
#[derive(Default, Copy, Clone)]
struct MediumMatchInfo<V>
where
    V: Copy + Default + Into<usize>,
{
    pub prefix: u32,
    pub inline_suffixes: [u32; N_INLINE_SUFFIXES_MEDIUM],
    pub inline_lengths: [u8; N_INLINE_SUFFIXES_MEDIUM],
    pub inline_ids: [V; N_INLINE_SUFFIXES_MEDIUM],
    pub n_suffixes: u16,
    pub offset: u16,
    pub answer_id: V,
    pub answer_length: u8,
}


pub struct StaticLongestPrefixMatcher<V>
where
    V: Copy + Default + Into<usize>,
{
    short_answer: Vec<(V, u8)>,
    long_phf: PH<u64, Linear>,
    long_info: Vec<LongMatchInfo<V>>,
    long_buckets: Vec<(u64, u8, V)>,
    medium_phf: PH<u64, Linear>,
    medium_info: Vec<MediumMatchInfo<V>>,
    medium_buckets: Vec<(u32, u8, V)>,
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

        // Medium match handling
        if data.len() >= 4 {
            let suffix_len = data.len().min(7) - 4;
            let prefix = bytes_to_u64_le(&data, 4);
            let suffix = bytes_to_u64_le(&data[4..], suffix_len);

            let medium_answer = self.compute_medium_answer(prefix, suffix, suffix_len);
            if medium_answer.is_some() {
                return medium_answer;
            }
        }

        // Short match handling
        let len = data.len().min(3);
        let prefix = bytes_to_u64_le(&data, len) as usize;
        let (id, len) = self.short_answer[prefix];
        return Some((id, len as usize));
    }

    #[inline]
    pub fn compute_long_answer(&self, prefix: u64, suffix: u64, suffix_len: usize) -> Option<(V, usize)> {
        let index = self.long_phf.index(&prefix);

        if index >= self.long_info.len() || prefix != self.long_info[index].prefix {
            return None;
        }

        let long_info = &self.long_info[index];

        for i in 0..N_INLINE_SUFFIXES_LONG.min(long_info.n_suffixes as usize) {
            let inline_suffix = long_info.inline_suffixes[i as usize];
            let inline_id = long_info.inline_ids[i as usize];
            let inline_len = long_info.inline_lengths[i as usize] as usize;
            if is_prefix(suffix, inline_suffix, suffix_len, inline_len) {
                return Some((inline_id, 8 + inline_len));
            }
        }

        if long_info.n_suffixes as usize > N_INLINE_SUFFIXES_LONG {
            let start = long_info.offset as usize;
            let end = start + long_info.n_suffixes as usize - N_INLINE_SUFFIXES_LONG;

            for i in start..end {
                let item = &self.long_buckets[i];
                if is_prefix(suffix, item.0, suffix_len, item.1 as usize) {
                    return Some((item.2, 8 + item.1 as usize));
                }
            }
        }

        return Some((long_info.answer_id, long_info.answer_length as usize));
    }

    #[inline]
    pub fn compute_medium_answer(&self, prefix: u64, suffix: u64, suffix_len: usize) -> Option<(V, usize)> {
        let index = self.medium_phf.index(&prefix);

        if index >= self.medium_info.len() || prefix != self.medium_info[index].prefix as u64 {
            return None;
        }

        let medium_info = &self.medium_info[index];

        for i in 0..N_INLINE_SUFFIXES_MEDIUM.min(medium_info.n_suffixes as usize) {
            let inline_suffix = medium_info.inline_suffixes[i as usize];
            let inline_id = medium_info.inline_ids[i as usize];
            let inline_len = medium_info.inline_lengths[i as usize] as usize;
            if is_prefix(suffix, inline_suffix as u64, suffix_len, inline_len) {
                return Some((inline_id, 4 + inline_len));
            }
        }

        if medium_info.n_suffixes as usize > N_INLINE_SUFFIXES_MEDIUM {
            let start = medium_info.offset as usize;
            let end = start + medium_info.n_suffixes as usize - N_INLINE_SUFFIXES_MEDIUM;

            for i in start..end {
                let item = &self.medium_buckets[i];
                if is_prefix(suffix, item.0 as u64, suffix_len, item.1 as usize) {
                    return Some((item.2, 4 + item.1 as usize));
                }
            }
        }

        return Some((medium_info.answer_id, medium_info.answer_length as usize));
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
