use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

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
pub struct StaticLongestPrefixMatcher<V> {
    dictionary: FxHashMap<(u64, u8), V>,
    long_dictionary: FxHashMap<u64, (u16, u8, V, u8, u64, u8, u64, u8)>, // prefix, start, length, answer, answer_length, first_suffix, first_suffix_len, last_suffix, last_suffix_len
    buckets: Vec<(u64, u8, V)>,                                          // start, num, id, lenght
}

impl<V> StaticLongestPrefixMatcher<V>
where
    V: Copy + Default + Into<usize>,
{
    #[inline]
    pub fn find_longest_match(&self, data: &[u8]) -> Option<(V, usize)> {
        // Long match handling
        if data.len() > 8 {
            let suffix_len = data.len().min(16) - 8;
            let prefix = bytes_to_u64_le(&data, 8);
            let suffix = bytes_to_u64_le(&data[8..], suffix_len);

            if let Some(&(
                start,
                bucket_length,
                answer,
                answer_len,
                first_suffix,
                first_len,
                second_suffix,
                second_len,
            )) = self.long_dictionary.get(&prefix)
            {
                if is_prefix(suffix, first_suffix, suffix_len, first_len as usize) {
                    return Some((answer, 8 + first_len as usize));
                }

                if is_prefix(suffix, second_suffix, suffix_len, second_len as usize) {
                    return Some((answer, 8 + second_len as usize));
                }

                for &(entry_suffix, entry_suffix_len, entry_id) in
                    self.buckets[start as usize..start as usize + bucket_length as usize].iter()
                {
                    if is_prefix(suffix, entry_suffix, suffix_len, entry_suffix_len as usize) {
                        return Some((entry_id, 8 + entry_suffix_len as usize));
                    }
                }
                return Some((answer, answer_len as usize));
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

        None
    }
}

impl<V> From<LongestPrefixMatcher<V>> for StaticLongestPrefixMatcher<V>
where
    V: Copy + Default + Into<usize>,
{
    fn from(lpm: LongestPrefixMatcher<V>) -> Self {
        let mut long_dictionary = FxHashMap::default();
        let mut buckets = Vec::new();

        for (&prefix, bucket) in lpm.buckets.iter() {
            let (answer, answer_length) = lpm.find_longest_match(&prefix.to_le_bytes()).unwrap();
            let first_suffix = bucket[0].0 .0;
            let first_suffix_len = bucket[0].0 .1;
            let last_suffix = if bucket.len() > 1 {
                bucket[1].0 .0
            } else {
                first_suffix
            };
            let last_suffix_len = if bucket.len() > 1 {
                bucket[1].0 .1
            } else {
                first_suffix_len
            };

            long_dictionary.insert(
                prefix,
                (
                    buckets.len() as u16,
                    bucket.len() as u8 - bucket.len().min(2) as u8,
                    answer,
                    answer_length as u8,
                    first_suffix,
                    first_suffix_len,
                    last_suffix,
                    last_suffix_len,
                ),
            );

            for &((suffix, suffix_len), id) in bucket.iter().skip(2) {
                buckets.push((suffix, suffix_len, id));
            }
        }

        Self {
            dictionary: lpm.dictionary,
            long_dictionary,
            buckets,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct LongestPrefixMatcher<V> {
    dictionary: FxHashMap<(u64, u8), V>,
    buckets: FxHashMap<u64, Vec<((u64, u8), V)>>,
}

impl<V> LongestPrefixMatcher<V>
where
    V: Copy + Into<usize>,
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
        } else {
            let prefix = bytes_to_u64_le(data, 8);
            let suffix_len = length - 8;
            let suffix = bytes_to_u64_le(&data[8..], suffix_len);
            let bucket = self.buckets.entry(prefix).or_default();
            bucket.push((((suffix, suffix_len as u8)), id));
            bucket.sort_unstable_by(|&a, &b| b.0 .1.cmp(&a.0 .1));
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
                for &((entry_suffix, entry_suffix_len), entry_id) in bucket {
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

        None
    }
}

#[inline(always)]
fn bytes_to_u64_le(bytes: &[u8], len: usize) -> u64 {
    let ptr = bytes.as_ptr();
    let value = unsafe { *(ptr as *const u64) };

    value & MASKS[len]
}

#[inline(always)]
fn is_prefix(text: u64, prefix: u64, text_size: usize, prefix_size: usize) -> bool {
    prefix_size <= text_size && ((prefix ^ text) & MASKS[prefix_size]) == 0
    //prefix_size <= text_size && shared_prefix_size(text, prefix) >= prefix_size
}

#[inline(always)]
fn shared_prefix_size(a: u64, b: u64) -> usize {
    ((a ^ b).trailing_zeros() >> 3) as usize
}
