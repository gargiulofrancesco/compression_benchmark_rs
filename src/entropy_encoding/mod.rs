use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

use crate::bit_vector::BitVector;

/// Calculates the Shannon entropy of a collection of elements of a generic type.
///
/// This function computes the Shannon entropy of a collection of elements, which measures the uncertainty or 
/// randomness in the distribution of the elements. The Shannon entropy is defined as the sum of the products 
/// between each element's probability and the negative logarithm (base 2) of that probability.
/// 
/// # Arguments
///
/// - `collection`: A collection of elements. The collection can be any type that implements
///   `IntoIterator`, where the items are hashable and equatable (i.e., they implement `Eq` and `Hash`).
///
/// # Returns
///
/// The entropy of the collection as a `f64` value. If the collection is empty, the entropy is `0.0`.
///
/// # Examples
///
/// ```
/// use std::collections::HashSet;
/// use pef::entropy_encoding::*;
///
/// let data = vec![1, 2, 2, 3, 3, 3];
/// let entropy = shannon_entropy(data.iter());
/// println!("Entropy: {}", entropy);
/// ```
pub fn shannon_entropy<T, I>(collection: I) -> f64 
where
    T: Eq + Hash,
    I: Iterator<Item = T>
{
    let mut frequency_map = HashMap::new();
    let mut total_elements = 0;

    for item in collection {
        *frequency_map.entry(item).or_insert(0) += 1;
        total_elements += 1;
    }

    let total_elements = total_elements as f64;

    let entropy = frequency_map.values()
        .map(|&count|{
            let probability = count as f64 / total_elements;
            -probability * probability.log2()
        })
        .sum();

    entropy
}

/// Calculates the k-th order empirical entropy of a collection of elements of a generic type.
///
/// This function computes the high-order empirical entropy of a collection of elements, which measures
/// the uncertainty or randomness in the collection when considering subsequences of length `k`. Higher 
/// order entropies take into account the dependency between elements in the collection, thus providing 
/// a more refined measure of entropy compared to the 0-th order entropy (Shannon entropy).
///
/// # Arguments
///
/// - `collection`: A collection of elements. The collection can be any type that implements
///   `IntoIterator`, where the items are hashable, equatable (i.e., they implement `Eq` and `Hash`), 
///   and copyable (i.e., they implement `Copy`).
/// - `k`: The length of the subsequences to consider. This determines the "order" of the entropy.
///
/// # Returns
///
/// The k-th order entropy of the collection as a `f64` value. If the collection is shorter than `k` elements
/// the entropy is `0.0`.
///
/// # Examples
///
/// ```
/// use std::collections::HashSet;
/// use pef::entropy_encoding::*;
///
/// let data = vec![1, 2, 2, 3, 3, 3];
/// let entropy = kth_order_empirical_entropy(data.iter(), 2);
/// println!("2nd Order Entropy: {}", entropy);
/// ```
pub fn kth_order_empirical_entropy<T, I>(collection: I, k: usize) -> f64 
where
    T: Eq + Hash + Copy,
    I: Iterator<Item = T>
{
    if k == 0 {
        return shannon_entropy(collection);
    }

    let mut iter = collection;
    let mut window= VecDeque::new();
    let mut map: HashMap<Vec<T>, Vec<T>> = HashMap::new();
    let mut collection_length = k;

    // Populate the initial window
    for _ in 0..k {
        if let Some(item) = iter.next() {
            window.push_back(item);
        } else {
            return 0.0;
        }
    }

    // Process the rest of the collection
    for item in iter {
        let key = window.iter().copied().collect::<Vec<T>>();
        if map.contains_key(&key) {
            map.get_mut(&key).unwrap().push(item);
        } else {
            map.insert(key, vec![item]);
        }

        window.pop_front();
        window.push_back(item);
        collection_length += 1;
    }

    let sum_entropy: f64 = map.values()
        .map(|sequence| {
            shannon_entropy(sequence.iter()) * sequence.len() as f64
        })
        .sum();

    sum_entropy / collection_length as f64
}

