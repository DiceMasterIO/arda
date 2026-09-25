//! Fine land material from one immutable, absolute-coordinate terrain window.

use arda_core::{AreaCoord, TerrainField, TerrainKind, TerrainPoint};

use super::{
    invalid, land_material, modulate, relief_light, sample_index, AxisKernel, RenderError,
    SAMPLE_SIDE,
};

const CELL_UM: i128 = 100_000_000;
const AREA_UM: i128 = 512 * CELL_UM;

/// Closed physical coverage of the canonical fine source, in micrometres.
///
/// These are the first and last fine source nodes, which may extend past the
/// saved-world edge. They are not the bounds of an area window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasFineWorldBounds {
    /// Northwest source node.
    pub min: TerrainPoint,
    /// Southeast source node.
    pub max: TerrainPoint,
}

#[derive(Debug)]
pub(super) struct FineAtlas {
    field: TerrainField,
    origin_x: i128,
    origin_y: i128,
    bounds: AtlasFineWorldBounds,
    // North, east, south, west are actual saved-world edges from AtlasHalo.
    edges: [bool; 4],
}

impl FineAtlas {
    pub(super) fn new(
        area: AreaCoord,
        bounds: AtlasFineWorldBounds,
        field: TerrainField,
        edges: [bool; 4],
    ) -> Result<Self, RenderError> {
        if bounds.min.x_um > bounds.max.x_um || bounds.min.y_um > bounds.max.y_um {
            return Err(invalid("fine source bounds are reversed"));
        }
        let origin_x = i128::from(area.x) * AREA_UM;
        let origin_y = i128::from(area.y) * AREA_UM;
        let last_x = origin_x + 511 * CELL_UM;
        let last_y = origin_y + 511 * CELL_UM;
        if origin_x < i128::from(bounds.min.x_um)
            || origin_y < i128::from(bounds.min.y_um)
            || last_x > i128::from(bounds.max.x_um)
            || last_y > i128::from(bounds.max.y_um)
        {
            return Err(invalid("saved area centers lie outside fine source"));
        }
        let this = Self {
            field,
            origin_x,
            origin_y,
            bounds,
            edges,
        };
        this.validate_coverage()?;
        Ok(this)
    }

    /// The possible 32K Linear centers extend by almost half a saved cell;
    /// a fine-spacing derivative extends the required window once more.
    fn validate_coverage(&self) -> Result<(), RenderError> {
        let spacing = i128::from(self.field.spacing_um());
        let extremes_x = [
            self.origin_x - CELL_UM / 2 - spacing,
            self.origin_x + 511 * CELL_UM + CELL_UM / 2 + spacing,
        ];
        let extremes_y = [
            self.origin_y - CELL_UM / 2 - spacing,
            self.origin_y + 511 * CELL_UM + CELL_UM / 2 + spacing,
        ];
        for &x in &extremes_x {
            for &y in &extremes_y {
                let point = self.clamp_source(x, y)?;
                if self.field.sample(point).is_none() {
                    return Err(invalid("fine window omits pixel or derivative halo"));
                }
            }
        }
        Ok(())
    }

    fn clamp_source(&self, x: i128, y: i128) -> Result<TerrainPoint, RenderError> {
        let x = if x < i128::from(self.bounds.min.x_um) {
            if !self.edges[3] {
                return Err(invalid("fine query west of in-world source"));
            }
            i128::from(self.bounds.min.x_um)
        } else if x > i128::from(self.bounds.max.x_um) {
            if !self.edges[1] {
                return Err(invalid("fine query east of in-world source"));
            }
            i128::from(self.bounds.max.x_um)
        } else {
            x
        };
        let y = if y < i128::from(self.bounds.min.y_um) {
            if !self.edges[0] {
                return Err(invalid("fine query north of in-world source"));
            }
            i128::from(self.bounds.min.y_um)
        } else if y > i128::from(self.bounds.max.y_um) {
            if !self.edges[2] {
                return Err(invalid("fine query south of in-world source"));
            }
            i128::from(self.bounds.max.y_um)
        } else {
            y
        };
        Ok(TerrainPoint {
            x_um: i64::try_from(x).map_err(|_| invalid("fine x coordinate overflow"))?,
            y_um: i64::try_from(y).map_err(|_| invalid("fine y coordinate overflow"))?,
        })
    }

