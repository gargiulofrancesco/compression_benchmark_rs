use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::arch::x86_64::*;

use bucket_fn::{BucketFn, Linear};
use cacheline_ef::CachelineEfVec;
use ptr_hash::{hash::*, PtrHash, PtrHashParams, Sharding};
use ptr_hash::{
    hash::{Hasher, Murmur2_64},
    pack::Packed,
    *,
};
use std::collections::{HashMap, HashSet};
type PH<Key, BF> = PtrHash<Key, BF, CachelineEfVec, hash::FxHash, Vec<u8>>;

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

// 2+2+1+8+8+2+2+1=26 bytes but it uses 32 bytes due to alignments. When add key, it'll be 34. need to save 2 bytes
// 20000*26=520000 bytes = 520KB

#[derive(Default, Copy, Clone)]
struct InfoLongMatch<V>
where
    V: Copy + Default + Into<usize>,
{
    key: u64, // Needed when using perfect hashing
    answer_id: V,
    answer_length: u8,
    base_id: V,             // if of the first element in the block
    first_suffixes: u64, //[u64; 2], // first suffix and second suffix. The first suffix id is base_id - 1, secondo suffix id is base_id-2. So, if second one does not exist, it is equal to the first one and its id is not wasted.
    first_suffixes_len: u8, //[u8; 2], // first suffix length and second suffix length
    offset: u16,         // offset in the buckets_suffix and buckets_length
    number_suffixes: u8, // number of suffixes in the block
}

impl<V> InfoLongMatch<V>
where
    V: Copy + Default + Into<usize>,
{
    #[inline]
    pub fn compute_answer(
        &self,
        key: u64,
        text: u64,
        text_len: usize,
        suffixes: &[u64],
        lengths: &[u64],
        masks: &[u64],
    ) -> Option<(V, usize)> {
        if key != self.key {
            return None;
        }

        // return Some((self.answer_id, self.answer_length as usize));

        // println!("suffix: {:064b} len: {}", text, text_len);
        // for i in 0..=1 {
        // println!(
        //     "match{i}: {:064b} len: {}",
        //     self.first_suffixes[i], self.first_suffixes_len[i]
        // );

        if is_prefix(
            text,
            self.first_suffixes,
            text_len,
            self.first_suffixes_len as usize,
        ) {
            return Some((self.base_id, 8 + self.first_suffixes_len as usize));
            // FIXME: it is base_id -1 or -2!
        };
        // }

        let mut start = self.offset as usize;
        let end = start + self.number_suffixes as usize;

        let mut numbers = self.number_suffixes as i32;

        for i in start..end {
            // println!("match:  {:064b} len: {}", suffixes[i], lengths[i]);
            unsafe {
                let curr_length = *lengths.get_unchecked(i) as usize;
                let curr_suffix = *suffixes.get_unchecked(i);

                if is_prefix(text, curr_suffix, text_len, curr_length) {
                    return Some((self.base_id, 8 + curr_length));
                }
            }
        }

        // // println!("numbers {}", numbers);
        // while numbers > 0 {
        //     let pos = is_prefix_16_avx(start, suffixes, lengths, masks, text, text_len as u8);

        //     if pos < 8 as usize {
        //         if start + pos >= end {
        //             break;
        //         }
        //         return Some((V::default(), 8 + lengths[start + pos] as usize));
        //     }
        //     numbers -= 8;
        //     start += 8;
        // }

        return Some((self.answer_id, self.answer_length as usize));
    }
}

pub struct StaticLongestPrefixMatcher<V>
where
    V: Copy + Default + Into<usize>,
{
    dictionary: FxHashMap<(u64, u8), V>,
    long_dictionary: PH<u64, Linear>,
    long_info: Vec<InfoLongMatch<V>>,
    buckets_suffix: Vec<u64>,
    buckets_length: Vec<u64>,
    masks: Vec<u64>, // start, num, id, length
}

// Convert a __m256i to a String of bits
unsafe fn print_mm256(vec: __m256i) {
    // Convert __m256i to an array of 4 u64 elements
    let arr: [u64; 4] = std::mem::transmute(vec);

    // Iterate through each 64-bit integer and collect its bits as a string
    let s = arr
        .iter()
        .rev() // Reverse to ensure the bit order matches the memory layout
        .map(|&val| format!("{:064b}", val))
        .collect::<Vec<_>>()
        .join(" ");
    println!("{}", s);
}