/// Encodes a positive integer `n` using Elias gamma encoding and appends the result to a `BitVector`.
///
/// Elias gamma encoding is a universal code encoding positive integers. It represents an integer `n` 
/// by concatenating two parts: a unary prefix (representing the length of the binary representation of `n`),
/// followed by the binary representation of `n`.
/// 
/// # Arguments
///
/// * `n` - The positive integer to be encoded.
/// * `bitvector` - A mutable reference to a `BitVector` where the encoded bits will be appended.
///
/// # Panics
///
/// Panics if `n` is not greater than 0.
///
/// # Examples
///
/// ```
/// use pef::bitvector::*;
/// use pef::entropy_encoding::*;
///
/// let mut bitvector = BitVector::new();
/// elias_gamma_encode(10, &mut bitvector);
/// ```
pub fn elias_gamma_encode(n: u64, bitvector: &mut BitVector) {
    assert!(n > 0, "Elias gamma encoding only supports positive integers");

    let leading_zeroes = n.leading_zeros() as usize;
    let length = 64 - leading_zeroes;
    let unary_prefix_zeroes = length - 1;

    // Append the unary prefix
    if unary_prefix_zeroes > 0 {
        bitvector.append_bits(0, unary_prefix_zeroes);
    }

    // Append the binary representation of `n`
    let reversed_n = n.reverse_bits() >> leading_zeroes;
    bitvector.append_bits(reversed_n, length);
}

/// Decodes an Elias gamma encoded integer from a BitVector starting at a specified index.
///
/// This function decodes an Elias gamma encoded integer from the given BitVector starting at the specified index. 
/// The decoding process involves finding the unary part, which represents the number of leading zeroes,
/// and then reading the subsequent binary part to extract the encoded integer.
///
/// # Arguments
///
/// * `index` - The starting index in the BitVector from which to begin decoding.
/// * `bitvector` - A reference to a BitVector containing the encoded data.
///
/// # Returns
///
//// Returns `Some((decoded_value, code_length))` if decoding is successful, where `decoded_value` is the decoded integer 
/// and `code_length` is the total length of the Elias gamma encoded integer in bits.
/// Returns `None` if the index exceeds the length of the BitVector or if decoding fails.
///
/// # Examples
///
/// ```
/// use pef::bitvector::*;
/// use pef::entropy_encoding::*;
///
/// let mut bitvector: BitVector = BitVector::new();
/// let n = 42;
/// elias_gamma_encode(n, &mut bitvector);
/// assert_eq!(elias_gamma_decode(0, &bitvector), Some((n, 11)));
/// ```
pub fn elias_gamma_decode(index: usize, bitvector: &BitVector) -> Option<(u64, usize)> {
    let mut index = index;
    let mut n_zeroes = 0;
    while index < bitvector.len() && !bitvector.get(index).unwrap() {
        n_zeroes += 1;
        index += 1;
    }
    let length = n_zeroes + 1;

    let reversed_n = bitvector.get_bits(index, length)?;
    let n = reversed_n.reverse_bits() >> (64 - length);

    Some((n, n_zeroes + length))
}

/// Encodes a positive integer `n` using Elias delta encoding and appends the result to a `BitVector`.
///
/// Elias delta encoding is a universal code encoding positive integers. It represents an integer `n` 
/// by concatenating two parts: its length encoded in Elias gamma encoding, followed by its binary 
/// representation without the leading '1'
///
/// # Arguments
///
/// * `n` - The positive integer to be encoded.
/// * `bitvector` - A mutable reference to a `BitVector` where the encoded bits will be appended.
///
/// # Panics
///
/// Panics if `n` is not greater than 0.
///
/// # Examples
///
/// ```
/// use pef::bitvector::*;
/// use pef::entropy_encoding::*;
///
/// let mut bitvector = BitVector::new();
/// elias_delta_encode(10, &mut bitvector);
/// ```
pub fn elias_delta_encode(n: u64, bitvector: &mut BitVector) {
    assert!(n > 0, "Elias delta encoding only supports positive integers");

    let leading_zeroes = n.leading_zeros() as usize;
    let length = 64 - leading_zeroes;

    // Use Elias gamma encoding on the length of the binary representation
    elias_gamma_encode(length as u64, bitvector);

    // Append the binary representation without the leading '1'
    let reversed_n = n.reverse_bits() >> leading_zeroes;
    bitvector.append_bits(reversed_n >> 1, length - 1);
}

