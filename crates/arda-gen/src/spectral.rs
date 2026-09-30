//! Deterministic, integer Fourier filtering of a supplied white field.
//!
//! This is a source filter, not a height generator or coast policy. Its FFT
//! divides by two at every butterfly in *both* directions; global min/max
//! mapping cancels the resulting positive common scale factor.

use arda_core::HeightMm;
use thiserror::Error;

#[path = "spectral_tables.rs"]
pub(super) mod spectral_tables;
#[path = "spectral_white.rs"]
mod spectral_white;
pub use spectral_white::{white_noise, white_noise_world};
#[path = "spectral_scale.rs"]
mod spectral_scale;
pub use spectral_scale::{calibration, filter_relief, ReliefScale};

const Q60_I64: i64 = 1_i64 << 60;
const Q60: i128 = 1_i128 << 60;
const MAX_AXIS: usize = 32_768;

/// Invalid filter input, insufficient caller-admitted scratch, or arithmetic failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum SpectralError {
    /// Both axes must be supported radix-two lengths.
    #[error("spectral axes must be powers of two between 2 and 32768")]
    InvalidDimensions,
    /// The white-field length does not equal width times height.
    #[error("spectral input length does not match dimensions")]
    LengthMismatch,
    /// A supplied white sample is outside the fixed-point unit interval.
    #[error("spectral Q60 white sample lies outside [-1, 1]")]
    InvalidNoiseRange,
    /// Output height bounds must be strictly increasing.
    #[error("spectral output height range must be increasing")]
    InvalidHeightRange,
    /// Relief calibration requires a positive, representable millimetre span.
    #[error("spectral relief scale is invalid")]
    InvalidReliefScale,
    /// A precomputed Q16 macro-relief gate exceeded its full-scale value.
    #[error("spectral relief gate is outside Q16 bounds")]
    InvalidReliefGate,
    /// The supplied field has no non-DC relief after filtering.
    #[error("spectral field has no relief")]
    NoRelief,
    /// A checked size or fixed-point operation exceeded its representation.
    #[error("spectral arithmetic overflow")]
    ArithmeticOverflow,
    /// Working storage exceeds the caller's budget or cannot be reserved.
    #[error("spectral scratch budget exceeded")]
    ResourceLimit,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Complex {
    re: i64,
    im: i64,
}

/// Checks supported FFT axes and their addressable product.
pub(super) fn validated_shape(width: usize, height: usize) -> Result<usize, SpectralError> {
    if !(2..=MAX_AXIS).contains(&width)
        || !(2..=MAX_AXIS).contains(&height)
        || !width.is_power_of_two()
        || !height.is_power_of_two()
    {
        return Err(SpectralError::InvalidDimensions);
    }
    width
        .checked_mul(height)
        .ok_or(SpectralError::ArithmeticOverflow)
}

fn round_ratio(numerator: i128, denominator: i128) -> i128 {
    // Callers use strictly positive, bounded denominators and products.
    if numerator < 0 {
        -((-numerator + denominator / 2) / denominator)
    } else {
        (numerator + denominator / 2) / denominator
    }
}

fn narrow(value: i128) -> Result<i64, SpectralError> {
    i64::try_from(value).map_err(|_| SpectralError::ArithmeticOverflow)
}

fn multiply(a: Complex, b: Complex) -> Result<Complex, SpectralError> {
    let real = i128::from(a.re) * i128::from(b.re) - i128::from(a.im) * i128::from(b.im);
    let imag = i128::from(a.re) * i128::from(b.im) + i128::from(a.im) * i128::from(b.re);
    Ok(Complex {
        re: narrow(round_ratio(real, Q60))?,
        im: narrow(round_ratio(imag, Q60))?,
    })
}

