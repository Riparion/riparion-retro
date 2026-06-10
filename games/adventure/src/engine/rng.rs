//! A bit-for-bit reimplementation of CPython's `random.Random`.
//!
//! Colossal Cave's behaviour — when dwarves appear, where they wander, the
//! pirate's whims, the random pit deaths, the "% of the time" travel branches —
//! is driven entirely by the random number generator. The canonical reference
//! port we follow (`brandon-rhodes/python-adventure`) uses Python's Mersenne
//! Twister, and ships seeded walkthroughs as golden-master tests. To replay
//! those walkthroughs *exactly* (our strongest fidelity guarantee) the engine's
//! RNG must reproduce Python's `random()` and `choice()` to the bit.
//!
//! So this module is MT19937 plus CPython's `genrand_res53` (`random()`),
//! integer-seeding (`init_by_array`), and `_randbelow`/`getrandbits`
//! (`choice`). Fresh games take a seed from [`retro_kit::rng::random_seed`]
//! (browser entropy on wasm); the walkthrough tests pass the same small integer
//! seeds python-adventure does. It is fully serializable so saves resume with
//! an identical stream.

use serde::{Deserialize, Serialize};

const N: usize = 624;
const M: usize = 397;
const MATRIX_A: u32 = 0x9908_b0df;
const UPPER_MASK: u32 = 0x8000_0000;
const LOWER_MASK: u32 = 0x7fff_ffff;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PyRandom {
    /// The 624-word state vector. Stored as a `Vec` because serde derives only
    /// cover arrays up to length 32.
    mt: Vec<u32>,
    mti: usize,
    /// Test-only log of the operation sequence, used to diff RNG consumption
    /// against the reference implementation.
    #[cfg(test)]
    #[serde(skip)]
    pub ops: Vec<String>,
}

impl PyRandom {
    /// Seed exactly as `random.Random(seed)` does for a non-negative integer:
    /// CPython's `init_by_array` over the seed's little-endian 32-bit digits.
    pub fn from_seed(seed: u64) -> Self {
        let mut key: Vec<u32> = Vec::new();
        let mut s = seed;
        // The little-endian 32-bit digits of the seed; at least one word.
        loop {
            key.push((s & 0xffff_ffff) as u32);
            s >>= 32;
            if s == 0 {
                break;
            }
        }
        let mut r = PyRandom {
            mt: vec![0u32; N],
            mti: N + 1,
            #[cfg(test)]
            ops: Vec::new(),
        };
        r.init_by_array(&key);
        r
    }

    fn init_genrand(&mut self, s: u32) {
        self.mt[0] = s;
        for i in 1..N {
            let prev = self.mt[i - 1];
            self.mt[i] = (1_812_433_253u32
                .wrapping_mul(prev ^ (prev >> 30)))
            .wrapping_add(i as u32);
        }
        self.mti = N;
    }

    fn init_by_array(&mut self, key: &[u32]) {
        self.init_genrand(19_650_218);
        let mut i = 1usize;
        let mut j = 0usize;
        let mut k = if N > key.len() { N } else { key.len() };
        while k > 0 {
            let prev = self.mt[i - 1];
            self.mt[i] = (self.mt[i] ^ (1_664_525u32.wrapping_mul(prev ^ (prev >> 30))))
                .wrapping_add(key[j])
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= N {
                self.mt[0] = self.mt[N - 1];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
            k -= 1;
        }
        k = N - 1;
        while k > 0 {
            let prev = self.mt[i - 1];
            self.mt[i] = (self.mt[i] ^ (1_566_083_941u32.wrapping_mul(prev ^ (prev >> 30))))
                .wrapping_sub(i as u32);
            i += 1;
            if i >= N {
                self.mt[0] = self.mt[N - 1];
                i = 1;
            }
            k -= 1;
        }
        self.mt[0] = 0x8000_0000;
    }

    fn genrand_int32(&mut self) -> u32 {
        if self.mti >= N {
            for kk in 0..(N - M) {
                let y = (self.mt[kk] & UPPER_MASK) | (self.mt[kk + 1] & LOWER_MASK);
                self.mt[kk] = self.mt[kk + M] ^ (y >> 1) ^ if y & 1 != 0 { MATRIX_A } else { 0 };
            }
            for kk in (N - M)..(N - 1) {
                let y = (self.mt[kk] & UPPER_MASK) | (self.mt[kk + 1] & LOWER_MASK);
                self.mt[kk] =
                    self.mt[kk + M - N] ^ (y >> 1) ^ if y & 1 != 0 { MATRIX_A } else { 0 };
            }
            let y = (self.mt[N - 1] & UPPER_MASK) | (self.mt[0] & LOWER_MASK);
            self.mt[N - 1] = self.mt[M - 1] ^ (y >> 1) ^ if y & 1 != 0 { MATRIX_A } else { 0 };
            self.mti = 0;
        }
        let mut y = self.mt[self.mti];
        self.mti += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    /// CPython's `genrand_res53`: a 53-bit float in `[0.0, 1.0)`.
    pub fn random(&mut self) -> f64 {
        #[cfg(test)]
        self.ops.push("R".to_string());
        let a = (self.genrand_int32() >> 5) as f64; // 27 bits
        let b = (self.genrand_int32() >> 6) as f64; // 26 bits
        (a * 67_108_864.0 + b) * (1.0 / 9_007_199_254_740_992.0)
    }

    /// CPython's `getrandbits(k)` for `0 < k <= 32`.
    fn getrandbits(&mut self, k: u32) -> u32 {
        debug_assert!((1..=32).contains(&k));
        self.genrand_int32() >> (32 - k)
    }

    /// CPython 3's `_randbelow_with_getrandbits(n)` for `n >= 1`.
    pub fn randbelow(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        let k = 32 - (n as u32).leading_zeros(); // n.bit_length()
        let mut r = self.getrandbits(k) as usize;
        while r >= n {
            r = self.getrandbits(k) as usize;
        }
        r
    }

    /// `random.choice(seq)`: return the index `seq[randbelow(len)]`.
    pub fn choice_index(&mut self, len: usize) -> usize {
        #[cfg(test)]
        self.ops.push(format!("C{len}"));
        self.randbelow(len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reference values produced by CPython 3:
    //   >>> r = random.Random(1); [r.random() for _ in range(3)]
    #[test]
    fn random_matches_cpython_seed_1() {
        let mut r = PyRandom::from_seed(1);
        let got: Vec<f64> = (0..3).map(|_| r.random()).collect();
        let want = [
            0.13436424411240122,
            0.8474337369372327,
            0.763774618976614,
        ];
        for (g, w) in got.iter().zip(want.iter()) {
            assert!((g - w).abs() < 1e-15, "got {g}, want {w}");
        }
    }

    // >>> r = random.Random(42); [r.random() for _ in range(2)]
    #[test]
    fn random_matches_cpython_seed_42() {
        let mut r = PyRandom::from_seed(42);
        let want = [0.6394267984578837, 0.025010755222666936];
        for w in want {
            let g = r.random();
            assert!((g - w).abs() < 1e-15, "got {g}, want {w}");
        }
    }

    // >>> r = random.Random(1); [r._randbelow(5) for _ in range(5)]
    // (CPython: choice over a length-5 sequence.)
    #[test]
    fn randbelow_matches_cpython_seed_1() {
        let mut r = PyRandom::from_seed(1);
        let got: Vec<usize> = (0..5).map(|_| r.randbelow(5)).collect();
        assert_eq!(got, vec![1, 4, 0, 2, 0]);
    }
}