/// Decodes an Elias delta encoded integer from a BitVector starting at a specified index.
///
/// This function decodes an Elias delta encoded integer from the given BitVector starting at the specified index.
/// The decoding process involves extracting the integer's length, which is encoded using Elias gamma encoding,
/// and then reading the subsequent bits to obtain the decoded integer.
///
/// # Arguments
///
/// * `index` - The starting index in the BitVector from which to begin decoding.
/// * `bitvector` - A reference to a BitVector containing the encoded data.
///
/// # Returns
///
//// Returns `Some((decoded_value, code_length))` if decoding is successful, where `decoded_value` is the decoded integer 
/// and `code_length` is the total length of the Elias delta encoded integer in bits.
/// Returns `None` if the index exceeds the length of the BitVector or if decoding fails.
///
/// # Examples
///
/// ```
/// use pef::bitvector::*;
/// use pef::entropy_encoding::*;
///
/// let mut bitvector: BitVector = BitVector::new();
/// let n = 13;
/// elias_delta_encode(n, &mut bitvector);
/// assert_eq!(elias_delta_decode(0, &bitvector), Some((n, 8)));
/// ```
pub fn elias_delta_decode(index: usize, bitvector: &BitVector) -> Option<(u64, usize)> {
    let mut index = index;
    let (n_length, gamma_code_length) = elias_gamma_decode(index, bitvector)?;
    let n_length = n_length as usize;
    index += gamma_code_length; 

    let reversed_n = (bitvector.get_bits(index, n_length - 1)? << 1) + 1;
    let n = reversed_n.reverse_bits() >> (64 - n_length);

    Some((n, gamma_code_length + n_length - 1))
}

/// Encodes a positive integer `n` using Fibonacci encoding and appends the result to a `BitVector`.
///
/// Fibonacci encoding represents an integer `n` using a combination of Fibonacci numbers.
/// The encoding starts with the largest Fibonacci number less than or equal to `n` and
/// proceeds downwards, appending `1` for each Fibonacci number used in the representation
/// of `n`, and `0` otherwise.
///
/// # Arguments
///
/// * `n` - The positive integer to be encoded.
/// * `bitvector` - A mutable reference to a `BitVector` where the encoded bits will be appended.
///
/// # Panics
///
/// Panics if `n` is not greater than 0.
///
/// # Examples
///
/// ```
/// use pef::bitvector::*;
/// use pef::entropy_encoding::*;
///
/// let mut bitvector = BitVector::new();
/// fibonacci_encode(13, &mut bitvector);
/// ```
pub fn fibonacci_encode(n: u64, bitvector: &mut BitVector) {
    assert!(n > 0, "Fibonacci encoding only supports positive integers");

    let mut fib: Vec<u128> = vec![0, 1, 1];
    let mut n = n as u128;
    
    // Generating Fibonacci sequence until the smallest Fibonacci number >= n
    let mut i = 2;
    while fib[i] < n {
        let next_fib = fib[i] + fib[i - 1];
        fib.push(next_fib);
        i += 1;
    }

    let mut code: u128 = 0;
    while n > 0 {
        if n >= fib[i] {
            code |= 1 << (i - 2);
            n -= fib[i];
        }
        i -= 1;
    }

    let low_bits = code as u64;
    let high_bits = (code >> 64) as u64;
    bitvector.append_bits(low_bits, 64 - low_bits.leading_zeros() as usize);
    bitvector.append_bits(high_bits, 64 - high_bits.leading_zeros() as usize);
    bitvector.push(true);
}

