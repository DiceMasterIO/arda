//! Deterministic integer hashing: every random choice in the crate is a pure
//! function of the world seed and global coordinates (goal 42, §8
//! determinism), so block generation order can never change the output.
//! The functions live in `arda-wfc`, shared with the town WFC (logic/09
//! §hash); this module re-exports them unchanged.

pub use arda_wfc::hash::{hash2, hash3, mix, signed, unit, Stream};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_stays_in_range_and_is_spread() {
        let mut lo = 0;
        for i in 0..10_000 {
            let u = unit(hash2(1, 2, i, -i));
            assert!((0.0..1.0).contains(&u));
            if u < 0.5 {
                lo += 1;
            }
        }
        assert!((4_700..5_300).contains(&lo), "{lo}");
    }

    #[test]
    fn hash_depends_on_every_input() {
        let base = hash3(1, 2, 3, 4, 5);
        assert_ne!(base, hash3(0, 2, 3, 4, 5));
        assert_ne!(base, hash3(1, 0, 3, 4, 5));
        assert_ne!(base, hash3(1, 2, 0, 4, 5));
        assert_ne!(base, hash3(1, 2, 3, 0, 5));
        assert_ne!(base, hash3(1, 2, 3, 4, 0));
        assert_ne!(hash2(1, 2, 3, 4), hash2(1, 2, 4, 3));
    }
}