    fn height(&self, x: i128, y: i128) -> Result<i32, RenderError> {
        let point = self.clamp_source(x, y)?;
        self.field
            .sample(point)
            .map(|h| h.raw())
            .ok_or_else(|| invalid("fine window omits requested height"))
    }

    fn material(&self, x: i128, y: i128, wetness: u8) -> Result<([u8; 3], u16), RenderError> {
        let point = self.clamp_source(x, y)?;
        let x = i128::from(point.x_um);
        let y = i128::from(point.y_um);
        let s = i128::from(self.field.spacing_um());
        let height = self.height(x, y)?;
        let left = self.clamp_source(x - s, y)?;
        let right = self.clamp_source(x + s, y)?;
        let up = self.clamp_source(x, y - s)?;
        let down = self.clamp_source(x, y + s)?;
        let dx = equivalent_delta(
            self.height(i128::from(right.x_um), y)?,
            self.height(i128::from(left.x_um), y)?,
            i128::from(right.x_um) - i128::from(left.x_um),
        )?;
        let dy = equivalent_delta(
            self.height(x, i128::from(down.y_um))?,
            self.height(x, i128::from(up.y_um))?,
            i128::from(down.y_um) - i128::from(up.y_um),
        )?;
        Ok((land_material(height, dx, dy, wetness), relief_light(dx, dy)))
    }

    pub(super) fn sample(
        &self,
        x: AxisKernel,
        y: AxisKernel,
        classes: &[TerrainKind],
        wetness: &[u8],
    ) -> Result<[u8; 3], RenderError> {
        let (xs, xe, nx) = axis_positions(x, self.origin_x, self.field.spacing_um())?;
        let (ys, ye, ny) = axis_positions(y, self.origin_y, self.field.spacing_um())?;
        let mut color_sum = [0_u128; 3];
        let mut count = 0_u128;
        for iy in 0..ny {
            let py = position(ys, ye - ys, iy, ny);
            for ix in 0..nx {
                let px = position(xs, xe - xs, ix, nx);
                let Some(wet) =
                    saved_wetness(px, py, self.origin_x, self.origin_y, classes, wetness)?
                else {
                    continue;
                };
                let (palette, light) = self.material(px, py, wet)?;
                let color = modulate(palette, light);
                for (sum, c) in color_sum.iter_mut().zip(color) {
                    *sum += u128::from(c);
                }
                count += 1;
            }
        }
        if count == 0 {
            return Err(invalid("fine footprint has no saved land ownership"));
        }
        let mut color = [0_u8; 3];
        for (channel, sum) in color.iter_mut().zip(color_sum) {
            *channel = u8::try_from((sum + count / 2) / count)
                .map_err(|_| invalid("fine averaged RGB outside channel range"))?;
        }
        Ok(color)
    }
}

fn equivalent_delta(high: i32, low: i32, separation_um: i128) -> Result<i64, RenderError> {
    if separation_um <= 0 {
        return Err(invalid("fine derivative has zero physical baseline"));
    }
    let numerator = i128::from(high) - i128::from(low);
    let equivalent = round_div(numerator * 2 * CELL_UM, separation_um);
    i64::try_from(equivalent).map_err(|_| invalid("fine derivative overflow"))
}

fn round_div(n: i128, d: i128) -> i128 {
    if n < 0 {
        -((-n + d / 2) / d)
    } else {
        (n + d / 2) / d
    }
}

fn axis_positions(
    kernel: AxisKernel,
    origin: i128,
    spacing: u32,
) -> Result<(i128, i128, u32), RenderError> {
    match kernel {
        AxisKernel::Linear {
            low,
            high_weight,
            denominator,
        } => {
            let numerator =
                (i128::from(low) * i128::from(denominator) + i128::from(high_weight)) * CELL_UM;
            let p = origin + round_div(numerator, i128::from(denominator));
            Ok((p, p, 1))
        }
        AxisKernel::Box { start, end } => {
            let a = origin + (i128::from(start) * CELL_UM - CELL_UM / 2);
            let b = origin + (i128::from(end) * CELL_UM - CELL_UM / 2);
            let count = u32::try_from(((b - a) + i128::from(spacing) - 1) / i128::from(spacing))
                .map_err(|_| invalid("fine box quadrature count overflow"))?;
            Ok((a, b, count.max(1)))
        }
    }
}

