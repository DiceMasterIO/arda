//! Legacy MT19937 and world-keyed phases, evaluated with one fixed-point cosine.

use super::{spectral_tables::COS_QUARTER_Q60, validated_shape, SpectralError};
use arda_core::{rng as world_rng, SeedKey, Stage, Tier};
use rand_core::RngCore;

struct Mt19937 {
    state: [u32; 624],
    index: usize,
}

impl Mt19937 {
    fn new(seed: u32) -> Self {
        let mut state = [0_u32; 624];
        state[0] = seed;
        #[allow(clippy::cast_possible_truncation)] // The index is at most 623.
        for i in 1..624 {
            state[i] = 1_812_433_253_u32
                .wrapping_mul(state[i - 1] ^ (state[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        Self { state, index: 624 }
    }

    fn next(&mut self) -> u32 {
        if self.index == 624 {
            for i in 0..624 {
                let x = (self.state[i] & 0x8000_0000) | (self.state[(i + 1) % 624] & 0x7fff_ffff);
                self.state[i] = self.state[(i + 397) % 624]
                    ^ (x >> 1)
                    ^ if x & 1 == 0 { 0 } else { 0x9908_b0df };
            }
            self.index = 0;
        }
        let mut value = self.state[self.index];
        self.index += 1;
        value ^= value >> 11;
        value ^= (value << 7) & 0x9d2c_5680;
        value ^= (value << 15) & 0xefc6_0000;
        value ^= value >> 18;
        value
    }

    fn phase53(&mut self) -> u64 {
        phase53(self.next(), self.next())
    }
}

fn phase53(first: u32, second: u32) -> u64 {
    (u64::from(first >> 5) << 26) | u64::from(second >> 6)
}

fn phase_cosine(index: usize) -> i64 {
    let offset = index & 1023;
    match (index >> 10) & 3 {
        0 => COS_QUARTER_Q60[offset],
        1 => -COS_QUARTER_Q60[1024 - offset],
        2 => -COS_QUARTER_Q60[offset],
        _ => COS_QUARTER_Q60[1024 - offset],
    }
}

fn cosine_q60(phase: u64) -> Result<i64, SpectralError> {
    let index = usize::try_from(phase >> 41).map_err(|_| SpectralError::ArithmeticOverflow)?;
    let remainder = i128::from(phase & ((1_u64 << 41) - 1));
    let denominator = 1_i128 << 41;
    let numerator = i128::from(phase_cosine(index)) * (denominator - remainder)
        + i128::from(phase_cosine(index + 1)) * remainder;
    let rounded = if numerator < 0 {
        -((-numerator + denominator / 2) / denominator)
    } else {
        (numerator + denominator / 2) / denominator
    };
    i64::try_from(rounded).map_err(|_| SpectralError::ArithmeticOverflow)
}

// Shared admitted whole-field allocation and fixed-point cosine mapping.
// The caller supplies the stream's already-paired 53-bit phase; this function
// does not alter legacy/native or world-keyed draw order.
fn generate_white(
    width: usize,
    height: usize,
    max_bytes: u64,
    mut next_phase: impl FnMut() -> u64,
) -> Result<Vec<i64>, SpectralError> {
    let count = validated_shape(width, height)?;
    let bytes = count
        .checked_mul(size_of::<i64>())
        .and_then(|n| u64::try_from(n).ok())
        .ok_or(SpectralError::ResourceLimit)?;
    if bytes > max_bytes {
        return Err(SpectralError::ResourceLimit);
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(count)
        .map_err(|_| SpectralError::ResourceLimit)?;
    for _ in 0..count {
        output.push(cosine_q60(next_phase())?);
    }
    Ok(output)
}

/// Generates the native-reference MT19937 white field from its 32-bit seed.
///
/// This historical stream remains byte-exact for calibration and prior controls.
/// The cosine uses a checked-in table with linear interpolation, not platform
/// math. `max_bytes` bounds the returned whole-domain vector, excluding the
/// later filter. Publication tiles never get independent phases.
pub fn white_noise(
    seed: u32,
    width: usize,
    height: usize,
    max_bytes: u64,
) -> Result<Vec<i64>, SpectralError> {
    let mut rng = Mt19937::new(seed);
    generate_white(width, height, max_bytes, || rng.phase53())
}

/// Generates one whole-field Q60 white source from Arda's full 64-bit world seed.
///
/// The stream is `rng(world_seed, SeedKey::new(Continent, Relief, 0, 0,
/// attempt))`. Consecutive core `u32` draws form each native-style 53-bit phase
/// (top 27 bits of the first, top 26 of the second); only the RNG stream
/// changes. This intentionally differs from the historical MT19937 field.
pub fn white_noise_world(
    world_seed: u64,
    attempt: u8,
    width: usize,
    height: usize,
    max_bytes: u64,
) -> Result<Vec<i64>, SpectralError> {
    let key = SeedKey::new(Tier::Continent, Stage::Relief, 0, 0, attempt);
    let mut rng = world_rng(world_seed, key);
    generate_white(width, height, max_bytes, || {
        phase53(rng.next_u32(), rng.next_u32())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_mt19937_reference_words() {
        let mut rng = Mt19937::new(5489);
        assert_eq!(
            (0..5).map(|_| rng.next()).collect::<Vec<_>>(),
            [
                3_499_211_612,
                581_869_302,
                3_890_346_734,
                3_586_334_585,
                545_404_204
            ]
        );
    }

    #[test]
    fn paired_draws_match_native_seed42_across_twists() {
        // NumPy RandomState(42) reference at fixed positions, including the
        // first sample after multiple MT state refreshes and the final cell.
        let expected = [
            (0, 3_373_557_479_352_566),
            (1, 8_563_273_192_166_996),
            (311, 706_672_259_555_733),
            (312, 228_339_197_198_839),
            (313, 8_670_766_083_264_604),
            (623, 4_364_195_071_117_448),
            (624, 5_568_743_917_367_744),
            (625, 3_322_878_659_396_641),
            (1023, 8_746_515_102_163_596),
            (4095, 4_191_382_484_583_197),
            (262_143, 2_319_662_611_339_112),
        ];
        let mut rng = Mt19937::new(42);
        let mut at = 0;
        for (position, wanted) in expected {
            while at < position {
                let _ = rng.phase53();
                at += 1;
            }
            assert_eq!(rng.phase53(), wanted, "phase {position}");
            at += 1;
        }
    }

    #[test]
    fn phase_axes_and_last_segment_are_bounded() {
        assert_eq!(cosine_q60(0).unwrap(), 1_i64 << 60);
        assert_eq!(cosine_q60(1_u64 << 51).unwrap(), 0);
        assert_eq!(cosine_q60(1_u64 << 52).unwrap(), -(1_i64 << 60));
        assert_eq!(cosine_q60(3_u64 << 51).unwrap(), 0);
        assert!((0..=1_i64 << 60).contains(&cosine_q60((1_u64 << 53) - 1).unwrap()));
    }

    #[test]
    fn white_allocation_is_admitted_and_repeats() {
        assert!(matches!(
            white_noise(42, 8, 8, 511),
            Err(SpectralError::ResourceLimit)
        ));
        let a = white_noise(42, 8, 8, 512).unwrap();
        assert_eq!(a, white_noise(42, 8, 8, 512).unwrap());
        assert_ne!(a, white_noise(43, 8, 8, 512).unwrap());
        assert!(a.iter().all(|&v| v.abs() <= 1_i64 << 60));
    }

    #[test]
    fn world_stream_matches_core_key_and_pinned_phases() {
        let key = SeedKey::new(Tier::Continent, Stage::Relief, 0, 0, 0);
        let mut rng = world_rng(42, key);
        let raw: Vec<_> = (0..8).map(|_| rng.next_u32()).collect();
        assert_eq!(
            raw,
            [
                3_625_273_031,
                2_505_261_950,
                1_700_667_286,
                1_791_905_562,
                3_081_249_771,
                1_801_665_087,
                1_928_406_092,
                2_656_173_969,
            ]
        );
        let actual = white_noise_world(42, 0, 4, 2, 64).unwrap();
        assert_eq!(actual[0], cosine_q60(phase53(raw[0], raw[1])).unwrap());
        assert_eq!(actual[1], cosine_q60(phase53(raw[2], raw[3])).unwrap());
        assert_eq!(
            actual,
            [
                642_482_829_702_831_801,
                -915_265_158_910_886_779,
                -234_440_625_679_284_263,
                -1_094_215_090_947_621_759,
                927_015_876_041_153_660,
                -195_868_258_911_111_346,
                -1_043_895_333_307_983_607,
                -338_825_724_806_824_630,
            ]
        );
        assert_eq!(
            white_noise(42, 4, 2, 64).unwrap(),
            [
                -812_879_468_611_418_660,
                1_098_081_311_674_854_795,
                -130_158_129_791_199_804,
                -938_411_862_175_052_670,
                641_921_674_994_949_201,
                642_066_810_111_656_572,
                1_076_991_803_733_337_363,
                768_810_646_779_371_603,
            ]
        );
    }

    #[test]
    fn world_seed_attempt_shape_and_budget_are_independent() {
        let base = white_noise_world(42, 0, 8, 8, 512).unwrap();
        assert_eq!(base, white_noise_world(42, 0, 8, 8, 512).unwrap());
        assert_ne!(
            base,
            white_noise_world(42 | (1_u64 << 32), 0, 8, 8, 512).unwrap()
        );
        assert_ne!(base, white_noise_world(42, 1, 8, 8, 512).unwrap());
        assert_ne!(base, white_noise(42, 8, 8, 512).unwrap());
        assert_eq!(
            white_noise_world(42, 0, 8, 8, 511),
            Err(SpectralError::ResourceLimit)
        );
        assert_eq!(
            white_noise_world(42, 0, 8, 7, 512),
            Err(SpectralError::InvalidDimensions)
        );
        assert!(base.iter().all(|&v| v.unsigned_abs() <= (1_u64 << 60)));
    }
}
