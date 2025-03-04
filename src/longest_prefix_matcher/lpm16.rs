use rustc_hash::FxHashMap;
use serde::{Serialize, Deserialize};
use bucket_fn::Linear;
use cacheline_ef::CachelineEfVec;
use ptr_hash::{PtrHash, PtrHashParams};
use ptr_hash::*;
type PH<Key, BF> = PtrHash<Key, BF, CachelineEfVec, hash::FxHash, Vec<u8>>;

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
pub struct LongestPrefixMatcher {
    dictionary: FxHashMap<(u64, u8), u16>, 
    buckets: FxHashMap<u64, Vec<(u64, u8, u16)>>, 
}

impl LongestPrefixMatcher {   
    pub fn new() -> Self {
        Self {
            dictionary: FxHashMap::default(),
            buckets: FxHashMap::default(),
        }
    }

    #[inline]
    pub fn insert(&mut self, data: &[u8], id: u16) -> bool {
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
    pub fn find_longest_match(&self, data: &[u8]) -> Option<(u16, usize)> {
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

    pub fn finalize(&self) -> (StaticLongestPrefixMatcher, Vec<u16>) {
        let mut long_dictionary = FxHashMap::default();
        let mut buckets = Vec::new();
        let mut current_id = 0;
        let mut remap_ids: Vec<u16> = vec![u16::MAX; 1 << 16]; // map: new_id -> old_id

        for (&prefix, bucket) in self.buckets.iter() {
            let (answer_id, answer_length) = self.find_longest_match(&prefix.to_le_bytes()).unwrap();
            let offset = buckets.len() as u16;
            let mut n_suffixes: u8 = 0;
            let mut inline_suffixes: [u64; 2] = [bucket[0].0; 2];
            let mut inline_suffixes_len: [u8; 2] = [bucket[0].1; 2];

            if bucket.len() > 1 {
                inline_suffixes[1] = bucket[1].0;
                inline_suffixes_len[1] = bucket[1].1;

                remap_ids[current_id] = bucket[1].2;
                remap_ids[current_id + 1] = bucket[0].2;
                current_id += 2;
            }
            else {
                remap_ids[current_id] = bucket[0].2;
                current_id += 1;
            }

            let base_id = current_id as u16;

            for &(suffix, len, id) in bucket.iter().skip(2) {
                buckets.push((suffix, len));
                n_suffixes += 1;

                remap_ids[current_id] = id;
                current_id += 1;
            }

            assert!(
                n_suffixes < 128,
                "Number of suffixes is too high because we are packing their number within 7 bits"
            );

            let lengths = InfoLongMatch::encode_lengths(
                answer_length as u8,
                n_suffixes,
                inline_suffixes_len,
            );

            let info_long_match = InfoLongMatch {
                prefix,
                inline_suffixes,
                lengths,
                offset,
                base_id,
                answer_id,
            };

            long_dictionary.insert(prefix, info_long_match);
        }

        let mut short_dictionary = FxHashMap::default();
        for (&(prefix, length), &id) in self.dictionary.iter() {
            remap_ids[current_id] = id;
            current_id += 1;

            if length == 8 {
                if long_dictionary.contains_key(&prefix) {
                    continue;
                }

                let lengths = InfoLongMatch::encode_lengths(8, 0, [1, 1]);

                let info_long_match = InfoLongMatch {
                    prefix,
                    inline_suffixes: [0, 0],
                    lengths,
                    offset: 0,
                    base_id: 0,
                    answer_id: id,
                };

                long_dictionary.insert(prefix, info_long_match);

                continue;
            }

            short_dictionary.insert((prefix, length), id);
        }

        let prefixes = long_dictionary.keys().copied().collect::<Vec<_>>();
        let mphf = PH::<_, Linear>::new(&prefixes, PtrHashParams::default_fast());
        let max = prefixes.iter()
            .map(|prefix| mphf.index(prefix))
            .fold(0, |acc, idx| acc.max(idx));

        let mut reverse_remap_ids: Vec<u16> = vec![u16::MAX; 1 << 16]; // map: old_id -> new_id
        for new_id in 0..current_id {
            let old_id = remap_ids[new_id] as usize;
            reverse_remap_ids[old_id] = new_id as u16;
        }

        // Remap short dictionary
        for ((_,_), old_id) in short_dictionary.iter_mut() {
            *old_id = reverse_remap_ids[*old_id as usize];
        }

        // Remap long dictionary
        let mut long_info = vec![InfoLongMatch::default(); max as usize + 1];
        for (prefix, p) in long_dictionary.iter_mut() {
            p.answer_id = reverse_remap_ids[p.answer_id as usize];
            let index = mphf.index(prefix) as usize;
            long_info[index] = *p;
        }

        let static_lpm = StaticLongestPrefixMatcher {
            short_dictionary,
            long_dictionary: mphf,
            long_info,
            buckets,
        };

        (static_lpm, remap_ids)
    }
}

#[repr(align(32))] // Ensure 32-byte alignment
#[derive(Default, Copy, Clone)]
struct InfoLongMatch {
    pub prefix: u64,
    pub inline_suffixes: [u64; 2],
    pub lengths: u16,
    pub offset: u16, 
    pub base_id: u16,
    pub answer_id: u16,
}

impl InfoLongMatch {
    #[inline]
    fn decode_lengths(lengths: u16) -> (u8, u8, [u8; 2]) {
        let answer_length = (lengths >> (16 - 3)) as u8 + 1;
        let number_suffixes = (lengths >> (16 - 3 - 7)) as u8 & 0b1111111;
        let first_suffix_lengths = ((lengths >> (16 - 3 - 7 - 3)) & 0b111) as u8 + 1;
        let second_suffix_lengths = ((lengths >> (16 - 3 - 7 - 3 - 3)) & 0b111) as u8 + 1;
        (
            answer_length,
            number_suffixes,
            [first_suffix_lengths, second_suffix_lengths],
        )
    }

    #[inline]
    fn encode_lengths(
        answer_length: u8,
        number_suffixes: u8,
        first_suffixes_lengths: [u8; 2],
    ) -> u16 {
        let mut res = (answer_length as u16 - 1) << (16 - 3); // value in [1, 8] using 3 bits
        res |= (number_suffixes as u16) << (16 - 3 - 7); // value in [0, 128) using 7 bits
        res |= (first_suffixes_lengths[0] as u16 - 1) << (16 - 3 - 7 - 3); // value in [1, 8) using 3 bits
        res |= (first_suffixes_lengths[1] as u16 - 1) << (16 - 3 - 7 - 3 - 3); // value in [1, 8) using 3 bits

        res
    }
}

pub struct StaticLongestPrefixMatcher {
    short_dictionary: FxHashMap<(u64, u8), u16>,
    long_dictionary: PH<u64, Linear>,
    long_info: Vec<InfoLongMatch>,
    buckets: Vec<(u64, u8)>,
}

impl StaticLongestPrefixMatcher {
    #[inline]
    pub fn find_longest_match(&self, data: &[u8]) -> Option<(u16, usize)> {
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
    pub fn compute_long_answer(&self, prefix: u64, suffix: u64, suffix_len: usize) -> Option<(u16, usize)> {
        let index = self.long_dictionary.index(&prefix);

        if index >= self.long_info.len() || prefix != self.long_info[index].prefix {
            return None;
        }

        let long_info = &self.long_info[index];
        let (answer_length, number_suffixes, inline_suffixes_len) = InfoLongMatch::decode_lengths(long_info.lengths);

        // First inlined suffix
        let inline_suffix = long_info.inline_suffixes[0];
        if is_prefix(suffix, inline_suffix, suffix_len, inline_suffixes_len[0] as usize) {
            return Some((long_info.base_id - 1, 8 + inline_suffixes_len[0] as usize));
        }

        // Second inlined suffix
        let inline_suffix = long_info.inline_suffixes[1];
        if is_prefix(suffix, inline_suffix, suffix_len, inline_suffixes_len[1] as usize) {
            return Some((long_info.base_id - 2, 8 + inline_suffixes_len[1] as usize));
        }

        for i in 0..number_suffixes {
            let item_pos = long_info.offset as usize + i as usize;
            let item = &self.buckets[item_pos];
            if is_prefix(suffix, item.0, suffix_len, item.1 as usize) {
                return Some((long_info.base_id + i as u16, 8 + item.1 as usize));
            }
        }
        
        return Some((long_info.answer_id, answer_length as usize));
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
