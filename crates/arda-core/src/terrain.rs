//! Immutable, absolute-coordinate fine terrain samples.
//!
//! The grid owns one canonical set of millimetre heights. Every consumer
//! samples that set; changing query spacing cannot regenerate its landforms.

use crate::HeightMm;
use thiserror::Error;

/// A physical position in absolute micrometres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainPoint {
    /// Eastward coordinate.
    pub x_um: i64,
    /// Southward coordinate.
    pub y_um: i64,
}

/// Invalid grid geometry or sample storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TerrainFieldError {
    /// Both axes require at least two lattice samples.
    #[error("terrain grid axes must each contain at least two samples")]
    InvalidDimensions,
    /// Adjacent samples must be separated by a positive distance.
    #[error("terrain grid spacing must be positive")]
    ZeroSpacing,
    /// The dimensions cannot be represented as an addressable sample count.
    #[error("terrain grid sample count exceeds addressable storage")]
    CapacityOverflow,
    /// The supplied height count differs from width times height.
    #[error("terrain grid height count does not match dimensions")]
    LengthMismatch,
    /// The last sample's absolute coordinate exceeds the signed domain.
    #[error("terrain grid extent overflows absolute coordinates")]
    ExtentOverflow,
}

/// Owned lattice heights with closed first-to-last coverage on each axis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainField {
    origin: TerrainPoint,
    spacing_um: u32,
    width: u32,
    height: u32,
    heights: Vec<HeightMm>,
}

/// Validated dimensions and last absolute coordinates, shared with the file codec.
pub(crate) fn validated_geometry(
    origin: TerrainPoint,
    spacing_um: u32,
    width: u32,
    height: u32,
) -> Result<usize, TerrainFieldError> {
    if width < 2 || height < 2 {
        return Err(TerrainFieldError::InvalidDimensions);
    }
    if spacing_um == 0 {
        return Err(TerrainFieldError::ZeroSpacing);
    }
    let count = usize::try_from(u64::from(width) * u64::from(height))
        .map_err(|_| TerrainFieldError::CapacityOverflow)?;
    count
        .checked_mul(std::mem::size_of::<HeightMm>())
        .ok_or(TerrainFieldError::CapacityOverflow)?;
    let last_x = i128::from(origin.x_um) + i128::from(width - 1) * i128::from(spacing_um);
    let last_y = i128::from(origin.y_um) + i128::from(height - 1) * i128::from(spacing_um);
    if last_x > i128::from(i64::MAX) || last_y > i128::from(i64::MAX) {
        return Err(TerrainFieldError::ExtentOverflow);
    }
    Ok(count)
}

/// Cell and fractional micrometre coordinates for one in-domain query.
pub(crate) struct TerrainStencil {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) fx: i128,
    pub(crate) fy: i128,
    pub(crate) spacing: i128,
}

pub(crate) fn sample_stencil(
    origin: TerrainPoint,
    spacing_um: u32,
    width: u32,
    height: u32,
    point: TerrainPoint,
) -> Option<TerrainStencil> {
    let s = i128::from(spacing_um);
    let dx = i128::from(point.x_um) - i128::from(origin.x_um);
    let dy = i128::from(point.y_um) - i128::from(origin.y_um);
    let xmax = i128::from(width - 1) * s;
    let ymax = i128::from(height - 1) * s;
    if dx < 0 || dx > xmax || dy < 0 || dy > ymax {
        return None;
    }
    let x = (dx / s).min(i128::from(width - 2));
    let y = (dy / s).min(i128::from(height - 2));
    Some(TerrainStencil {
        x: u32::try_from(x).ok()?,
        y: u32::try_from(y).ok()?,
        fx: dx - x * s,
        fy: dy - y * s,
        spacing: s,
    })
}

