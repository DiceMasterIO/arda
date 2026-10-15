//! Annual land loss of a cell (logic/02 Steps 4; review 3.2-1).
//!
//! Recipes up to 8 take the dry-land loss as `A = min(P/2, E)`: runoff
//! never falls below half the rain, however arid the cell. Under Budyko's
//! framework (Fu's curve) runoff falls to about 12% of the rain at an
//! aridity index `E/P` of 2 and under 1% at 12, so with the half-rain rule
//! arid basins filled terminal lakes covering whole floors (seed 74 at
//! 15-35°: one lake on 4.7% of the land) and never stayed dry.
//!
//! Recipe 9 uses Fu's curve `A/P = 1 + φ − (1 + φ^ω)^(1/ω)` with `φ = E/P`
//! and ω = 2.6, read from an integer table with linear interpolation (no
//! floating point). It keeps `A <= min(P, E)`.

/// How a cell's annual land loss is taken from its rain and open-water
/// evaporation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RunoffModel {
    /// `A = min(P/2, E)` (recipes up to 8 and legacy terrain).
    #[default]
    HalfRain,
    /// Fu's Budyko curve with ω = 2.6 (recipe 9).
    Budyko,
}

/// Fu's curve, ω = 2.6: `(φ Q12, A/P Q16)` knots. Beyond the last knot
/// the loss is held at its last value (runoff 0.15% of the rain).
const FU: [(u128, u128); 25] = [
    (0, 0),
    (512, 8_079),
    (1_024, 15_704),
    (1_536, 22_654),
    (2_048, 28_805),
    (2_560, 34_117),
    (3_072, 38_619),
    (3_584, 42_385),
    (4_096, 45_514),
    (5_120, 50_260),
    (6_144, 53_557),
    (7_168, 55_899),
    (8_192, 57_609),
    (10_240, 59_875),
    (12_288, 61_264),
    (14_336, 62_179),
    (16_384, 62_816),
    (20_480, 63_626),
    (24_576, 64_106),
    (32_768, 64_632),
    (40_960, 64_903),
    (49_152, 65_063),
    (65_536, 65_238),
    (98_304, 65_380),
    (131_072, 65_438),
];

/// Fu's `A/P` (Q16) at aridity `phi` (Q12).
fn fu_q16(phi_q12: u128) -> u128 {
    for w in FU.windows(2) {
        let ((x0, y0), (x1, y1)) = (w[0], w[1]);
        if phi_q12 <= x1 {
            return y0 + (y1 - y0) * (phi_q12 - x0) / (x1 - x0);
        }
    }
    FU[FU.len() - 1].1
}

impl RunoffModel {
    /// Annual land loss `A` for precipitation `p` and open-water
    /// evaporation `e`, in the same units (litres or millimetres).
    #[must_use]
    pub fn land_loss(self, p: u128, e: u128) -> u128 {
        match self {
            Self::HalfRain => (p / 2).min(e),
            Self::Budyko => {
                if p == 0 {
                    return 0;
                }
                let phi = e.saturating_mul(4_096) / p;
                let a = p.saturating_mul(fu_q16(phi)) / 65_536;
                a.min(p).min(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runoff_permille(model: RunoffModel, p: u128, e: u128) -> u128 {
        (p - model.land_loss(p, e)) * 1_000 / p
    }

    /// Review 3.2-1: the half-rain rule keeps runoff at half the rain at
    /// any aridity; Fu's curve gives about 12% at E/P = 2 and under 1% at 12.
    #[test]
    fn budyko_runoff_falls_with_aridity_and_half_rain_does_not() {
        let p = 500_000;
        assert_eq!(runoff_permille(RunoffModel::HalfRain, p, 2 * p), 500);
        assert_eq!(runoff_permille(RunoffModel::HalfRain, p, 12 * p), 500);
        let at2 = runoff_permille(RunoffModel::Budyko, p, 2 * p);
        let at12 = runoff_permille(RunoffModel::Budyko, p, 12 * p);
        assert!((115..=125).contains(&at2), "E/P = 2: {at2}‰");
        assert!(at12 <= 10, "E/P = 12: {at12}‰");
        // Humid cells lose nearly all their evaporation.
        let humid = RunoffModel::Budyko.land_loss(p, p / 8);
        assert!(humid * 100 >= (p / 8) * 95, "{humid}");
    }

    #[test]
    fn budyko_loss_is_bounded_and_monotone() {
        let mut last = 0;
        for e in (0..40_000_u128).step_by(37) {
            let a = RunoffModel::Budyko.land_loss(1_000, e);
            assert!(a <= 1_000 && a <= e, "{e}: {a}");
            assert!(a >= last, "loss falls at {e}");
            last = a;
        }
        assert_eq!(RunoffModel::Budyko.land_loss(0, 500), 0);
        assert_eq!(RunoffModel::Budyko.land_loss(500, 0), 0);
    }
}
