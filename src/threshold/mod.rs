pub struct Threshold {
    threshold: u16,                     // The dynamic threshold value
    target_sample_size: usize,          // Target number of bytes to process before stopping
    current_sample_size: usize,         // Total bytes processed so far
    tokens_to_insert: usize,            // Number of tokens needed to fully populate the dictionary 
    update_period: usize,               // How many token insertions before we update the threshold
    current_update_merges: usize,       // Number of tokens inserted in the current update batch
    current_update_bytes: usize,        // Number of bytes processed in the current update batch
}    

impl Threshold {
    pub fn new(target_sample_size: usize, tokens_to_insert: usize, update_period: usize) -> Self {
        Threshold {
            threshold: 0,
            target_sample_size,
            current_sample_size: 0, 
            tokens_to_insert,
            update_period,
            current_update_merges: 0,
            current_update_bytes: 0,
        }
    }

    #[inline]
    pub fn get(&self) -> u16 {
        self.threshold
    }

    #[inline]
    pub fn update(&mut self, match_length: usize, did_merge: bool) {
        self.current_update_bytes += match_length;
        self.current_sample_size += match_length;

        if did_merge {
            self.tokens_to_insert -= 1;
            self.current_update_merges += 1;

            if self.current_update_merges == self.update_period {
                let bytes_per_token = (self.current_update_bytes as f64 / self.current_update_merges as f64).ceil() as usize;
                let predicted_missing_bytes = self.tokens_to_insert * bytes_per_token;
                let predicted_sample_size = self.current_sample_size + predicted_missing_bytes;

                if predicted_sample_size > self.target_sample_size {
                    self.threshold = self.threshold.saturating_sub(1);
                }
                else if predicted_sample_size < self.target_sample_size {
                    self.threshold = if self.threshold < u16::MAX - 1 { self.threshold + 1 } else { u16::MAX - 1 };
                }

                self.current_update_bytes = 0;
                self.current_update_merges = 0;
            }
        }
    }
}