pub(crate) fn interpolate_stencil(
    stencil: &TerrainStencil,
    corners: [HeightMm; 4],
) -> Option<HeightMm> {
    let s = stencil.spacing;
    let fx = stencil.fx;
    let fy = stencil.fy;
    let raw = corners.map(|h| i128::from(h.raw()));
    let numerator = raw[0] * (s - fx) * (s - fy)
        + raw[1] * fx * (s - fy)
        + raw[2] * (s - fx) * fy
        + raw[3] * fx * fy;
    let denominator = s * s;
    let rounded = if numerator < 0 {
        -((-numerator + denominator / 2) / denominator)
    } else {
        (numerator + denominator / 2) / denominator
    };
    let lo = raw.into_iter().min()?;
    let hi = raw.into_iter().max()?;
    i32::try_from(rounded.clamp(lo, hi)).ok().map(HeightMm::new)
}

impl TerrainField {
    /// Validates and takes ownership of all lattice heights.
    ///
    /// `width` and `height` count samples, so the last coordinate is
    /// `origin + (count - 1) * spacing_um`.
    pub fn new(
        origin: TerrainPoint,
        spacing_um: u32,
        width: u32,
        height: u32,
        heights: Vec<HeightMm>,
    ) -> Result<Self, TerrainFieldError> {
        let count = validated_geometry(origin, spacing_um, width, height)?;
        if heights.len() != count {
            return Err(TerrainFieldError::LengthMismatch);
        }
        Ok(Self {
            origin,
            spacing_um,
            width,
            height,
            heights,
        })
    }

    /// First sample's absolute position.
    #[must_use]
    pub const fn origin(&self) -> TerrainPoint {
        self.origin
    }

    /// Distance between lattice samples, in micrometres.
    #[must_use]
    pub const fn spacing_um(&self) -> u32 {
        self.spacing_um
    }

    /// Number of samples along the eastward axis.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Number of samples along the southward axis.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Row-major owned heights, borrowed without mutable access.
    #[must_use]
    pub fn heights(&self) -> &[HeightMm] {
        &self.heights
    }

