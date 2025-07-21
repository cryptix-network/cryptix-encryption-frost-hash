use std::time::Instant;

// === Configuration ===
const NUM_ROUNDS: usize = 24;
const ROUND_CONSTANTS: [u64; NUM_ROUNDS] = [
    0x243F6A8885A308D3, 0x13198A2E03707344, 0xA4093822299F31D0, 0x082EFA98EC4E6C89,
    0x452821E638D01377, 0xBE5466CF34E90C6C, 0xC0AC29B7C97C50DD, 0x3F84D5B5B5470917,
    0x9216D5D98979FB1B, 0xD1310BA698DFB5AC, 0x2FFD72DBD01ADFB7, 0xB8E1AFED6A267E96,
    0x9B05688C2B3E6C1F, 0x1F83D9ABFB41BD6B, 0x5BE0CD19137E2179, 0xCBBB9D5DC1059ED8,
    0x629A292A367CD507, 0x9159015A3070DD17, 0x152FECD8F70E5939, 0x67332667FFC00B31,
    0x8EB44A8768581511, 0xDB0C2E0D64F98FA7, 0x47B5481DBEFA4FA4, 0x0FC19DC68B8CD5B5,
];

// === Dynamic S-Box Generation Using Input Bytes ===
fn generate_sbox(seed: u64, input_bytes: &[u8]) -> [u8; 256] {
    let mut sbox = [0u8; 256];
    for i in 0..256 {
        sbox[i] = i as u8;
    }
    // Seed mix
    let mut mixed_seed = seed;
    if input_bytes.len() > 7 {
        mixed_seed ^= (input_bytes[2] as u64) << 16;
        mixed_seed ^= (input_bytes[5] as u64) << 8;
        mixed_seed ^= input_bytes[7] as u64; 
    } else {
        for (i, &b) in input_bytes.iter().enumerate() {
            mixed_seed ^= (b as u64) << (8 * (i % 8));
        }
    }

    // Shuffle 1
    let mut state = mixed_seed;
    for i in (1..256).rev() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let j = (state % (i as u64 + 1)) as usize;
        sbox.swap(i, j);
    }

    // Shuffle 2
    for i in (1..256).rev() {
        state = state.wrapping_mul(1442695040888963407).wrapping_add(1);
        let j = (state % (i as u64 + 1)) as usize;
        sbox.swap(i, j);
    }

    sbox
}


// Byte substitution with dynamic S-Box
fn byte_substitute(x: u8, sbox: &[u8; 256]) -> u8 {
    sbox[x as usize]
}

fn apply_byte_substitution(x: u64, sbox: &[u8; 256]) -> u64 {
    let mut result = 0u64;
    for i in 0..8 {
        let byte = ((x >> (i * 8)) & 0xFF) as u8;
        let substituted = byte_substitute(byte, sbox);
        result |= (substituted as u64) << (i * 8);
    }
    result
}

// Improved bit mixing with an additional nonlinear round and input bytes
fn bit_mix(mut x: u64, rc: u64, input_bytes: &[u8]) -> u64 {
    x = x.wrapping_add(rc);
    x ^= x.rotate_left(7);
    x = x.wrapping_add(x.rotate_left(17));
    x ^= x.wrapping_mul(0x94D049BB133111EB);
    x = x.wrapping_sub(x.rotate_right(19));
    x ^= rc.rotate_left(11);
    x ^= (x.wrapping_mul(0xA24BAED4963EE407)).rotate_left(23);

    if input_bytes.len() > 6 {
        let b3 = input_bytes[3] as u64;
        let b6 = input_bytes[6] as u64;
        x ^= (b3.wrapping_mul(0xDEADBEEFDEADBEEF) ^ b6.wrapping_mul(0xBADF00D1BADF00D1)).rotate_left(13);
    }
    x
}

// One round permutation, with stronger interaction between state words and input bytes
fn permute_round(state: &mut [u64; 8], round: usize, sbox: &[u8; 256], input_bytes: &[u8]) {
    let rc = ROUND_CONSTANTS[round % ROUND_CONSTANTS.len()];
    for i in 0..8 {
        state[i] = bit_mix(state[i], rc, input_bytes);
    }
    for i in 0..8 {
        state[i] = apply_byte_substitution(state[i], sbox);
    }
    for i in 0..8 {
        let left = state[(i + 7) % 8];
        let right = state[(i + 1) % 8];
        let center = state[(i + 4) % 8];
        let extra = state[(i + 3) % 8]; 
        state[i] ^= left.rotate_left((round as u32 + i as u32 * 2) % 64);
        state[i] ^= right.rotate_right((round as u32 + i as u32 * 3) % 64);
        state[i] ^= center.rotate_left(11);
        state[i] ^= extra.wrapping_mul(0x9E3779B97F4A7C15).rotate_left((round as u32 + i as u32 * 5) % 64);
    }
}