#[inline]
pub fn is_prefix_16_avx(
    offset: usize,
    buckets_suffix: &[u64],
    buckets_length: &[u64],
    masks: &[u64],
    suffix: u64,
    suffix_len: u8,
) -> usize {
    let mut res = 1 << 17;

    //println!("suffix: {:064b} len: {}", suffix, suffix_len);
    unsafe {
        let zero_vec = _mm256_setzero_si256();
        let all_ones = _mm256_set1_epi64x(-1i64);

        for i in 0..2 {
            let suffixes_ptr = buckets_suffix.as_ptr().add(offset + i * 4);
            let mut results = _mm256_loadu_si256(suffixes_ptr as *const __m256i);

            let lengths_ptr = buckets_length.as_ptr().add(offset + i * 4);
            let lengths = _mm256_loadu_si256(lengths_ptr as *const __m256i);

            let bit_counts = _mm256_slli_epi64(lengths, 3);
            let masks = _mm256_sllv_epi64(all_ones, bit_counts);

            let vec = _mm256_set1_epi64x(suffix as i64);
            results = _mm256_xor_si256(results, vec);

            // let masks_ptr = masks.as_ptr().add(offset + i * 4);
            // let masks = _mm256_loadu_si256(masks_ptr as *const __m256i);
            results = _mm256_and_si256(results, masks);

            let lengths = _mm256_cmpgt_epi64(_mm256_set1_epi64x(suffix_len as i64 + 1), lengths);

            let cmp = _mm256_cmpeq_epi64(results, zero_vec);

            let cmp = _mm256_and_si256(cmp, lengths);
            let mask = _mm256_movemask_pd(_mm256_castsi256_pd(cmp));

            res |= mask << (i * 4);
        }
    }
    res.trailing_zeros() as usize
}

// #[inline]
// pub fn is_prefix_16_avx(
//     buckets_suffix: &[u64; 16],
//     buckets_length: &[u8; 16],
//     suffix: u64,
//     suffix_len: u8,
// ) -> usize {
//     // use shuffle? lengths are 4 bits
//     let mut masks = [0; 16];
//     for i in 0..16 {
//         masks[i] = u64::MAX >> (buckets_length[i] as usize * 8); //MASKS[buckets_length[i] as usize];
//     }

//     let mut res = 0_i32;
//     unsafe {
//         let zero_vec = _mm256_setzero_si256();

//         for i in 0..4 {
//             let mut results =
//                 _mm256_loadu_si256(buckets_suffix[i * 4..(i + 1) * 4].as_ptr() as *const __m256i);
//             let vec = _mm256_set1_epi64x(suffix as i64);
//             results = _mm256_xor_si256(results, vec);
//             results = _mm256_and_si256(
//                 results,
//                 _mm256_loadu_si256(masks[i * 4..(i + 1) * 4].as_ptr() as *const __m256i),
//             );
//             let cmp = _mm256_cmpeq_epi64(results, zero_vec);
//             let mask = _mm256_movemask_pd(_mm256_castsi256_pd(cmp));
//             res |= mask << (i * 4);
//         }
//     }

//     res.trailing_zeros() as usize
// }

#[inline]
pub fn is_prefix_16(
    buckets_suffix: &[u64],
    buckets_length: &[u8],
    buckets_mask: &[u64],
    suffix: u64,
    suffix_len: u8,
) -> usize {
    let mut results = vec![0; 16];
    let mut bit: u16 = u16::MAX;
    let mut bot: u16 = 0;

    // println!("suffix: {:064b} len: {}", suffix, suffix_len);
    for i in 0..16 {
        // println!("result: {:064b} len: {}", results[i], buckets_length[i]);
        results[i] = (buckets_suffix[i] ^ suffix) & buckets_mask[i];
        // println!(
        //     "MASKS:  {:064b} bu len {}",
        //     MASKS[buckets_length[i] as usize], buckets_length[i]
        // );
        // println!("result: {:064b} len: {}", results[i], buckets_length[i]);
        bit |= if buckets_length[i] <= suffix_len as u8 {
            1 << i
        } else {
            0
        };
        bot |= if results[i] == 0 { 1 << i } else { 0 };
        // println!("bit: {:016b} bots: {:016b}", bit, bot);
    }

    (bit & bot).trailing_zeros() as usize
}

/*
Create masks by cmp length l of keys with suffix_len s

se l >= s

l l l ... 3 2 1 0 epi_8
<=
s s s ... s s s s
-----------------
1 1 1 ... 1 1 1 1

*/

#[inline]
pub fn is_prefix_8(
    buckets_suffix: &[u64; 8],
    buckets_length: &[u8; 8],
    suffix: u64,
    suffix_len: u8,
) -> usize {
    let mut results = buckets_suffix.clone();
    let mut bit: u16 = u16::MAX;
    let mut bot: u16 = 0;

    // use shuffle? lengths are 4 bits
    let mut masks = [0; 16];
    for i in 0..8 {
        masks[i] = u64::MAX >> (buckets_length[i] as usize * 8); //MASKS[buckets_length[i] as usize];
    }

    // println!("suffix: {:064b} len: {}", suffix, suffix_len);
    for i in 0..8 {
        // println!("result: {:064b} len: {}", results[i], buckets_length[i]);
        results[i] = (results[i] ^ suffix) & masks[i];
        // println!(
        //     "MASKS:  {:064b} bu len {}",
        //     MASKS[buckets_length[i] as usize], buckets_length[i]
        // );
        // println!("result: {:064b} len: {}", results[i], buckets_length[i]);
        bit |= if buckets_length[i] <= suffix_len as u8 {
            1 << i
        } else {
            0
        };
        bot |= if results[i] == 0 { 1 << i } else { 0 };
        // println!("bit: {:016b} bots: {:016b}", bit, bot);
    }

    (bit & bot).trailing_zeros() as usize
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

            let index = self.long_dictionary.index(&prefix);

            if let Some(answer) = self.long_info[index].compute_answer(
                prefix,
                suffix,
                suffix_len,
                &self.buckets_suffix,
                &self.buckets_length,
                &self.masks,
            ) {
                return Some(answer);
            }
        }

        // Short match handling
        let mut prefix = bytes_to_u64_le(&data, 8);
        for length in (1..=7.min(data.len())).rev() {
            prefix = prefix & MASKS[length];
            if let Some(&id) = self.dictionary.get(&(prefix, length as u8)) {
                return Some((id, length));
            }
        }

        Some((V::default(), 0))
    }
}