/// Decodes a Fibonacci encoded integer from a BitVector starting at a specified index.
///
/// This function decodes a Fibonacci encoded integer from the given BitVector starting at the specified index.
/// The decoding process involves iteratively reading bits from the BitVector, reconstructing the original integer
/// using the Fibonacci sequence representation.
///
/// # Arguments
///
/// * `index` - The starting index in the BitVector from which to begin decoding.
/// * `bitvector` - A reference to a BitVector containing the encoded data.
///
/// # Returns
///
/// Returns `Some((decoded_value, code_length))` if decoding is successful, where `decoded_value` is the decoded integer
/// and `code_length` is the total length of the Fibonacci encoded integer in bits.
/// Returns `None` if the index exceeds the length of the BitVector or if decoding fails.
///
/// # Examples
///
/// ```
/// use pef::bitvector::*;
/// use pef::entropy_encoding::*;
///
/// let mut bitvector = BitVector::new();
/// let n = 13;
/// fibonacci_encode(n, &mut bitvector);
/// assert_eq!(fibonacci_decode(0, &bitvector), Some((n, 7)));
/// ```
pub fn fibonacci_decode(index: usize, bitvector: &BitVector) -> Option<(u64, usize)> {
    let mut curr_index = index;

    let mut last_bit = false;
    let mut next_bit = false;
    let mut fib = vec![0, 1, 1];
    let mut n = 0;
    
    let mut i = 2;
    while !(last_bit && next_bit) {
        last_bit = next_bit;
        next_bit = bitvector.get(curr_index)?;

        if next_bit && !last_bit {
            n += fib[i];
        }

        fib.push(fib[i] + fib[i-1]);
        curr_index += 1;
        i += 1;
    }

    Some((n, curr_index - index))
}

/// Encodes a positive integer `n` using Variable Byte encoding and appends the result to a `BitVector`.
///
/// Variable Byte encoding is a compression technique used to encode positive integers. It represents an integer `n` 
/// by dividing it into a sequence of bytes, where each byte represents 7 bits of the integer. The most significant bit
/// of each byte is set to `0` except for the last byte, which has its most significant bit set to `1`. This allows for
/// efficient storage of small integers while still supporting larger integers.
/// 
/// # Arguments
///
/// * `n` - The positive integer to be encoded.
/// * `bitvector` - A mutable reference to a `BitVector` where the encoded bits will be appended.
///
/// # Examples
///
/// ```
/// use pef::bitvector::*;
/// use pef::entropy_encoding::*;
///
/// let mut bitvector = BitVector::new();
/// variable_byte_encode(42, &mut bitvector);
/// ```
pub fn variable_byte_encode(n: u64, bitvector: &mut BitVector) {
    let mut bytes: Vec<u8> = Vec::new();
    let mut num = n;

    while num >= 128 {
        bytes.push((num & 0x7F) as u8);
        num >>= 7;
    }
    bytes.push((num | 0x80) as u8);

    for byte in bytes {
        bitvector.append_bits(byte as u64, 8);
    }
}

/// Decodes a Variable Byte encoded integer from a BitVector starting at a specified index.
///
/// This function decodes a Variable Byte encoded integer from the given BitVector starting at the specified index.
/// The decoding process involves reading bytes from the BitVector and reconstructing the original integer by combining
/// the 7 least significant bits of each byte. The most significant bit of each byte is used as a delimiter to determine
/// the end of the encoded integer.
///
/// # Arguments
///
/// * `index` - The starting index in the BitVector from which to begin decoding.
/// * `bitvector` - A reference to a BitVector containing the encoded data.
///
/// # Returns
///
/// Returns `Some((decoded_value, code_length))` if decoding is successful, where `decoded_value` is the decoded integer
/// and `code_length` is the total length of the Variable Byte encoded integer in bits.
/// Returns `None` if the index exceeds the length of the BitVector or if decoding fails.
///
/// # Examples
///
/// ```
/// use pef::bitvector::*;
/// use pef::entropy_encoding::*;
///
/// let mut bitvector: BitVector = BitVector::new();
/// let n = 42;
/// variable_byte_encode(n, &mut bitvector);
/// assert_eq!(variable_byte_decode(0, &bitvector), Some((n, 8)));
/// ```
pub fn variable_byte_decode(index: usize, bitvector: &BitVector) -> Option<(u64, usize)> {
    let mut curr_index = index;
    let mut n: u64 = 0;
    let mut shift: u64 = 0;
    let mut byte;
    
    loop {
        byte = bitvector.get_bits(curr_index, 8)?;
        
        n |= (byte & 0x7F) << shift;
        curr_index += 8;
        shift += 7;
        
        if byte > 127 {
            break;
        }
    }
    
    Some((n, curr_index - index))
}
