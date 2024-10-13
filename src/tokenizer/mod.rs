use core::str;
use rustc_hash::FxHashMap;

pub struct Tokenizer {
    tokenized_strings: Vec<Vec<usize>>,
    token_to_string: Vec<String>, // Map from token ID to string
    string_to_token: FxHashMap<String, usize>, // Map from string to token ID
}

impl Tokenizer {
    pub fn new(capacity: usize) -> Tokenizer {
        Tokenizer{
            tokenized_strings: Vec::with_capacity(capacity),
            token_to_string: Vec::new(),
            string_to_token: FxHashMap::default(),
        }
    }

    #[inline(always)]
    pub fn tokenize(&mut self, text: &str) {
        let mut tokens = Vec::with_capacity(text.len());
        let mut start = None;

        for (i, ch) in text.char_indices() {
            if ch.is_ascii_alphanumeric() {
                // If it's part of a word (alphanumeric), accumulate it
                if start.is_none() {
                    start = Some(i);  // Start a new token
                }
            } else {
                if let Some(st) = start.take() {
                    // Process the current alphanumeric token
                    let token = &text[st..i];
                    let token_id = self.get_or_insert_token(token);
                    tokens.push(token_id);
                }
                // Process the current character as its own token
                let token = &text[i..i + ch.len_utf8()];
                let token_id = self.get_or_insert_token(token);
                tokens.push(token_id);
            }
        }

        // Process the last token if necessary
        if let Some(st) = start.take() {
            let token = &text[st..];
            let token_id = self.get_or_insert_token(token);
            tokens.push(token_id);
        }

        // Store the tokenized string (as token IDs)
        self.tokenized_strings.push(tokens);
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
        for tokens in &self.tokenized_strings {
            for &token_id in tokens {
                let count = token_frequencies.entry(token_id).or_insert(0);
                *count += 1;
            }
        }
        token_frequencies
    }
}