// Complete permutation with dynamic S-Box (from initial state seed + input bytes)
fn permute(state: &mut [u64; 8], input_bytes: &[u8]) {
    let seed = state.iter().fold(0u64, |acc, &v| acc ^ v);
    let sbox = generate_sbox(seed, input_bytes);

    for round in 0..NUM_ROUNDS {
        permute_round(state, round, &sbox, input_bytes);
    }
}

// Padding
fn pad_block(data: &[u8]) -> Vec<u8> {
    let mut padded = data.to_vec();
    padded.push(0x80);
    while padded.len() % 8 != 0 {
        padded.push(0x00);
    }
    padded
}

// Hash function
fn hash(inputs: &[&[u8]]) -> [u64; 8] {
    let mut state = [0u64; 8];
    for &block in inputs {
        let padded = pad_block(block);
        for (i, chunk) in padded.chunks(8).enumerate() {
            let mut val = 0u64;
            for (j, &b) in chunk.iter().enumerate() {
                val |= (b as u64) << (j * 8);
            }
            state[i % 8] ^= val;
        }
        permute(&mut state, &padded);
    }
    state
}

// Helper function to print hashes
fn print_hash(label: &str, input: &[u8], hashval: &[u64; 8]) {
    print!("{} ({} bytes): ", label, input.len());
    for &v in hashval {
        print!("{:016x}", v);
    }
    println!();
}

// === main ===
fn main() {
    
    let test_inputs = vec![
        b"".as_ref(),
        b"short".as_ref(),
        b"some medium length data".as_ref(),
        b"this is a longer input data to test the hash function with multiple rounds".as_ref(),
        &[0u8; 1000][..],
    ];


    println!("=== Cryptix Frost Hash ===");

    println!("=== Serial Hashes ===");
    for (i, input) in test_inputs.iter().enumerate() {
        let hashval = hash(&[*input]);
        print_hash(&format!("Input {}", i), input, &hashval);
    }

    let bigdata = vec![0x55u8; 10_000_000];
    let start = Instant::now();
    let _ = hash(&[&bigdata]);
    println!("Hash time for 10MB: {:.3?}", start.elapsed());

    // Tests
    avalanche_test();
    collision_test();
    speed_test();
    determinism_test();
    differential_test();
}


// === Tests ===

// Avalanche test function
fn avalanche_test() {
    let input = b"hello world";
    let mut modified = input.clone().to_vec();
    modified[0] ^= 0x01;

    let h1 = hash(&[input]);
    let h2 = hash(&[&modified]);

    println!("--- Avalanche Test ---");
    print_hash("Original", input, &h1);
    print_hash("Modified", &modified, &h2);

    let mut diff_bits = 0;
    for i in 0..8 {
        diff_bits += (h1[i] ^ h2[i]).count_ones();
    }
    println!("Differing bits: {}", diff_bits);
}

// Collision test: scan many similar inputs for collisions
fn collision_test() {
    println!("--- Collision Test ---");
    let base = b"collision_test_base_string";
    let mut collisions = 0;
    let tries = 200;

    for i in 0..tries {
        let mut input = base.to_vec();
        input.push(i as u8);
        let h1 = hash(&[&input]);

        for j in (i + 1)..tries {
            let mut input2 = base.to_vec();
            input2.push(j as u8);
            let h2 = hash(&[&input2]);

            if h1 == h2 {
                println!("Collision found between inputs {} and {}", i, j);
                collisions += 1;
            }
        }
    }

    println!("Total collisions in {} tries: {}", tries, collisions);
}

// Speed test for various input sizes
fn speed_test() {
    println!("--- Speed Test ---");
    let sizes = [1, 16, 64, 256, 1024, 4096, 16_384, 65_536];
    for &size in &sizes {
        let data = vec![0xAAu8; size];
        let start = Instant::now();
        let _ = hash(&[&data]);
        let elapsed = start.elapsed();
        println!("Input size: {:6} bytes, Time: {:?}", size, elapsed);
    }
}

// Determinism test: same input -> same output
fn determinism_test() {
    println!("--- Determinism Test ---");
    let input = b"determinism_test_input_data";
    let h1 = hash(&[input]);
    let h2 = hash(&[input]);
    assert_eq!(h1, h2);
    println!("Determinism test passed!");
}

// Differential Analysis: Evaluates Avalanche Effect Across Input Variations
fn differential_test() {
    let base = b"diff_test_input_data_for_hash";
    let mut total_diff = 0u32;
    let mut pairs = 0u32;

    for i in 0..base.len() {
        for b in 0u8..=255 {
            let input1 = base.to_vec();
            let mut input2 = base.to_vec();
            input2[i] = b;

            let h1 = hash(&[&input1]);
            let h2 = hash(&[&input2]);

            let mut diff_bits = 0;
            for idx in 0..8 {
                diff_bits += (h1[idx] ^ h2[idx]).count_ones();
            }
            total_diff += diff_bits;
            pairs += 1;
        }
    }

    println!("Differential test average differing bits: {}", total_diff / pairs);
}



