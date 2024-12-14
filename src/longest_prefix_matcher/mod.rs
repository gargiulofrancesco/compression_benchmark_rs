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
    long_match_buckets: FxHashMap<u64, Vec<(Vec<u8>, V)>>,        // mapping from hash to bucket of dictionary entries
    short_match_lookup: FxHashMap<u64, V>,                        // mapping from prefix (1-8 bytes) to dictionary ID 
}

impl<V> LongestPrefixMatcher<V> 
where 
    V: Copy,
{   
    pub fn new() -> Self {        
        let mut long_match_buckets: FxHashMap<u64, Vec<(Vec<u8>, V)>> = FxHashMap::default();
        let mut short_match_lookup: FxHashMap<u64, V> = FxHashMap::default();

        // Preallocate to reduce dynamic resizing
        long_match_buckets.reserve(u16::MAX as usize);
        short_match_lookup.reserve(u16::MAX as usize);

        Self {
            long_match_buckets,
            short_match_lookup,
        }
    }

    #[inline]
    pub fn insert(&mut self, entry: &[u8], id: V) {
        if entry.len() > MIN_MATCH {
            let prefix_u64 = Self::bytes_to_u64_le(&entry, MIN_MATCH);
            let suffix_entry = entry[MIN_MATCH..].to_vec();

            self.long_match_buckets
                .entry(prefix_u64)
                .or_default()
                .push((suffix_entry, id));

            let bucket = self.long_match_buckets.get_mut(&prefix_u64).unwrap();
            bucket.sort_by(|a, b| b.0.len().cmp(&a.0.len())); 
        } else {
            let prefix_u64 = Self::bytes_to_u64_le(&entry, entry.len());
            self.short_match_lookup.insert(prefix_u64, id);
        }
    }

    pub fn with_dictionary(dictionary: &mut Vec<(Vec<u8>, V)>) -> Self {        
        let mut long_match_buckets: FxHashMap<u64, Vec<(Vec<u8>, V)>> = FxHashMap::default();
        let mut short_match_lookup: FxHashMap<u64, V> = FxHashMap::default();

        // Preallocate to reduce dynamic resizing
        long_match_buckets.reserve(dictionary.len());
        short_match_lookup.reserve(dictionary.len());

        // Sort dictionary by length in descending order to optimize matching
        dictionary.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        // Separate and preprocess dictionary entries
        for (entry, id) in dictionary.iter() {
            if entry.len() > MIN_MATCH {
                let prefix_u64 = Self::bytes_to_u64_le(entry, MIN_MATCH);
                let suffix_entry = entry[MIN_MATCH..].to_vec();

                long_match_buckets
                    .entry(prefix_u64)
                    .or_default()
                    .push((suffix_entry, *id));
            } else {
                let prefix_u64 = Self::bytes_to_u64_le(&entry, entry.len());
                short_match_lookup.insert(prefix_u64, *id);
            }
        }

        Self {
            long_match_buckets,
            short_match_lookup,
        }
    }

    #[inline]
    pub fn find_longest_match(&self, data: &[u8]) -> Option<(V, usize)> {
        // Long match handling
        if data.len() > MIN_MATCH {
            let prefix_u64 = Self::bytes_to_u64_le(&data, MIN_MATCH);
            
            if let Some(bucket) = self.long_match_buckets.get(&prefix_u64) {
                for (entry, id) in bucket {
                    if data[MIN_MATCH..].starts_with(entry) {
                        return Some((*id, entry.len() + MIN_MATCH));
                    }
                }
            }
        }

        // Short match handling
        for length in (1..=MIN_MATCH.min(data.len())).rev() {
            let prefix_u64 = Self::bytes_to_u64_le(&data, length);
            
            if let Some(&id) = self.short_match_lookup.get(&prefix_u64) {
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