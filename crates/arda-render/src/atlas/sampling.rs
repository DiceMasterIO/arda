//! Sampling a prepared [`AtlasTerrain`]: cell colours, channel offsets,
//! recipe selection and the anti-aliased pixel kernel with contour class.

use super::fine::{self, FineAtlas};
use super::validate_kernel;
use super::{axis_kernel, invalid, modulate, sample_index, AtlasTerrain, AxisKernel, AREA};
use crate::RenderError;
use arda_core::{AreaCoord, CellCoord, TerrainKind};

impl AtlasTerrain {
    /// Returns the derived color at one saved cell center.
    #[must_use]
    pub fn colour(&self, at: CellCoord) -> [u8; 3] {
        let x = match axis_kernel(u32::from(at.x()), 512) {
            Ok(kernel) => kernel,
            Err(_) => {
                unreachable!("bounds-checked CellCoord yields an exact-center x kernel at T=512")
            }
        };
        let y = match axis_kernel(u32::from(at.y()), 512) {
            Ok(kernel) => kernel,
            Err(_) => {
                unreachable!("bounds-checked CellCoord yields an exact-center y kernel at T=512")
            }
        };
        let index = match sample_index(
            i16::try_from(at.x()).unwrap_or_else(|_| unreachable!("CellCoord x fits i16")),
            i16::try_from(at.y()).unwrap_or_else(|_| unreachable!("CellCoord y fits i16")),
        ) {
            Ok(index) => index,
            Err(_) => unreachable!("bounds-checked CellCoord lies inside atlas sample grid"),
        };
        match self.sample(x, y, self.classes[index]) {
            Ok(rgb) => rgb,
            Err(_) => unreachable!("exact-center kernel retains the center's stored terrain class"),
        }
    }

    /// Whether recipe-5 formed shading and thalweg rivers apply.
    pub(crate) fn is_formed(&self) -> bool {
        self.fine.as_ref().is_some_and(FineAtlas::is_formed)
    }

    /// Recipe-5 river vertex offset from the saved cell centre, micrometres.
    pub(crate) fn channel_offset_um(&self, node: arda_core::GlobalCell) -> (i64, i64) {
        let Some(fine) = self.fine.as_ref() else {
            return (0, 0);
        };
        let (x, y) = (
            i128::from(node.x) * 100_000_000,
            i128::from(node.y) * 100_000_000,
        );
        let (tx, ty) = fine.thalweg_offset_um(x, y).unwrap_or((0, 0));
        let width = fine
            .local_cell(x, y)
            .and_then(|(cx, cy)| sample_index(cx, cy).ok())
            .map_or(0, |i| self.channel_width[i]);
        let (mx, my) = if self.data_meanders {
            (0, 0)
        } else {
            fine.meander_offset_um(x, y, width).unwrap_or((0, 0))
        };
        (tx + mx, ty + my)
    }

    /// Selects the formed look for the world's fine-terrain recipe
    /// (logic/04 §atlas-formed recipes): recipe 6 and later draw the v0.2
    /// look (palette, surface detail, curved rivers); recipe 5 renders
    /// exactly as v0.1 drew it. Constructors default to recipe 6.
    #[must_use]
    pub fn with_recipe(mut self, recipe_version: u16) -> Self {
        if let Some(fine) = self.fine.as_mut() {
            fine.set_recipe(recipe_version);
        }
        self
    }

    /// Whether this formed terrain draws the recipe-6 look.
    pub(crate) fn is_formed_v6(&self) -> bool {
        self.fine
            .as_ref()
            .is_some_and(|f| f.is_formed() && f.is_v6())
    }

    /// Paints recipe-5 shores from the stored shore layer (logic/04
    /// §atlas-formed shore, goals 16 and 31) instead of the height-and-slope
    /// guess: beaches, shingle, cliffs, rocky shores, marsh, tidal flats and
    /// estuaries each get their own material.
    #[must_use]
    pub fn with_shore(mut self, layer: &arda_core::ShoreLayer, area: AreaCoord) -> Self {
        let (gx0, gy0) = (i64::from(area.x) * 512, i64::from(area.y) * 512);
        for y in -1..=AREA {
            for x in -1..=AREA {
                let (gx, gy) = (gx0 + i64::from(x), gy0 + i64::from(y));
                let (Ok(gx), Ok(gy), Ok(i)) =
                    (u32::try_from(gx), u32::try_from(gy), sample_index(x, y))
                else {
                    continue;
                };
                self.shore[i] = layer.class_at(gx, gy) as u8;
            }
        }
        self
    }

