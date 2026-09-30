//! The stable hash and subseed functions (`logic/09` §hash, I21).
//!
//! Two families, both frozen by golden tests; changing any output is a
//! format change for every crate that depends on it.
//!
//! # Integer hash: [`hash`]
//!
//! For rules two crates or two blocks must agree on at high volume (edge
//! offsets, scatter tiles, noise lattices, variant picks). It is the
//! SplitMix64-finaliser chain of feat/tactical-terrain, generalised to any
//! number of arguments:
//!
//! ```text
//! h0     = mix(seed ^ mix(tag))
//! h(i+1) = mix(h(i) ^ rotl(arg_i as u64, ROT[i mod 8]))
//! ROT    = [0, 29, 43, 11, 53, 17, 37, 5]
//! ```
//!
//! With two arguments it equals that branch's `hash2`, with three its
//! `hash3`, so its outputs carry over unchanged when it merges. `tag` is a
//! fixed per-rule constant; [`tag`] derives one from a rule name at compile
//! time (64-bit FNV-1a of the UTF-8 bytes).
//!
//! # Byte hash: [`digest`] and [`subseed`]
//!
//! For keyed random streams that need more than 64 bits of state.
//! [`digest`] is BLAKE3 over the plain concatenation of its parts; framing is
//! the caller's job (fixed-width integers, at most one variable-length part,
//! placed last). This is exactly the scheme of `arda-npc`'s streams
//! (`logic/13` §npc-seed). [`subseed`] derives a 64-bit seed for a named
//! sub-generator:
//!
//! ```text
//! subseed(seed, domain, args) = u64_le(first 8 bytes of
//!     BLAKE3-derive-key("arda-ids subseed v1",
//!         le64(seed) || le64(len(domain)) || domain || le64(arg_0) || …))
//! ```

/// Rotation applied to argument `i` of [`hash`], indexed `i mod 8`.
const ROT: [u32; 8] = [0, 29, 43, 11, 53, 17, 37, 5];

/// BLAKE3 key-derivation context of [`subseed`]. Never change it.
const SUBSEED_CONTEXT: &str = "arda-ids subseed v1";

/// The SplitMix64 finaliser.
#[must_use]
pub const fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The canonical integer hash of a seed, a rule tag and signed arguments.
#[must_use]
pub const fn hash(seed: u64, tag: u64, args: &[i64]) -> u64 {
    let mut h = mix(seed ^ mix(tag));
    let mut i = 0;
    while i < args.len() {
        h = mix(h ^ args[i].cast_unsigned().rotate_left(ROT[i % ROT.len()]));
        i += 1;
    }
    h
}

/// A rule tag from its name: 64-bit FNV-1a of the UTF-8 bytes.
#[must_use]
pub const fn tag(name: &str) -> u64 {
    let bytes = name.as_bytes();
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    let mut i = 0;
    while i < bytes.len() {
        h ^= bytes[i] as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
        i += 1;
    }
    h
}

/// BLAKE3 of the concatenation of `parts`.
#[must_use]
pub fn digest(parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    for part in parts {
        hasher.update(part);
    }
    *hasher.finalize().as_bytes()
}

/// [`digest`] as four little-endian 64-bit words, for seeding a PRNG state.
#[must_use]
pub fn digest_words(parts: &[&[u8]]) -> [u64; 4] {
    words(&digest(parts))
}

/// A 64-bit seed for the sub-generator `domain` of `seed`, keyed by `args`.
#[must_use]
pub fn subseed(seed: u64, domain: &str, args: &[u64]) -> u64 {
    let mut hasher = blake3::Hasher::new_derive_key(SUBSEED_CONTEXT);
    hasher.update(&seed.to_le_bytes());
    hasher.update(&(domain.len() as u64).to_le_bytes());
    hasher.update(domain.as_bytes());
    for arg in args {
        hasher.update(&arg.to_le_bytes());
    }
    words(hasher.finalize().as_bytes())[0]
}

fn words(bytes: &[u8; 32]) -> [u64; 4] {
    let (chunks, _) = bytes.as_chunks::<8>();
    let mut out = [0u64; 4];
    for (word, chunk) in out.iter_mut().zip(chunks) {
        *word = u64::from_le_bytes(*chunk);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// feat/tactical-terrain's `hash2`/`hash3`, copied verbatim.
    fn refine_hash2(seed: u64, tag: u64, x: i64, y: i64) -> u64 {
        let a = mix(seed ^ mix(tag));
        let b = mix(a ^ x.cast_unsigned());
        mix(b ^ y.cast_unsigned().rotate_left(29))
    }

    fn refine_hash3(seed: u64, tag: u64, x: i64, y: i64, z: i64) -> u64 {
        mix(refine_hash2(seed, tag, x, y) ^ z.cast_unsigned().rotate_left(43))
    }

    #[test]
    fn matches_the_tactical_terrain_hash() {
        for (x, y, z) in [(0, 0, 0), (-5, 17, 3), (i64::MIN, i64::MAX, -1)] {
            assert_eq!(hash(42, 7, &[x, y]), refine_hash2(42, 7, x, y));
            assert_eq!(hash(42, 7, &[x, y, z]), refine_hash3(42, 7, x, y, z));
        }
    }

    #[test]
    fn outputs_are_frozen() {
        assert_eq!(mix(0), GOLDEN_MIX0);
        assert_eq!(hash(42, tag("edge.river"), &[3, -4]), GOLDEN_HASH);
        assert_eq!(tag(""), 0xCBF2_9CE4_8422_2325);
        assert_eq!(tag("a"), 0xAF63_DC4C_8601_EC8C);
        assert_eq!(subseed(42, "npc", &[7, 9]), GOLDEN_SUBSEED);
        assert_eq!(digest_words(&[b"arda", b"-ids"])[0], GOLDEN_DIGEST0);
        assert_eq!(digest(&[b"arda", b"-ids"]), digest(&[b"arda-ids"]));
    }

    #[test]
    fn inputs_separate_streams() {
        assert_ne!(hash(1, 2, &[3, 4]), hash(1, 2, &[4, 3]));
        assert_ne!(hash(1, 2, &[3]), hash(1, 3, &[3]));
        assert_ne!(subseed(1, "a", &[]), subseed(1, "b", &[]));
        assert_ne!(subseed(1, "ab", &[]), subseed(1, "a", &[u64::from(b'b')]));
        assert_ne!(subseed(1, "a", &[0]), subseed(2, "a", &[0]));
    }

    const GOLDEN_MIX0: u64 = 0xE220_A839_7B1D_CDAF;
    const GOLDEN_HASH: u64 = 0x1C7C_1503_463A_2656;
    const GOLDEN_SUBSEED: u64 = 0x1A48_D9DC_3581_6BF9;
    const GOLDEN_DIGEST0: u64 = 0x2F21_9D21_2A46_A6E4;
}
