//! The opt-in world grade of world-derived tactical images (goal 49,
//! `?world_grade=1`): the compositor pulls ground and water toward the
//! world map's colours ([`arda_tactical::WorldTint`]), read from the relief
//! shader on a world-anchored lattice, so zooming from the relief levels
//! into a tactical map keeps the palette, ground tone and water colour.
//! Without the flag nothing here runs and images are unchanged.

use super::Tactical;
use crate::error::{ServerError, ServerResult};
use arda_midzoom::ReliefWorld;
use arda_tactical::compose::world_tint::DEFAULT_STRENGTH_Q12;
use arda_tactical::{TacticalLayout, WorldTint};
use std::sync::Arc;

/// Squares between world-colour samples (12.5 m): the world map's tone and
/// light at hillside scale, not its 10 m texture.
pub const TINT_STEP_SQ: u32 = 8;
/// One square in world micrometres (convention I2: 100 m / 64).
const SQUARE_UM: i64 = 1_562_500;

impl Tactical {
    /// Lets world images take the opt-in world grade from `world`'s relief
    /// shading. Without a source, `?world_grade=1` is refused.
    pub fn set_tint_source(&mut self, world: Arc<ReliefWorld>) {
        self.tint_source = Some(world);
    }

    /// Whether the world grade is available.
    #[must_use]
    pub fn has_world_grade(&self) -> bool {
        self.tint_source.is_some()
    }

    /// The world-colour lattice covering `layout` (which must carry its
    /// world origin), aligned to the global [`TINT_STEP_SQ`] grid so every
    /// render of a pixel interpolates the same points.
    ///
    /// # Errors
    /// No source, no origin, or a relief read failure.
    pub fn world_tint(&self, layout: &TacticalLayout) -> ServerResult<WorldTint> {
        let rw = self.tint_source.as_ref().ok_or_else(|| {
            ServerError::NotFound(
                "this world has no formed fine terrain for the world grade".into(),
            )
        })?;
        let origin = layout.origin.ok_or_else(|| {
            ServerError::BadRequest("the world grade needs a world-anchored layout".into())
        })?;
        let step = i64::from(TINT_STEP_SQ);
        let lo = origin.map(|o| o.div_euclid(step) * step);
        let extent = [i64::from(layout.width), i64::from(layout.height)];
        let points = |a: usize| {
            let hi = origin[a] + extent[a];
            usize::try_from((hi - lo[a]).div_euclid(step) + 2).unwrap_or(2)
        };
        let size = (points(0), points(1));
        let lattice = arda_midzoom::world_lattice(
            rw,
            (lo[0] * SQUARE_UM, lo[1] * SQUARE_UM),
            step * SQUARE_UM,
            size,
        )
        .map_err(|e| ServerError::Internal(format!("world grade: {e}")))?;
        // Rivers are lines on the world map; its own blue-teal (the light
        // channel band) stands in for them. Lakes and sea keep their colour.
        let river = [66, 142, 166];
        let water = lattice
            .colours
            .iter()
            .zip(&lattice.land)
            .map(|(&c, &land)| if land { river } else { c })
            .collect();
        Ok(WorldTint {
            origin: lo,
            step: TINT_STEP_SQ,
            size,
            ground: land_colours(&lattice.colours, &lattice.land, size),
            water,
            strength_q12: DEFAULT_STRENGTH_Q12,
        })
    }
}

/// Ground colours with world water points replaced by the mean of their
/// land neighbours (grown a few steps, then the lattice's land mean), so a
/// tactical bank the world draws as water is not tinted blue. A lattice
/// with no land at all keeps a neutral world shore sand.
fn land_colours(colours: &[[u8; 3]], land: &[bool], (w, h): (usize, usize)) -> Vec<[u8; 3]> {
    let mut out = colours.to_vec();
    let mut known = land.to_vec();
    for _ in 0..4 {
        let (prev, prev_known) = (out.clone(), known.clone());
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if prev_known[i] {
                    continue;
                }
                let (mut sum, mut n) = ([0_u32; 3], 0_u32);
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let (nx, ny) = (x.checked_add_signed(dx), y.checked_add_signed(dy));
                    if let (Some(nx), Some(ny)) = (nx, ny) {
                        if nx < w && ny < h && prev_known[ny * w + nx] {
                            for (s, c) in sum.iter_mut().zip(prev[ny * w + nx]) {
                                *s += u32::from(c);
                            }
                            n += 1;
                        }
                    }
                }
                if n > 0 {
                    out[i] = sum.map(|s| u8::try_from(s / n).unwrap_or(255));
                    known[i] = true;
                }
            }
        }
    }
    let (mut sum, mut n) = ([0_u64; 3], 0_u64);
    for (c, _) in out.iter().zip(&known).filter(|(_, k)| **k) {
        for (s, v) in sum.iter_mut().zip(c) {
            *s += u64::from(*v);
        }
        n += 1;
    }
    let fallback = if n > 0 {
        sum.map(|s| u8::try_from(s / n).unwrap_or(255))
    } else {
        [220, 201, 154]
    };
    for (c, k) in out.iter_mut().zip(&known) {
        if !k {
            *c = fallback;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::land_colours;

    #[test]
    fn water_points_take_their_land_neighbours_colour() {
        let blue = [20, 60, 120];
        let olive = [120, 130, 70];
        let colours = [olive, blue, blue, blue];
        let land = [true, false, false, false];
        let out = land_colours(&colours, &land, (2, 2));
        assert_eq!(out, vec![olive; 4]);
        let sea = land_colours(&[blue; 4], &[false; 4], (2, 2));
        assert_eq!(sea, vec![[220, 201, 154]; 4]);
    }
}