    /// Paints recipe-7 arid water (logic/04 §atlas-formed arid basins):
    /// `salt_at(gx, gy)` gives the global 100 m cell's code — saline lake,
    /// salt crust or mudflat ([`SALT_SALINE_LAKE`], [`SALT_CRUST`],
    /// [`SALT_MUDFLAT`]) or [`SALT_NONE`]. Saline lakes are lighter
    /// turquoise, salt pans white crust with pale mudflat margins, and dry
    /// land below sea level inside the continent is drawn as land.
    ///
    /// [`SALT_SALINE_LAKE`]: super::SALT_SALINE_LAKE
    /// [`SALT_CRUST`]: super::SALT_CRUST
    /// [`SALT_MUDFLAT`]: super::SALT_MUDFLAT
    /// [`SALT_NONE`]: super::SALT_NONE
    #[must_use]
    pub fn with_salt(mut self, area: AreaCoord, salt_at: impl Fn(i64, i64) -> u8) -> Self {
        let (gx0, gy0) = (i64::from(area.x) * 512, i64::from(area.y) * 512);
        self.salt = vec![super::SALT_NONE; super::SAMPLE_SIDE * super::SAMPLE_SIDE];
        for y in -1..=AREA {
            for x in -1..=AREA {
                if let Ok(i) = sample_index(x, y) {
                    self.salt[i] = salt_at(gx0 + i64::from(x), gy0 + i64::from(y));
                }
            }
        }
        self
    }

    pub(crate) fn has_lake_depths(&self) -> bool {
        self.has_lake_depths
    }

    pub(crate) fn sample(
        &self,
        x: AxisKernel,
        y: AxisKernel,
        class: TerrainKind,
    ) -> Result<[u8; 3], RenderError> {
        validate_kernel(x)?;
        validate_kernel(y)?;
        if let Some(fine) = self.fine.as_ref().filter(|f| f.is_formed()) {
            {
                return fine.sample_formed(
                    x,
                    y,
                    &fine::SavedFields {
                        classes: &self.classes,
                        wetness: &self.wetness,
                        moisture: &self.moisture,
                        forest_density: &self.forest_density,
                        temperature: &self.temperature,
                        heights: &self.heights,
                        lake_surface: &self.lake_surface,
                        shore: &self.shore,
                        salt: &self.salt,
                    },
                );
            }
        }
        if class == TerrainKind::Land {
            if let Some(fine) = &self.fine {
                return fine.sample(
                    x,
                    y,
                    &self.classes,
                    &self.wetness,
                    &self.moisture,
                    &self.forest_density,
                );
            }
        }
        let mut palette_sum = [0_u64; 3];
        let mut light_sum = 0_u64;
        let mut weight_sum = 0_u64;
        let mut visit = |x: i16, y: i16, weight: u64| -> Result<(), RenderError> {
            let index = sample_index(x, y)?;
            if self.classes[index] == class {
                for (sum, channel) in palette_sum.iter_mut().zip(self.palette[index]) {
                    *sum += u64::from(channel) * weight;
                }
                light_sum += u64::from(self.light[index]) * weight;
                weight_sum += weight;
            }
            Ok(())
        };
        match (x, y) {
            (
                AxisKernel::Linear {
                    low: xl,
                    high_weight: xh,
                    denominator: xd,
                },
                AxisKernel::Linear {
                    low: yl,
                    high_weight: yh,
                    denominator: yd,
                },
            ) => {
                for (cx, wx) in [(xl, xd - xh), (xl + 1, xh)] {
                    for (cy, wy) in [(yl, yd - yh), (yl + 1, yh)] {
                        visit(cx, cy, u64::from(wx) * u64::from(wy))?;
                    }
                }
            }
            (
                AxisKernel::Linear {
                    low,
                    high_weight,
                    denominator,
                },
                AxisKernel::Box { start, end },
            ) => {
                for (cx, wx) in [(low, denominator - high_weight), (low + 1, high_weight)] {
                    for cy in start..end {
                        visit(
                            cx,
                            i16::try_from(cy).map_err(|_| invalid("box row conversion"))?,
                            u64::from(wx),
                        )?;
                    }
                }
            }
            (
                AxisKernel::Box { start, end },
                AxisKernel::Linear {
                    low,
                    high_weight,
                    denominator,
                },
            ) => {
                for cx in start..end {
                    for (cy, wy) in [(low, denominator - high_weight), (low + 1, high_weight)] {
                        visit(
                            i16::try_from(cx).map_err(|_| invalid("box column conversion"))?,
                            cy,
                            u64::from(wy),
                        )?;
                    }
                }
            }
            (AxisKernel::Box { start: xs, end: xe }, AxisKernel::Box { start: ys, end: ye }) => {
                for cx in xs..xe {
                    for cy in ys..ye {
                        visit(
                            i16::try_from(cx).map_err(|_| invalid("box column conversion"))?,
                            i16::try_from(cy).map_err(|_| invalid("box row conversion"))?,
                            1,
                        )?;
                    }
                }
            }
        }
        if weight_sum == 0 {
            return Err(invalid("no sample has the owning terrain class"));
        }
        let mut palette = [0; 3];
        for (channel, sum) in palette.iter_mut().zip(palette_sum) {
            *channel = u8::try_from((sum + weight_sum / 2) / weight_sum)
                .map_err(|_| invalid("palette average outside RGB bounds"))?;
        }
        let light = u16::try_from((light_sum + weight_sum / 2) / weight_sum)
            .map_err(|_| invalid("light average outside Q12 bounds"))?;
        Ok(modulate(palette, light))
    }

