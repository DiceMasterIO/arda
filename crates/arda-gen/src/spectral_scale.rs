//! Fixed-gain, physically band-limited relief from one reference calibration.

use super::{narrow, round_ratio, transform_2d, validated_shape, Complex, SpectralError, Q60_I64};
use arda_core::HeightMm;

/// Immutable gain calibrated from one square reference field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReliefScale {
    reference_period_samples: u32,
    span_mm: u32,
    calibration_raw_range: i128,
}

impl ReliefScale {
    /// Longest retained wavelength, in source sample intervals.
    #[must_use]
    pub const fn reference_period_samples(&self) -> u32 {
        self.reference_period_samples
    }

    /// Reference field's positive peak-to-trough height span.
    #[must_use]
    pub const fn span_mm(&self) -> u32 {
        self.span_mm
    }

    /// Reference filtered range in unscaled Q60 units.
    #[must_use]
    pub const fn calibration_raw_range(&self) -> i128 {
        self.calibration_raw_range
    }
}

fn admitted_count(
    input: &[i64],
    width: usize,
    height: usize,
    max_scratch_bytes: u64,
) -> Result<usize, SpectralError> {
    let count = validated_shape(width, height)?;
    if input.len() != count {
        return Err(SpectralError::LengthMismatch);
    }
    if input
        .iter()
        .any(|&sample| sample.unsigned_abs() > Q60_I64.unsigned_abs())
    {
        return Err(SpectralError::InvalidNoiseRange);
    }
    let grid = count
        .checked_mul(std::mem::size_of::<Complex>() + std::mem::size_of::<HeightMm>())
        .ok_or(SpectralError::ArithmeticOverflow)?;
    let column = height
        .checked_mul(std::mem::size_of::<Complex>())
        .ok_or(SpectralError::ArithmeticOverflow)?;
    let peak = grid
        .checked_add(column)
        .ok_or(SpectralError::ArithmeticOverflow)?;
    if u64::try_from(peak).map_err(|_| SpectralError::ArithmeticOverflow)? > max_scratch_bytes {
        return Err(SpectralError::ResourceLimit);
    }
    Ok(count)
}

fn banded_coefficient(
    coefficient: Complex,
    k_squared: i128,
    long: usize,
    period: u32,
) -> Result<Complex, SpectralError> {
    if k_squared == 0 {
        return Ok(Complex::default());
    }
    let long_squared = i128::try_from(long)
        .map_err(|_| SpectralError::ArithmeticOverflow)?
        .pow(2);
    let period_squared = i128::from(period).pow(2);
    let denominator = k_squared
        .checked_mul(period_squared)
        .ok_or(SpectralError::ArithmeticOverflow)?;
    if denominator < long_squared {
        return Ok(Complex::default());
    }
    Ok(Complex {
        re: narrow(round_ratio(
            i128::from(coefficient.re) * long_squared,
            denominator,
        ))?,
        im: narrow(round_ratio(
            i128::from(coefficient.im) * long_squared,
            denominator,
        ))?,
    })
}

fn filtered_field(
    input: &[i64],
    width: usize,
    height: usize,
    period: Option<u32>,
    max_scratch_bytes: u64,
) -> Result<Vec<Complex>, SpectralError> {
    let count = admitted_count(input, width, height, max_scratch_bytes)?;
    let mut field = Vec::new();
    field
        .try_reserve_exact(count)
        .map_err(|_| SpectralError::ResourceLimit)?;
    field.extend(input.iter().copied().map(|re| Complex { re, im: 0 }));
    let mut column = Vec::new();
    column
        .try_reserve_exact(height)
        .map_err(|_| SpectralError::ResourceLimit)?;
    transform_2d(&mut field, width, height, &mut column, false)?;

    let long = width.max(height);
    let xscale = long / width;
    let yscale = long / height;
    for y in 0..height {
        let ky = y.min(height - y) * yscale;
        for x in 0..width {
            let at = y * width + x;
            if x == 0 && y == 0 {
                field[at] = Complex::default();
                continue;
            }
            let kx = x.min(width - x) * xscale;
            let denominator =
                i128::try_from(kx * kx + ky * ky).map_err(|_| SpectralError::ArithmeticOverflow)?;
            if let Some(p) = period {
                field[at] = banded_coefficient(field[at], denominator, long, p)?;
            } else {
                field[at].re = narrow(round_ratio(i128::from(field[at].re), denominator))?;
                field[at].im = narrow(round_ratio(i128::from(field[at].im), denominator))?;
            }
        }
    }
    transform_2d(&mut field, width, height, &mut column, true)?;
    Ok(field)
}

/// Calibrates a fixed relief scale from a square reference white field.
///
/// The reference uses the legacy inverse-square filter without a wavelength
/// cutoff. Its FFT output has been divided by the square's cell count, so
/// the stored raw range multiplies by that count before height conversion.
pub fn calibration(
    input: &[i64],
    side: usize,
    span_mm: u32,
    max_scratch_bytes: u64,
) -> Result<ReliefScale, SpectralError> {
    if span_mm == 0 || span_mm > i32::MAX.unsigned_abs() {
        return Err(SpectralError::InvalidReliefScale);
    }
    let period = u32::try_from(side).map_err(|_| SpectralError::InvalidDimensions)?;
    let field = filtered_field(input, side, side, None, max_scratch_bytes)?;
    let lo = field
        .iter()
        .map(|c| c.re)
        .min()
        .ok_or(SpectralError::NoRelief)?;
    let hi = field
        .iter()
        .map(|c| c.re)
        .max()
        .ok_or(SpectralError::NoRelief)?;
    if lo == hi {
        return Err(SpectralError::NoRelief);
    }
    let count = i128::try_from(field.len()).map_err(|_| SpectralError::ArithmeticOverflow)?;
    let raw_range = (i128::from(hi) - i128::from(lo))
        .checked_mul(count)
        .ok_or(SpectralError::ArithmeticOverflow)?;
    Ok(ReliefScale {
        reference_period_samples: period,
        span_mm,
        calibration_raw_range: raw_range,
    })
}

