use std::arch::x86_64::*;

use crate::bit_vector::BitVector;

const MASKS: [([u8; 16], usize); 256] = initialize_masks();

const fn initialize_masks() -> [([u8; 16], usize); 256] {
    let mut masks = [([0u8; 16], 0usize); 256];
    let mut control_byte: u8 = 0;

    loop {
        let mut mask = [0u8; 16];
        let mut length = 8;

        let mut input_pos = 0;
        let mut mask_pos = 0;

        let mut bit = 0;
        while bit < 8 {
            // check if the current integer is composed by 1 or 2 bytes
            if control_byte & (1 << bit) == 0 {
                mask[mask_pos + 1] = 0x80;
                mask[mask_pos] = input_pos as u8;
                mask_pos += 2;
                input_pos += 1;
            } else {
                mask[mask_pos] = input_pos as u8;
                mask[mask_pos + 1] = 1 + input_pos as u8;
                mask_pos += 2;
                input_pos += 2;
                length += 1;
            }
            
            bit += 1;
        }

        masks[control_byte as usize] = (mask, length);
        
        if control_byte == 255 {
            break;
        }
        control_byte += 1;
    }
    
    masks
}

#[inline]
pub fn vbe_decode_simd(input: *const u8, control_word: u8, output: *mut [u16; 8]) -> usize {
    let (ref mask, encoded_len) = MASKS[control_word as usize];
    unsafe {
        let mask = _mm_loadu_si128(mask.as_ptr().cast());
        let input = _mm_loadu_si128(input.cast());
        let answer = _mm_shuffle_epi8(input, mask);

        _mm_storeu_si128(output.cast(), answer);
    }

    encoded_len
}

pub fn vbe_encode(n: u16, data: &mut Vec<u8>, cbits: &mut BitVector) {
    if n < 256 {
        data.push(n as u8);
        cbits.push(false);
    }
    else {
        data.push(n as u8);
        data.push((n >> 8) as u8);
        cbits.push(true);
    }
}

