use rustc_hash::FxHashMap;

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
        }
        else {
            let prefix = bytes_to_u64_le(data, 8);
            let suffix_len = length - 8;
            let suffix = bytes_to_u64_le(&data[8..], suffix_len);
            let bucket = self.buckets.entry(prefix).or_default();
            bucket.push((((suffix, suffix_len as u8)), id));
            bucket.sort_unstable_by(|&a, &b| b.0.1.cmp(&a.0.1));
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
