use core::str;
use rustc_hash::FxHashMap;

pub struct Tokenizer {
    data: Vec<usize>,
    end_positions: Vec<usize>,
    token_to_string: Vec<String>, // Map from token ID to string
    string_to_token: FxHashMap<String, usize>, // Map from string to token ID
}

impl Tokenizer {
    pub fn new(data_size: usize, n_items: usize) -> Tokenizer {
        Tokenizer{
            data: Vec::with_capacity(data_size),
            end_positions: Vec::with_capacity(n_items),
            token_to_string: Vec::new(),
            string_to_token: FxHashMap::default(),
        }
    }

    pub fn tokenize(&mut self, data: &[u8], end_positions: &[usize]) {
        let mut prev_end = 0;

        // Iterate over the ending positions to process each string section
        for &end in end_positions {
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
                        let token_id = self.get_or_insert_token(token);
                        self.data.push(token_id);
                        tokens_added += 1;
                    }
                    // Process the current non-alphanumeric character as its own token
                    let token = &text[i..i + ch.len_utf8()];
                    let token_id = self.get_or_insert_token(token);
                    self.data.push(token_id);
                    tokens_added += 1;
                }
            }

            // Process the last token if necessary
            if let Some(st) = start.take() {
                let token = &text[st..];
                let token_id = self.get_or_insert_token(token);
                self.data.push(token_id);
                tokens_added += 1;
            }

            // Update the end_positions
            self.end_positions.push(self.end_positions.last().cloned().unwrap_or(0) + tokens_added);
        }
    }

    #[inline(always)]
    fn get_or_insert_token(&mut self, token: &str) -> usize {
        if let Some(&token_id) = self.string_to_token.get(token) {
            return token_id; // Return existing token ID if it's already in the dictionary
        }
        // Otherwise, insert the new token
        let token_id = self.token_to_string.len();
        self.token_to_string.push(token.to_string());
        self.string_to_token.insert(token.to_string(), token_id);
        token_id
    }

    pub fn get_dictionary(&self) -> &Vec<String> {
        &self.token_to_string
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