fn fft(values: &mut [Complex], inverse: bool) -> Result<(), SpectralError> {
    let n = values.len();
    if n < 2 || !n.is_power_of_two() || n > MAX_AXIS {
        return Err(SpectralError::InvalidDimensions);
    }
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            values.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let table_index = usize::try_from(len.trailing_zeros())
            .map_err(|_| SpectralError::ArithmeticOverflow)?
            - 1;
        let (root_re, root_im) = spectral_tables::ROOTS_Q60[table_index];
        let root = Complex {
            re: root_re,
            im: if inverse { -root_im } else { root_im },
        };
        for start in (0..n).step_by(len) {
            let mut factor = Complex { re: Q60_I64, im: 0 };
            for offset in 0..len / 2 {
                let a = values[start + offset];
                let b = multiply(factor, values[start + offset + len / 2])?;
                values[start + offset] = Complex {
                    re: narrow(round_ratio(i128::from(a.re) + i128::from(b.re), 2))?,
                    im: narrow(round_ratio(i128::from(a.im) + i128::from(b.im), 2))?,
                };
                values[start + offset + len / 2] = Complex {
                    re: narrow(round_ratio(i128::from(a.re) - i128::from(b.re), 2))?,
                    im: narrow(round_ratio(i128::from(a.im) - i128::from(b.im), 2))?,
                };
                factor = multiply(factor, root)?;
            }
        }
        len *= 2;
    }
    Ok(())
}

fn transform_2d(
    field: &mut [Complex],
    width: usize,
    height: usize,
    column: &mut Vec<Complex>,
    inverse: bool,
) -> Result<(), SpectralError> {
    for row in field.chunks_exact_mut(width) {
        fft(row, inverse)?;
    }
    for x in 0..width {
        column.clear();
        for y in 0..height {
            column.push(field[y * width + x]);
        }
        fft(column, inverse)?;
        for y in 0..height {
            field[y * width + x] = column[y];
        }
    }
    Ok(())
}

