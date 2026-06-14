//! Small, stable hashing helpers shared across the workspace.

/// FNV-1a (64-bit). A fast, stable, non-cryptographic hash — used for
/// deterministic golden-trace digests and RNG-free "pick one" rotations. Stable
/// across builds, so values hashed here can be compared/asserted in tests.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}