/// Filters one whole white field to signed, zero-DC millimetre relief.
///
/// Modes with wavelength longer than `scale.reference_period_samples()` at
/// equal X/Y sample spacing are removed. Remaining coefficients receive the
/// inverse-square physical gain without per-field min/max normalization.
pub fn filter_relief(
    input: &[i64],
    width: usize,
    height: usize,
    scale: &ReliefScale,
    max_scratch_bytes: u64,
) -> Result<Vec<HeightMm>, SpectralError> {
    let field = filtered_field(
        input,
        width,
        height,
        Some(scale.reference_period_samples),
        max_scratch_bytes,
    )?;
    let count = i128::try_from(field.len()).map_err(|_| SpectralError::ArithmeticOverflow)?;
    let mut out = Vec::new();
    out.try_reserve_exact(field.len())
        .map_err(|_| SpectralError::ResourceLimit)?;
    for c in field {
        let numerator = i128::from(c.re)
            .checked_mul(count)
            .and_then(|n| n.checked_mul(i128::from(scale.span_mm)))
            .ok_or(SpectralError::ArithmeticOverflow)?;
        let offset = round_ratio(numerator, scale.calibration_raw_range);
        out.push(HeightMm::new(
            i32::try_from(offset).map_err(|_| SpectralError::ArithmeticOverflow)?,
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spectral::{filter_heights, white_noise};

    #[test]
    fn reference_is_legacy_height_up_to_constant_translation_and_rounding() {
        let input = white_noise(42, 8, 8, 512).unwrap();
        let scale = calibration(&input, 8, 3000, 10_000).unwrap();
        assert_eq!(scale.reference_period_samples(), 8);
        assert_eq!(scale.span_mm(), 3000);
        assert!(scale.calibration_raw_range() > 0);
        let legacy =
            filter_heights(&input, 8, 8, HeightMm::new(1), HeightMm::new(3001), 10_000).unwrap();
        let relief = filter_relief(&input, 8, 8, &scale, 10_000).unwrap();
        let translation = legacy[0].raw() - relief[0].raw();
        for (a, b) in legacy.iter().zip(relief) {
            assert!((a.raw() - b.raw() - translation).abs() <= 1);
        }
    }

    #[test]
    fn repeated_mode_has_equal_physical_amplitude_on_rectangle_and_square() {
        let q = Q60_I64;
        let reference: Vec<i64> = (0..8).flat_map(|_| [q, 0, -q, 0, q, 0, -q, 0]).collect();
        let scale = calibration(&reference, 8, 3000, 10_000).unwrap();
        let wider: Vec<i64> = (0..8)
            .flat_map(|_| [q, 0, -q, 0, q, 0, -q, 0, q, 0, -q, 0, q, 0, -q, 0])
            .collect();
        let larger_square: Vec<i64> = (0..16)
            .flat_map(|_| [q, 0, -q, 0, q, 0, -q, 0, q, 0, -q, 0, q, 0, -q, 0])
            .collect();
        let a = filter_relief(&reference, 8, 8, &scale, 10_000).unwrap();
        let b = filter_relief(&wider, 16, 8, &scale, 10_000).unwrap();
        let c = filter_relief(&larger_square, 16, 16, &scale, 10_000).unwrap();
        for y in 0..8 {
            for x in 0..8 {
                assert!((a[y * 8 + x].raw() - b[y * 16 + x].raw()).abs() <= 1);
                assert!((a[y * 8 + x].raw() - c[y * 16 + x].raw()).abs() <= 1);
            }
        }
    }

    #[test]
    fn longer_than_reference_period_is_removed() {
        let c = Complex { re: Q60_I64, im: 0 };
        assert_eq!(banded_coefficient(c, 1, 16, 8).unwrap(), Complex::default());
        assert_ne!(banded_coefficient(c, 4, 16, 8).unwrap(), Complex::default());
    }

    #[test]
    fn zero_field_has_zero_relief_and_bad_scales_are_rejected() {
        let reference = white_noise(42, 8, 8, 512).unwrap();
        assert_eq!(
            calibration(&reference, 8, 0, 10_000),
            Err(SpectralError::InvalidReliefScale)
        );
        assert_eq!(
            calibration(&reference, 8, u32::MAX, 10_000),
            Err(SpectralError::InvalidReliefScale)
        );
        assert_eq!(
            calibration(&[0; 64], 8, 3000, 10_000),
            Err(SpectralError::NoRelief)
        );
        let scale = calibration(&reference, 8, 3000, 10_000).unwrap();
        let out = filter_relief(&[0; 64], 8, 8, &scale, 10_000).unwrap();
        assert!(out.iter().all(|&h| h == HeightMm::SEA_LEVEL));
        assert_eq!(
            filter_relief(&[i64::MIN; 64], 8, 8, &scale, 10_000),
            Err(SpectralError::InvalidNoiseRange)
        );
        assert_eq!(
            filter_relief(&[0; 64], 8, 8, &scale, 1),
            Err(SpectralError::ResourceLimit)
        );
    }
}