    /// Resolves an unambiguous shoreline from saved height samples.
    pub(crate) fn contour_class(
        &self,
        x: AxisKernel,
        y: AxisKernel,
        owner: CellCoord,
        saved: TerrainKind,
    ) -> Result<TerrainKind, RenderError> {
        let (
            AxisKernel::Linear {
                low: xl,
                high_weight: xh,
                denominator: xd,
            },
            AxisKernel::Linear {
                low: yl,
                high_weight: yh,
                denominator: yd,
            },
        ) = (x, y)
        else {
            return Ok(saved);
        };
        if saved == TerrainKind::Lake || (xh == 0 && yh == 0) {
            return Ok(saved);
        }
        let ox = i16::try_from(owner.x()).map_err(|_| invalid("owner column conversion"))?;
        let oy = i16::try_from(owner.y()).map_err(|_| invalid("owner row conversion"))?;
        if self.classes[sample_index(ox, oy)?] != saved {
            return Err(invalid("atlas terrain disagrees with saved cell class"));
        }
        // A one-cell-wide saved island or strait must remain visible even when
        // neighboring heights have much larger magnitudes than its own.
        for (ax, ay, bx, by) in [(ox - 1, oy, ox + 1, oy), (ox, oy - 1, ox, oy + 1)] {
            let a = self.classes[sample_index(ax, ay)?];
            let b = self.classes[sample_index(bx, by)?];
            if a == b && a != saved && a != TerrainKind::Lake {
                return Ok(saved);
            }
        }
        let mut classes = [TerrainKind::Sea; 4];
        // Four i32 heights times Q16 axis weights fit i128 at the 32K limit.
        let mut height_sum = 0_i128;
        for (i, (cx, cy, weight)) in [
            (xl, yl, i128::from(xd - xh) * i128::from(yd - yh)),
            (xl + 1, yl, i128::from(xh) * i128::from(yd - yh)),
            (xl, yl + 1, i128::from(xd - xh) * i128::from(yh)),
            (xl + 1, yl + 1, i128::from(xh) * i128::from(yh)),
        ]
        .into_iter()
        .enumerate()
        {
            let index = sample_index(cx, cy)?;
            let class = self.classes[index];
            if class == TerrainKind::Lake {
                return Ok(saved);
            }
            classes[i] = class;
            let height = i128::from(self.heights[index]);
            let directed = if class == TerrainKind::Land {
                height.max(1)
            } else {
                height.min(-1)
            };
            height_sum += directed * weight;
        }
        if classes[0] == classes[3] && classes[1] == classes[2] && classes[0] != classes[1] {
            return Ok(saved);
        }
        Ok(match height_sum.cmp(&0) {
            std::cmp::Ordering::Greater => TerrainKind::Land,
            std::cmp::Ordering::Less => TerrainKind::Sea,
            std::cmp::Ordering::Equal => saved,
        })
    }
}