/// Filters caller-supplied Q60 real white noise by physical `1/|frequency|²`
/// and maps its filtered range once to millimetre heights.
///
/// Physical sample spacing is assumed equal on X and Y. For a rectangular
/// grid, `long/width` and `long/height` express integer frequencies on the
/// longer axis's wavelength scale. DC is removed. The borrowed input is
/// excluded from `max_scratch_bytes`; admission includes the complex grid,
/// one complex column, and the output height vector. All allocations are
/// fallible. The output is row-major and no query or region is normalized
/// separately.
pub fn filter_heights(
    input: &[i64],
    width: usize,
    height: usize,
    minimum: HeightMm,
    maximum: HeightMm,
    max_scratch_bytes: u64,
) -> Result<Vec<HeightMm>, SpectralError> {
    let count = validated_shape(width, height)?;
    if input.len() != count {
        return Err(SpectralError::LengthMismatch);
    }
    if minimum >= maximum {
        return Err(SpectralError::InvalidHeightRange);
    }
    if input
        .iter()
        .any(|&sample| sample.unsigned_abs() > Q60_I64.unsigned_abs())
    {
        return Err(SpectralError::InvalidNoiseRange);
    }
    let peak = count
        .checked_mul(std::mem::size_of::<Complex>() + std::mem::size_of::<HeightMm>())
        .and_then(|n| n.checked_add(height * std::mem::size_of::<Complex>()))
        .ok_or(SpectralError::ArithmeticOverflow)?;
    if u64::try_from(peak).map_err(|_| SpectralError::ArithmeticOverflow)? > max_scratch_bytes {
        return Err(SpectralError::ResourceLimit);
    }
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
            field[at].re = narrow(round_ratio(i128::from(field[at].re), denominator))?;
            field[at].im = narrow(round_ratio(i128::from(field[at].im), denominator))?;
        }
    }
    transform_2d(&mut field, width, height, &mut column, true)?;
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
    let denominator = i128::from(hi) - i128::from(lo);
    let range = i128::from(maximum.raw()) - i128::from(minimum.raw());
    let mut out = Vec::new();
    out.try_reserve_exact(count)
        .map_err(|_| SpectralError::ResourceLimit)?;
    for c in field {
        let delta = (i128::from(c.re) - i128::from(lo)) * range;
        let h = i128::from(minimum.raw()) + round_ratio(delta, denominator);
        out.push(HeightMm::new(
            i32::try_from(h).map_err(|_| SpectralError::ArithmeticOverflow)?,
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter(input: &[i64], width: usize, height: usize) -> Result<Vec<HeightMm>, SpectralError> {
        filter_heights(
            input,
            width,
            height,
            HeightMm::new(-1000),
            HeightMm::new(1000),
            1_000_000,
        )
    }

    #[test]
    fn dimensions_length_range_and_budget_are_checked() {
        assert_eq!(filter(&[0; 6], 2, 3), Err(SpectralError::InvalidDimensions));
        assert_eq!(filter(&[0; 2], 2, 2), Err(SpectralError::LengthMismatch));
        assert_eq!(filter(&[0; 4], 2, 2), Err(SpectralError::NoRelief));
        assert_eq!(
            filter(&[Q60_I64 / 3; 4], 2, 2),
            Err(SpectralError::NoRelief)
        );
        assert_eq!(
            filter(&[i64::MIN, 0, 0, 0], 2, 2),
            Err(SpectralError::InvalidNoiseRange)
        );
        assert_eq!(
            filter(&[Q60_I64 + 1, 0, 0, 0], 2, 2),
            Err(SpectralError::InvalidNoiseRange)
        );
        assert_eq!(
            filter_heights(&[0; 4], 2, 2, HeightMm::new(1), HeightMm::new(1), 1000),
            Err(SpectralError::InvalidHeightRange)
        );
        assert_eq!(
            filter_heights(&[0; 4], 2, 2, HeightMm::new(0), HeightMm::new(1), 1),
            Err(SpectralError::ResourceLimit)
        );
    }

    #[test]
    fn single_fourier_mode_survives_and_preserves_symmetry() {
        let q = Q60_I64;
        let input: Vec<i64> = (0..8).flat_map(|_| [q, 0, -q, 0, q, 0, -q, 0]).collect();
        let out = filter(&input, 8, 8).unwrap();
        for row in out.as_chunks::<8>().0 {
            assert_eq!(row[0], HeightMm::new(1000));
            assert_eq!(row[2], HeightMm::new(-1000));
            assert_eq!(row[1], row[3]);
            assert_eq!(row[1], row[5]);
            assert_eq!(row[0], row[4]);
        }
        assert_eq!(out, filter(&input, 8, 8).unwrap());
    }

    #[test]
    fn impulse_filter_is_periodic_and_reflection_symmetric() {
        let mut input = [0; 64];
        input[4 * 8 + 4] = Q60_I64;
        let out = filter(&input, 8, 8).unwrap();
        for y in 0..8 {
            for x in 0..8 {
                let opposite_x = (8 - x) % 8;
                let opposite_y = (8 - y) % 8;
                assert!((out[y * 8 + x].raw() - out[y * 8 + opposite_x].raw()).abs() <= 1);
                assert!((out[y * 8 + x].raw() - out[opposite_y * 8 + x].raw()).abs() <= 1);
            }
        }
    }

    #[test]
    fn rectangular_equal_physical_frequencies_receive_equal_weight() {
        // 8x4 equal-cell grid: kx=2 and ky=1 both represent two cycles per
        // longer-axis wavelength. A raw index-radius filter would not agree.
        let half = Q60_I64 / 2;
        let wave = [1_i64, 0, -1, 0];
        let input: Vec<i64> = (0..4)
            .flat_map(|y| (0..8).map(move |x| half * (wave[x % 4] + wave[y])))
            .collect();
        let out = filter(&input, 8, 4).unwrap();
        assert!((out[1].raw() - out[8].raw()).abs() <= 1);
        assert_eq!(out[0], HeightMm::new(1000));
        assert_eq!(out[2 + 2 * 8], HeightMm::new(-1000));
    }

    #[test]
    fn normalized_fft_forward_then_inverse_preserves_mode_up_to_common_scale() {
        let q = Q60_I64;
        let mut values = [
            Complex { re: q, im: 0 },
            Complex::default(),
            Complex { re: -q, im: 0 },
            Complex::default(),
        ];
        fft(&mut values, false).unwrap();
        fft(&mut values, true).unwrap();
        assert!((i128::from(values[0].re) - Q60 / 4).abs() <= 2);
        assert!(i128::from(values[1].re).abs() <= 2);
        assert!((i128::from(values[2].re) + Q60 / 4).abs() <= 2);
    }
}