impl<V> From<LongestPrefixMatcher<V>> for StaticLongestPrefixMatcher<V>
where
    V: Copy + Default + Into<usize>,
{
    fn from(lpm: LongestPrefixMatcher<V>) -> Self {
        let mut long_dictionary = FxHashMap::default();
        let mut buckets_suffix = Vec::new();
        let mut buckets_length = Vec::new();
        let mut masks = Vec::new();

        let p = InfoLongMatch {
            key: 0,
            answer_id: V::default(),
            answer_length: 0,
            base_id: V::default(),
            first_suffixes: 0,     //[0, 0],
            first_suffixes_len: 0, // [0, 0],
            offset: 0,
            number_suffixes: 0,
        };
        println!("Size of InfoLongMatch: {}", std::mem::size_of_val(&p));

        for (&prefix, bucket) in lpm.buckets.iter() {
            let (answer_id, answer_length) = lpm.find_longest_match(&prefix.to_le_bytes()).unwrap();
            let first_suffix = bucket[0].0 .0;
            let first_suffix_len = bucket[0].0 .1;
            // let second_suffix = if bucket.len() > 1 {
            //     bucket[1].0 .0
            // } else {
            //     first_suffix
            // };
            // let second_suffix_len = if bucket.len() > 1 {
            //     bucket[1].0 .1
            // } else {
            //     first_suffix_len
            // };

            assert!(buckets_suffix.len() < 1 << 16);
            let offset = buckets_suffix.len() as u16;
            let mut number_suffixes = 0;

            for &((suffix, suffix_len), _id) in bucket.iter().skip(1) {
                buckets_suffix.push(suffix);
                buckets_length.push(suffix_len as u64);
                masks.push(MASKS[suffix_len as usize]);
                number_suffixes += 1;
            }

            let info_long_match = InfoLongMatch {
                key: prefix,
                answer_id,
                answer_length: answer_length as u8,
                base_id: V::default(),                // FIXME
                first_suffixes: first_suffix,         //[first_suffix, second_suffix],
                first_suffixes_len: first_suffix_len, //[first_suffix_len, second_suffix_len],
                offset,
                number_suffixes,
            };

            long_dictionary.insert(prefix, info_long_match);
        }

        // add fake data to the end of the buckets to avoid bounds checking
        for _ in 0..15 {
            buckets_suffix.push(0);
            buckets_length.push(0);
            masks.push(0);
        }

        let mut dictionary = FxHashMap::default();
        for (&(key, length), &id) in lpm.dictionary.iter() {
            if length == 8 {
                if long_dictionary.contains_key(&key) {
                    let info = long_dictionary.get(&key).unwrap();
                    assert_eq!(info.answer_length, 8);
                    continue;
                }

                let info_long_match = InfoLongMatch {
                    key,
                    answer_id: id,
                    answer_length: 8 as u8,
                    base_id: V::default(), // FIXME
                    first_suffixes: 0,     //[0, 0],
                    first_suffixes_len: 0, //[0, 0],
                    offset: 0,
                    number_suffixes: 0,
                };

                long_dictionary.insert(key, info_long_match);

                continue;
            }
            dictionary.insert((key, length), id);
        }

        let keys = long_dictionary.keys().copied().collect::<Vec<_>>();
        println!(
            "N keys {} long dict len {}",
            keys.len(),
            long_dictionary.len()
        );

        let mphf = //<PtrHash>::new(&keys, PtrHashParams::default());
        PH::<_, Linear>::new(
            &keys,
            PtrHashParams {
                c: 3.0,
                alpha: 0.98,
                print_stats: true,
                slots_per_part: 1 << 12,
                keys_per_shard: 1 << 33,
                sharding: Sharding::None,
                ..Default::default()
            },
        );

        let mut taken = vec![false; 2 * keys.len()];
        let mut max = 0;
        for key in &keys {
            let idx = mphf.index(key);
            if idx > max {
                max = idx;
            }
            assert_eq!(taken[idx as usize], false);
            taken[idx as usize] = true;
        }

        println!(
            "Max: {} long dictionary size: {}",
            max,
            long_dictionary.len()
        );

        let mut long_info = vec![InfoLongMatch::default(); max as usize + 1];
        for (key, &p) in long_dictionary.iter() {
            let index = mphf.index(key) as usize;

            long_info[index] = p;
        }

        Self {
            dictionary,
            long_dictionary: mphf,
            long_info,
            buckets_suffix,
            buckets_length,
            masks,
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
