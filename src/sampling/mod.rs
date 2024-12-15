use rand::Rng;

pub fn sample_stratified(
    data: &[u8], 
    sample_size: usize, 
    block_size: usize, 
) -> Vec<u8> {
    if data.is_empty() || sample_size == 0 || block_size == 0 {
        return Vec::new();
    }

    let num_strata = ((data.len() as f64 * 1024.0 * 1024.0).log10().round() as usize * 5).max(1);
    let mut rng = rand::thread_rng();
    let mut sample = Vec::with_capacity(sample_size);
    
    // Divide data into equal strata
    let strata_size = data.len() / num_strata;
    
    // Calculate bytes per stratum for sampling
    let bytes_per_stratum = sample_size / num_strata;
    let extra_bytes = sample_size % num_strata;

    for i in 0..num_strata {
        let strata_start = i * strata_size;
        let strata_end = if i == num_strata - 1 { data.len() } else { (i + 1) * strata_size };
        
        if strata_end > strata_start {
            // Adjust bytes for this stratum, distributing any remainder
            let current_stratum_bytes = bytes_per_stratum + (if i < extra_bytes { 1 } else { 0 });
            
            let block_start = strata_start + rng.gen_range(0..strata_size.min(block_size));
            
            let actual_block_size = current_stratum_bytes
                .min(strata_end - block_start)
                .min(sample_size - sample.len());
            
            sample.extend_from_slice(&data[block_start..block_start + actual_block_size]);
        }
        
        if sample.len() >= sample_size {
            break;
        }
    }

    // Truncate to exact sample size if needed
    sample.truncate(sample_size);

    sample
}

pub fn sample_stratified_strings(
    data: &[u8],
    end_positions: &[usize],
    percentage: f64,
    num_segments: usize,
) -> (Vec<u8>, Vec<usize>) {
    assert!(percentage > 0.0 && percentage <= 1.0, "Invalid percentage");
    assert!(num_segments > 0, "Number of segments must be positive");

    let total_strings = end_positions.len();
    let strings_per_segment = (total_strings as f64 / num_segments as f64).ceil() as usize;
    let strings_to_sample_per_segment = ((strings_per_segment as f64) * percentage).round() as usize;

    let mut sampled_data = Vec::new();
    let mut sampled_end_positions = Vec::new();

    for segment_idx in 0..num_segments {
        let start_idx = segment_idx * strings_per_segment;
        let end_idx = ((segment_idx + 1) * strings_per_segment).min(total_strings);

        let segment_indices = start_idx..end_idx;
        for i in segment_indices.clone().take(strings_to_sample_per_segment) {
            let start = if i == 0 { 0 } else { end_positions[i - 1] };
            let end = end_positions[i];
            sampled_data.extend_from_slice(&data[start..end]);
            sampled_end_positions.push(sampled_data.len());
        }
    }

    (sampled_data, sampled_end_positions)
}