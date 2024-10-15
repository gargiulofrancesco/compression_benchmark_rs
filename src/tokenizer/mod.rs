use rustc_hash::FxHashMap;

pub struct Tokenizer {
    data: Vec<usize>,
    item_end_positions: Vec<usize>,
    token_to_bytes: Vec<u8>,                    // Map from token ID to byte sequences
    token_end_positions: Vec<usize>,            // End positions of each token byte sequence
    bytes_to_token: FxHashMap<Vec<u8>, usize>,  // Map from byte sequences to token ID
}

impl Tokenizer {
    pub fn new(data_size: usize, n_items: usize) -> Tokenizer {
        Tokenizer{
            data: Vec::with_capacity(data_size),
            item_end_positions: Vec::with_capacity(n_items),
            token_to_bytes: Vec::with_capacity(8 * (1024 * 1024)),
            token_end_positions: Vec::with_capacity(512 * 1024),
            bytes_to_token: FxHashMap::default(),
        }
    }

    pub fn tokenize(&mut self, data: &[u8], item_end_positions: &[usize]) {
        let mut prev_end = 0;

        // Iterate over the ending positions to process each string section
        for &end in item_end_positions {
            // Extract the substring for the current section
            let slice = &data[prev_end..end];
            prev_end = end;

            // Convert byte slice to string using unsafe
            let text = unsafe { std::str::from_utf8_unchecked(slice) };

            // Tokenize the current string
            let mut tokens_added = 0;
            let mut start = None;
            for (i, ch) in text.char_indices() {
                if ch.is_ascii_alphanumeric() {
                    // Accumulate alphanumeric characters as part of a token
                    if start.is_none() {
                        start = Some(i);  // Start a new token
                    }
                } else {
                    if let Some(st) = start.take() {
                        // Process the current alphanumeric token
                        let token = &text[st..i];
                        let token_id = self.get_or_insert_token(token.as_bytes());
                        self.data.push(token_id);
                        tokens_added += 1;
                    }
                    // Process the current non-alphanumeric character as its own token
                    let token = &text[i..i + ch.len_utf8()];
                    let token_id = self.get_or_insert_token(token.as_bytes());
                    self.data.push(token_id);
                    tokens_added += 1;
                }
            }

            // Process the last token if necessary
            if let Some(st) = start.take() {
                let token = &text[st..];
                let token_id = self.get_or_insert_token(token.as_bytes());
                self.data.push(token_id);
                tokens_added += 1;
            }

            // Update the item_end_positions
            self.item_end_positions.push(self.item_end_positions.last().cloned().unwrap_or(0) + tokens_added);
        }
    }

    #[inline(always)]
    fn get_or_insert_token(&mut self, token: &[u8]) -> usize {
        if let Some(&token_id) = self.bytes_to_token.get(token) {
            return token_id;
        }

        let token_id = self.token_end_positions.len();
        let end_offset = self.token_to_bytes.len() + token.len();  // Calculate end position
        self.token_to_bytes.extend_from_slice(token);
        self.token_end_positions.push(end_offset);  // Push end position

        self.bytes_to_token.insert(token.to_vec(), token_id);
        token_id
    }

    pub fn get_dictionary(&self) -> Vec<&[u8]> {
        std::iter::once(&0)
            .chain(self.token_end_positions.iter())
            .collect::<Vec<_>>()
            .windows(2)
            .map(|w| &self.token_to_bytes[*w[0]..*w[1]])
            .collect()
    }

    pub fn get_tokens_frequencies(&self) -> FxHashMap<usize, usize> {
        let mut token_frequencies: FxHashMap<usize, usize> = FxHashMap::default();
        for &token in self.data.iter() {
            let count = token_frequencies.entry(token).or_insert(0);
            *count += 1;
        }
        token_frequencies
    }
}