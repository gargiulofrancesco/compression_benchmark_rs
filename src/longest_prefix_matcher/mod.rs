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

const MIN_MATCH: usize = 8;

pub struct LongestPrefixMatcher<V> {
    long_match_buckets: FxHashMap<(u64, u8), Vec<V>>,     // mapping from prefix (8 bytes) to bucket of dictionary IDs
    short_match_lookup: FxHashMap<(u64, u8), V>,          // mapping from prefix (1-8 bytes) to dictionary ID 
    dictionary: Vec<u8>,
    end_positions: Vec<u32>,
}

impl<V> LongestPrefixMatcher<V> 
where 
    V: Copy + Into<usize>,
{   
    pub fn new() -> Self {
        Self {
            long_match_buckets: FxHashMap::default(),
            short_match_lookup: FxHashMap::default(),
            dictionary: Vec::with_capacity(1024 * 1024),
            end_positions: vec![0],
        }
    }

    #[inline]
    pub fn insert(&mut self, entry: &[u8], id: V) {
        if entry.len() > MIN_MATCH {
            let prefix = Self::bytes_to_u64_le(&entry, MIN_MATCH);
            self.dictionary.extend_from_slice(&entry[MIN_MATCH..]);
            self.end_positions.push(self.dictionary.len() as u32);

            let bucket = self.long_match_buckets.entry((prefix, MIN_MATCH as u8)).or_default();
            bucket.push(id);
            bucket.sort_unstable_by(|&id1, &id2| {
                let len1 = self.end_positions[id1.into() + 1] as usize 
                           - self.end_positions[id1.into()] as usize;
                let len2 = self.end_positions[id2.into() + 1] as usize 
                           - self.end_positions[id2.into()] as usize;
                len2.cmp(&len1)
            });
        } else {
            let prefix = Self::bytes_to_u64_le(&entry, entry.len());
            self.short_match_lookup.insert((prefix, entry.len() as u8), id);
            self.end_positions.push(self.dictionary.len() as u32);
        }
    }

    #[inline]
    pub fn find_longest_match(&self, data: &[u8]) -> Option<(V, usize)> {
        // Long match handling
        if data.len() > MIN_MATCH {
            let prefix = Self::bytes_to_u64_le(&data, MIN_MATCH);
            
            if let Some(bucket) = self.long_match_buckets.get(&(prefix, MIN_MATCH as u8)) {
                for &id in bucket {
                    let dict_start = self.end_positions[id.into()] as usize;
                    let dict_end = self.end_positions[id.into() + 1] as usize;
                    let length = dict_end - dict_start;
                    if data[MIN_MATCH..].starts_with(&self.dictionary[dict_start..dict_end]) {
                        return Some((id, MIN_MATCH + length));
                    }
                }
            }
        }

        // Short match handling
        for length in (1..=MIN_MATCH.min(data.len())).rev() {
            let prefix = Self::bytes_to_u64_le(&data, length);
            
            if let Some(&id) = self.short_match_lookup.get(&(prefix, length as u8)) {
                return Some((id, length));
            }
        }

        None
    }

    #[inline]
    fn bytes_to_u64_le(bytes: &[u8], len: usize) -> u64 {
        let ptr = bytes.as_ptr();
        let value = unsafe {
            *(ptr as *const u64)
        };

        value & MASKS[len]
    }
}