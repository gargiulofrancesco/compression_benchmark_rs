use super::Compressor;
use crate::bit_vector::BitVector;
use rustc_hash::FxHashMap;

const THRESHOLD: usize = 10;

pub struct OnPairCompressor {
    data: BitVector,                            // Store the compressed data as bytes
    item_end_positions: Vec<usize>,             // Store the end positions of each compressed item
    dictionary: Vec<u8>,                        // Store the dictionary
    dictionary_end_positions: Vec<u32>,         // Store the end positions of each element in the dictionary
    bits_per_token: usize,                      // Number of bits required to represent a token 
}

impl Compressor for OnPairCompressor {
    fn new(data_size: usize, n_elements: usize) -> Self {
        OnPairCompressor {
            data: BitVector::with_capacity(data_size * 8),
            item_end_positions: Vec::with_capacity(n_elements),
            dictionary: Vec::new(),
            dictionary_end_positions: Vec::new(),
            bits_per_token: 0,
        }
    }

    fn compress(&mut self, data: &[u8], end_positions: &[usize]) {
        let trie = OnPairCompressor::train(data, end_positions);
        self.bits_per_token = (f64::log2(trie.len() as f64)).ceil() as usize;
        self.parse(data, end_positions, &trie);
    }

    fn decompress(&self, buffer: &mut Vec<u8>) {
        let mut pos = 0;
        while pos < self.data.len() {
            let token_id = self.data.get_bits(pos, self.bits_per_token).unwrap() as usize;
            pos += self.bits_per_token;

            let start = self.dictionary_end_positions[token_id] as usize;
            let end = self.dictionary_end_positions[token_id + 1] as usize;
            
            buffer.extend(&self.dictionary[start..end]);
        }
    }

    fn get_item_at(&mut self, index: usize, buffer: &mut Vec<u8>) {
        let start = self.item_end_positions[index];
        let end = self.item_end_positions[index + 1];

        let mut pos = start;
        while pos < end {
            let token_id = self.data.get_bits(pos, self.bits_per_token).unwrap() as usize;
            pos += self.bits_per_token;

            let start = self.dictionary_end_positions[token_id] as usize;
            let end = self.dictionary_end_positions[token_id + 1] as usize;
            
            buffer.extend(&self.dictionary[start..end]);
        }
    }

    fn space_used_bytes(&self) -> usize {
        (self.data.len() / 8) + self.dictionary.len() + self.dictionary_end_positions.len() * 4
    }

    fn name(&self) -> &str {
        "On-Pair"
    }
}

impl OnPairCompressor {
    fn train(data: &[u8], end_positions: &[usize]) -> Trie {
        let mut frequency: FxHashMap<(usize, usize), usize> = FxHashMap::default();
        let mut trie = Trie::new();
        let mut next_token_id = 256;
    
        // Initialize the dictionary with single-byte tokens
        for i in 0..256 {
            let token = vec![i as u8];
            trie.insert(&token, i);
        }

        let mut start = 0;
        let mut pos = 0;
        
        for &end in end_positions.iter() {
            if start == end {
                continue;
            }
    
            let (match_length, match_token_id) = trie.find_longest_match(data, pos, end);
            let mut previous_token_id = match_token_id.unwrap();
            let mut previous_length = match_length;

            pos = start + previous_length;
    
            while pos < end {
                // Find the longest match in the Trie
                let (match_length, match_token_id) = trie.find_longest_match(data, pos, end);
                let match_token_id = match_token_id.unwrap();
    
                 // Update token frequency and possibly merge tokens
                *frequency.entry((previous_token_id, match_token_id)).or_insert(0) += 1;
    
                if frequency[&(previous_token_id, match_token_id)] > THRESHOLD {
                    let merged_token = &data[pos - previous_length..pos + match_length];
                    trie.insert(merged_token, next_token_id);
                    next_token_id += 1;
                    frequency.remove(&(previous_token_id, match_token_id));
                }
            
                previous_token_id = match_token_id;
                previous_length = match_length;
    
                pos += match_length;
            }
    
            start = end;
        }
    
        trie
    }
    
    fn parse(&mut self, data: &[u8], end_positions: &[usize], trie: &Trie) {
        // Initialize dictionary metadata
        self.dictionary_end_positions.push(0);
        self.item_end_positions.push(0);
    
        let mut dictionary_map: FxHashMap<usize, usize> = FxHashMap::default();
        let mut next_token_id = 0;
    
        let mut start = 0;
        for &end in end_positions.iter() {
            if start == end {
                self.item_end_positions.push(self.data.len());
                continue;
            }
    
            let mut pos = start;
            while pos < end {
                // Find the longest match in the Trie
                let (length, match_token_id) = trie.find_longest_match(data, pos, end);
                let match_token_id = match_token_id.unwrap();
    
                if let Some(&existing_token_id) = dictionary_map.get(&match_token_id) {
                    self.data.append_bits(existing_token_id as u64, self.bits_per_token);
                } else {
                    self.data.append_bits(next_token_id as u64, self.bits_per_token);
                    dictionary_map.insert(match_token_id, next_token_id);
    
                    self.dictionary.extend(&data[pos..pos + length]);
                    self.dictionary_end_positions.push(self.dictionary.len() as u32);
    
                    next_token_id += 1;
                }
    
                pos += length;
            }
    
            self.item_end_positions.push(self.data.len());
            start = end;
        }
    }    
}

#[derive(Default)]
struct TrieNode {
    children: FxHashMap<u8, TrieNode>,
    token_id: Option<usize>,
}

struct Trie {
    root: TrieNode,
    n: usize,
}

impl Trie {
    fn new() -> Self {
        Trie {
            root: TrieNode::default(),
            n: 0,
        }
    }

    fn insert(&mut self, sequence: &[u8], token_id: usize) {
        let mut node = &mut self.root;
        for &byte in sequence {
            node = node.children.entry(byte).or_insert_with(TrieNode::default);
        }
        node.token_id = Some(token_id);
        self.n += 1;
    }

    fn find_longest_match(&self, data: &[u8], start: usize, end: usize) -> (usize, Option<usize>) {
        let mut node = &self.root;
        let mut longest_match_len = 0;
        let mut last_token_id = None;

        for (i, &byte) in data[start..end].iter().enumerate() {
            if let Some(next_node) = node.children.get(&byte) {
                node = next_node;
                if let Some(token_id) = node.token_id {
                    longest_match_len = i + 1;
                    last_token_id = Some(token_id);
                }
            } else {
                break;
            }
        }

        (longest_match_len, last_token_id)
    }

    fn len(&self) -> usize {
        self.n
    }
}    