    /// Bilinearly interpolates the canonical samples, rounding once to the
    /// nearest millimetre with half-millimetre ties away from zero.
    ///
    /// Both last lattice coordinates are included. Outside coverage returns
    /// `None`; this method never wraps or clamps a query to the grid.
    #[must_use]
    pub fn sample(&self, point: TerrainPoint) -> Option<HeightMm> {
        let stencil = sample_stencil(self.origin, self.spacing_um, self.width, self.height, point)?;
        let x = usize::try_from(stencil.x).ok()?;
        let y = usize::try_from(stencil.y).ok()?;
        let width = usize::try_from(self.width).ok()?;
        let at = |ix: usize, iy: usize| -> Option<HeightMm> {
            self.heights
                .get(iy.checked_mul(width)?.checked_add(ix)?)
                .copied()
        };
        let corners = [at(x, y)?, at(x + 1, y)?, at(x, y + 1)?, at(x + 1, y + 1)?];
        interpolate_stencil(&stencil, corners)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(
        origin: TerrainPoint,
        spacing: u32,
        width: u32,
        height: u32,
        values: &[i32],
    ) -> TerrainField {
        TerrainField::new(
            origin,
            spacing,
            width,
            height,
            values.iter().copied().map(HeightMm::new).collect(),
        )
        .unwrap()
    }

    #[test]
    fn every_lattice_point_is_exact() {
        let f = field(
            TerrainPoint { x_um: -7, y_um: 19 },
            39_062_500,
            3,
            2,
            &[1, -2, 3, 4, 5, -6],
        );
        for y in 0..2 {
            for x in 0..3 {
                let p = TerrainPoint {
                    x_um: -7 + i64::from(x) * 39_062_500,
                    y_um: 19 + i64::from(y) * 39_062_500,
                };
                let index = usize::try_from(y * 3 + x).unwrap();
                assert_eq!(f.sample(p), Some(f.heights()[index]));
            }
        }
    }

    #[test]
    fn signed_plane_and_negative_origin_interpolate_at_fractions() {
        let origin = TerrainPoint {
            x_um: -100,
            y_um: -200,
        };
        let f = field(origin, 100, 2, 2, &[-1000, -800, -700, -500]);
        assert_eq!(
            f.sample(TerrainPoint {
                x_um: -75,
                y_um: -125
            }),
            Some(HeightMm::new(-725))
        );
        assert_eq!(
            f.sample(TerrainPoint {
                x_um: 0,
                y_um: -100
            }),
            Some(HeightMm::new(-500))
        );
    }

    #[test]
    fn half_millimetre_ties_round_away_from_zero() {
        let origin = TerrainPoint { x_um: 0, y_um: 0 };
        let plus = field(origin, 2, 2, 2, &[0, 1, 0, 1]);
        let minus = field(origin, 2, 2, 2, &[-1, 0, -1, 0]);
        assert_eq!(
            plus.sample(TerrainPoint { x_um: 1, y_um: 0 }),
            Some(HeightMm::new(1))
        );
        assert_eq!(
            minus.sample(TerrainPoint { x_um: 1, y_um: 0 }),
            Some(HeightMm::new(-1))
        );
    }

    #[test]
    fn extreme_signed_corners_remain_convex() {
        let f = field(
            TerrainPoint { x_um: 0, y_um: 0 },
            u32::MAX,
            2,
            2,
            &[i32::MIN, i32::MAX, i32::MAX, i32::MIN],
        );
        assert_eq!(
            f.sample(TerrainPoint {
                x_um: u32::MAX as i64,
                y_um: 0
            }),
            Some(HeightMm::new(i32::MAX))
        );
        let center = TerrainPoint {
            x_um: i64::from(u32::MAX / 2),
            y_um: i64::from(u32::MAX / 2),
        };
        assert!(f.sample(center).is_some());
    }

    #[test]
    fn coverage_is_closed_and_never_clamped() {
        let f = field(
            TerrainPoint {
                x_um: i64::MIN,
                y_um: -10,
            },
            10,
            2,
            2,
            &[1, 2, 3, 4],
        );
        assert_eq!(
            f.sample(TerrainPoint {
                x_um: i64::MIN + 10,
                y_um: 0
            }),
            Some(HeightMm::new(4))
        );
        assert_eq!(
            f.sample(TerrainPoint {
                x_um: i64::MIN + 11,
                y_um: 0
            }),
            None
        );
        assert_eq!(
            f.sample(TerrainPoint {
                x_um: i64::MIN,
                y_um: -11
            }),
            None
        );
        assert_eq!(
            f.sample(TerrainPoint {
                x_um: i64::MAX,
                y_um: 0
            }),
            None
        );
    }

    #[test]
    fn invalid_geometry_and_storage_are_rejected() {
        let o = TerrainPoint { x_um: 0, y_um: 0 };
        assert_eq!(
            TerrainField::new(o, 1, 1, 2, vec![]),
            Err(TerrainFieldError::InvalidDimensions)
        );
        assert_eq!(
            TerrainField::new(o, 0, 2, 2, vec![]),
            Err(TerrainFieldError::ZeroSpacing)
        );
        assert_eq!(
            TerrainField::new(o, 1, 2, 2, vec![]),
            Err(TerrainFieldError::LengthMismatch)
        );
        let edge = TerrainPoint {
            x_um: i64::MAX,
            y_um: 0,
        };
        assert_eq!(
            TerrainField::new(edge, 1, 2, 2, vec![HeightMm::new(0); 4]),
            Err(TerrainFieldError::ExtentOverflow)
        );
    }

    #[test]
    fn shared_boundary_queries_do_not_depend_on_order_or_spacing() {
        let f = field(
            TerrainPoint {
                x_um: -10,
                y_um: -10,
            },
            10,
            3,
            3,
            &[0, 10, 20, 10, 20, 30, 20, 30, 40],
        );
        let p = TerrainPoint { x_um: 0, y_um: 0 };
        for step in [1, 2, 5, 10] {
            for delta in (-10..=10).step_by(step) {
                let _ = f.sample(TerrainPoint {
                    x_um: i64::from(delta),
                    y_um: 0,
                });
            }
            assert_eq!(f.sample(p), Some(HeightMm::new(20)));
        }
    }
}