fn position(start: i128, span: i128, index: u32, count: u32) -> i128 {
    if count == 1 && span == 0 {
        start
    } else {
        start + round_div((i128::from(index) * 2 + 1) * span, i128::from(count) * 2)
    }
}

fn saved_wetness(
    x: i128,
    y: i128,
    ox: i128,
    oy: i128,
    classes: &[TerrainKind],
    wetness: &[u8],
) -> Result<Option<u8>, RenderError> {
    if classes.len() != SAMPLE_SIDE * SAMPLE_SIDE || wetness.len() != classes.len() {
        return Err(invalid("fine saved context has wrong dimensions"));
    }
    let qx = (x - ox).div_euclid(CELL_UM).clamp(-1, 511);
    let qy = (y - oy).div_euclid(CELL_UM).clamp(-1, 511);
    let fx = (x - ox - qx * CELL_UM).clamp(0, CELL_UM);
    let fy = (y - oy - qy * CELL_UM).clamp(0, CELL_UM);
    let mut sum = 0_i128;
    let mut weight_sum = 0_i128;
    for (cx, wx) in [(qx, CELL_UM - fx), (qx + 1, fx)] {
        for (cy, wy) in [(qy, CELL_UM - fy), (qy + 1, fy)] {
            let i = sample_index(
                i16::try_from(cx).map_err(|_| invalid("wetness x conversion"))?,
                i16::try_from(cy).map_err(|_| invalid("wetness y conversion"))?,
            )?;
            if classes[i] == TerrainKind::Land {
                let w = wx * wy;
                sum += i128::from(wetness[i]) * w;
                weight_sum += w;
            }
        }
    }
    if weight_sum == 0 {
        Ok(None)
    } else {
        Ok(Some(
            u8::try_from((sum + weight_sum / 2) / weight_sum)
                .map_err(|_| invalid("wetness interpolation overflow"))?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use arda_core::HeightMm;

    use super::*;

    fn field(
        origin: TerrainPoint,
        spacing: u32,
        width: u32,
        height: u32,
        mut height_at: impl FnMut(u32, u32) -> i32,
    ) -> TerrainField {
        let heights = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| HeightMm::new(height_at(x, y)))
            .collect();
        TerrainField::new(origin, spacing, width, height, heights).unwrap()
    }

    fn all_land() -> (Vec<TerrainKind>, Vec<u8>) {
        (
            vec![TerrainKind::Land; SAMPLE_SIDE * SAMPLE_SIDE],
            vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
        )
    }

    #[test]
    fn physical_plane_has_exact_height_and_equivalent_gradient() {
        let s = 39_062_500;
        let field = field(
            TerrainPoint {
                x_um: -2 * i64::from(s),
                y_um: -2 * i64::from(s),
            },
            s,
            5,
            5,
            |x, y| {
                1_500_000 + (i32::try_from(x).unwrap() - 2) * 5_000
                    - (i32::try_from(y).unwrap() - 2) * 2_500
            },
        );
        let atlas = FineAtlas {
            field,
            origin_x: 0,
            origin_y: 0,
            bounds: AtlasFineWorldBounds {
                min: TerrainPoint {
                    x_um: -2 * i64::from(s),
                    y_um: -2 * i64::from(s),
                },
                max: TerrainPoint {
                    x_um: 2 * i64::from(s),
                    y_um: 2 * i64::from(s),
                },
            },
            edges: [false; 4],
        };
        assert_eq!(atlas.height(0, 0).unwrap(), 1_500_000);
        let expected = (
            land_material(1_500_000, 25_600, -12_800, 0),
            relief_light(25_600, -12_800),
        );
        assert_eq!(atlas.material(0, 0, 0).unwrap(), expected);
        let (classes, wetness) = all_land();
        assert_eq!(
            atlas
                .sample(
                    AxisKernel::Linear {
                        low: 0,
                        high_weight: 0,
                        denominator: 2
                    },
                    AxisKernel::Linear {
                        low: 0,
                        high_weight: 0,
                        denominator: 2
                    },
                    &classes,
                    &wetness,
                )
                .unwrap(),
            modulate(expected.0, expected.1)
        );
    }

    #[test]
    fn overlapping_area_queries_share_exact_seam_radiance() {
        let s = 100_000_000;
        let source = field(
            TerrainPoint { x_um: -s, y_um: -s },
            u32::try_from(s).unwrap(),
            1_027,
            3,
            |x, y| {
                1_000_000 + i32::try_from(x).unwrap() * 1_000 + i32::try_from(y).unwrap() * 2_000
            },
        );
        let bounds = AtlasFineWorldBounds {
            min: source.origin(),
            max: TerrainPoint {
                x_um: -s + 1_026 * s,
                y_um: s,
            },
        };
        let left = FineAtlas {
            field: source.clone(),
            origin_x: 0,
            origin_y: 0,
            bounds,
            edges: [false; 4],
        };
        let right = FineAtlas {
            field: source,
            origin_x: AREA_UM,
            origin_y: 0,
            bounds,
            edges: [false; 4],
        };
        let (classes, wetness) = all_land();
        let x_left = AxisKernel::Linear {
            low: 511,
            high_weight: 1,
            denominator: 2,
        };
        let x_right = AxisKernel::Linear {
            low: -1,
            high_weight: 1,
            denominator: 2,
        };
        let y = AxisKernel::Linear {
            low: 0,
            high_weight: 0,
            denominator: 2,
        };
        assert_eq!(
            left.sample(x_left, y, &classes, &wetness).unwrap(),
            right.sample(x_right, y, &classes, &wetness).unwrap()
        );
    }

    #[test]
    fn fine_high_frequency_changes_box_and_linear_samples() {
        let s = 50_000_000;
        let source = field(
            TerrainPoint {
                x_um: -2 * s,
                y_um: -2 * s,
            },
            u32::try_from(s).unwrap(),
            8,
            8,
            |x, _| if x % 2 == 0 { 1_600_000 } else { 1_000_000 },
        );
        let atlas = FineAtlas {
            field: source,
            origin_x: 0,
            origin_y: 0,
            bounds: AtlasFineWorldBounds {
                min: TerrainPoint {
                    x_um: -2 * s,
                    y_um: -2 * s,
                },
                max: TerrainPoint {
                    x_um: 5 * s,
                    y_um: 5 * s,
                },
            },
            edges: [false; 4],
        };
        let (classes, wetness) = all_land();
        let y = AxisKernel::Linear {
            low: 0,
            high_weight: 0,
            denominator: 2,
        };
        let center = atlas
            .sample(
                AxisKernel::Linear {
                    low: 0,
                    high_weight: 0,
                    denominator: 2,
                },
                y,
                &classes,
                &wetness,
            )
            .unwrap();
        let half = atlas
            .sample(
                AxisKernel::Linear {
                    low: 0,
                    high_weight: 1,
                    denominator: 2,
                },
                y,
                &classes,
                &wetness,
            )
            .unwrap();
        assert_ne!(center, half);
        let box_color = atlas
            .sample(AxisKernel::Box { start: 0, end: 1 }, y, &classes, &wetness)
            .unwrap();
        assert_ne!(box_color, center);
        assert_ne!(box_color, half);
    }

    #[test]
    fn only_a_true_world_edge_clamps_outside_source_queries() {
        let source = field(
            TerrainPoint { x_um: 0, y_um: 0 },
            50_000_000,
            3,
            3,
            |x, y| 1_000_000 + i32::try_from(x + y).unwrap() * 1_000,
        );
        let bounds = AtlasFineWorldBounds {
            min: TerrainPoint { x_um: 0, y_um: 0 },
            max: TerrainPoint {
                x_um: 100_000_000,
                y_um: 100_000_000,
            },
        };
        let mut atlas = FineAtlas {
            field: source,
            origin_x: 0,
            origin_y: 0,
            bounds,
            edges: [false; 4],
        };
        assert!(atlas.height(-1, 50_000_000).is_err());
        atlas.edges[3] = true;
        assert_eq!(
            atlas.height(-1, 50_000_000).unwrap(),
            atlas.height(0, 50_000_000).unwrap()
        );
        assert!(atlas.height(50_000_000, -1).is_err());
    }
}
