//! Per-square rules derived from the layout, its placements and the sidecar.

use crate::error::SceneError;
use crate::rle::Grid;
use crate::sidecar::RulesSidecar;
use crate::types::{BlockerKind, CoverLevel, Movement, Obscurement, VisionBlocker};
use arda_tactical::catalog::{Asset, Layer};
use arda_tactical::compose::resolve;
use arda_tactical::layout::TacticalLayout;
use arda_tactical::noise::hash_str;
use arda_tactical::Library;

/// SRD 5.1: water this deep or deeper is swum, not waded.
pub const SWIM_DEPTH_FT: u8 = 5;
/// An elevation step this high or higher between neighbours needs climbing.
pub const CLIMB_STEP_FT: i32 = 10;

/// Neighbour offsets in climb-mask bit order: N, NE, E, SE, S, SW, W, NW.
pub const DIRS: [(i64, i64); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

/// A placement resolved to its asset and its rotated footprint.
#[derive(Debug, Clone)]
pub struct Placed<'a> {
    /// The resolved asset.
    pub asset: &'a Asset,
    /// Footprint rectangle in squares: `[x0, y0, x1, y1]`.
    pub rect: [f32; 4],
    /// Anchor position in squares.
    pub at: (f32, f32),
}

/// The seed the compositor resolves queries with, so the scene names the
/// same assets the painted image shows (`compose::render`).
#[must_use]
pub fn art_seed(lib: &Library, seed: u64) -> u64 {
    seed ^ hash_str(0, &lib.catalog.library_version)
}

/// Resolves every placement in layout order.
///
/// # Errors
/// A placement that matches no asset.
pub fn resolve_all<'a>(
    layout: &TacticalLayout,
    lib: &'a Library,
    seed: u64,
) -> Result<Vec<Placed<'a>>, SceneError> {
    let s = art_seed(lib, seed);
    let mut out = Vec::with_capacity(layout.placements.len());
    for (i, p) in layout.placements.iter().enumerate() {
        let a = resolve(lib, &p.asset, s, i).ok_or_else(|| {
            SceneError::Tactical(arda_tactical::TacticalError::Layout {
                layout: layout.name.clone(),
                message: format!("placement {i} matches no asset"),
            })
        })?;
        let turns = (p.rotation / 90) % 4;
        let anchor = a.anchor_or_centre();
        let (mut ax, mut ay) = (anchor.x, anchor.y);
        #[allow(clippy::cast_precision_loss)] // footprints are a few squares
        let (mut w, mut h) = (a.footprint.w as f32, a.footprint.h as f32);
        if p.mirror {
            ax = w - ax;
        }
        for _ in 0..turns {
            // Same turn as `compose::anchor_origin`: (x, y) -> (h - y, x).
            (ax, ay) = (h - ay, ax);
            (w, h) = (h, w);
        }
        let (x0, y0) = (p.x - ax, p.y - ay);
        out.push(Placed {
            asset: a,
            rect: [x0, y0, x0 + w, y0 + h],
            at: (p.x, p.y),
        });
    }
    Ok(out)
}

/// Squares whose centre lies inside `rect`, clamped to the map.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn covered(rect: [f32; 4], width: u32, height: u32) -> impl Iterator<Item = (u32, u32)> {
    // Centre c = i + 0.5 lies in [x0, x1) iff i in [ceil(x0 - 0.5), ceil(x1 - 0.5)).
    let lo = |v: f32, max: u32| (v - 0.5).ceil().clamp(0.0, max as f32) as u32;
    let (x0, x1) = (lo(rect[0], width), lo(rect[2], width));
    let (y0, y1) = (lo(rect[1], height), lo(rect[3], height));
    (y0..y1).flat_map(move |y| (x0..x1).map(move |x| (x, y)))
}

/// The per-square layers of a scene.
#[derive(Debug, Clone)]
pub struct Layers {
    /// Movement.
    pub movement: Grid<Movement>,
    /// Climb masks.
    pub climb: Grid<u8>,
    /// Cover.
    pub cover: Grid<CoverLevel>,
    /// Obscurement.
    pub obscured: Grid<Obscurement>,
    /// Elevation in feet.
    pub elevation_ft: Grid<i16>,
    /// Water depth in feet.
    pub water_depth_ft: Grid<u8>,
    /// Squares a floor asset (dock, bridge) spans; they are walked, not waded.
    pub bridged: Vec<bool>,
}

/// Derives every per-square layer.
///
/// # Errors
/// [`SceneError::Schema`] when the layout's square list is not
/// `width × height` long, or the sidecar does not match its size (review
/// round 2 #39: this public entry point indexed without
/// `TacticalLayout::check`).
pub fn derive(
    layout: &TacticalLayout,
    placed: &[Placed<'_>],
    rules: Option<&RulesSidecar>,
) -> Result<Layers, SceneError> {
    let (w, h) = (layout.width, layout.height);
    let n = w as usize * h as usize;
    if layout.squares.len() != n {
        return Err(SceneError::Schema(format!(
            "a {w}x{h} layout lists {} squares",
            layout.squares.len()
        )));
    }
    if let Some(r) = rules {
        r.check(w, h)?;
    }
    let idx = |x: u32, y: u32| y as usize * w as usize + x as usize;
    let mut blocked = vec![false; n];
    let mut difficult = vec![false; n];
    let mut bridged = vec![false; n];
    let mut cover = vec![CoverLevel::None; n];
    let mut obscured = vec![Obscurement::Clear; n];
    // Squares under a canopy: a sidecar that clears their heavy obscurement
    // leaves the foliage's light obscurement (review round 2 #39).
    let mut foliage = vec![false; n];
    for p in placed {
        let a = p.asset;
        let sight = if a.blocks_sight {
            Obscurement::Heavy
        } else {
            Obscurement::Light
        };
        for (x, y) in covered(p.rect, w, h) {
            let i = idx(x, y);
            difficult[i] |= a.difficult_terrain;
            match a.layer {
                // A canopy overhangs: it obscures its whole footprint but
                // only the trunk square stops movement or grants cover.
                Layer::Canopy => {
                    obscured[i] = obscured[i].max(sight);
                    foliage[i] = true;
                }
                Layer::Floor => bridged[i] = true,
                _ => {
                    blocked[i] |= a.blocks_movement;
                    cover[i] = cover[i].max(a.cover.into());
                    if a.blocks_sight {
                        obscured[i] = Obscurement::Heavy;
                    }
                }
            }
        }
        if a.layer == Layer::Canopy {
            if let Some(i) = trunk(p.at, w, h).map(|(x, y)| idx(x, y)) {
                blocked[i] |= a.blocks_movement;
                cover[i] = cover[i].max(a.cover.into());
            }
        }
    }
    let mut water: Vec<u8> = layout.squares.iter().map(|s| s.water_depth_ft).collect();
    if let Some(r) = rules {
        for (i, c) in r.squares.iter().enumerate().take(n) {
            if let Some(d) = c.difficult {
                difficult[i] = d;
            }
            if let Some(d) = c.water_depth_ft {
                water[i] = d;
            }
            if let Some(cv) = c.cover {
                cover[i] = cover[i].max(cv);
            }
            match c.blocks_sight {
                Some(true) => obscured[i] = Obscurement::Heavy,
                Some(false) if obscured[i] == Obscurement::Heavy => {
                    obscured[i] = if foliage[i] {
                        Obscurement::Light
                    } else {
                        Obscurement::Clear
                    };
                }
                _ => {}
            }
            if c.lightly_obscured == Some(true) && obscured[i] == Obscurement::Clear {
                obscured[i] = Obscurement::Light;
            }
            if let Some(b) = c.blocks_movement {
                blocked[i] = b;
            }
            if let Some(d) = c.deck {
                bridged[i] = d;
            }
        }
    }
    let movement = (0..n)
        .map(|i| {
            if blocked[i] {
                Movement::Impassable
            } else if !bridged[i] && water[i] >= SWIM_DEPTH_FT {
                Movement::Swim
            } else if !bridged[i] && water[i] > 0 {
                Movement::Wade
            } else if difficult[i] {
                Movement::Difficult
            } else {
                Movement::Normal
            }
        })
        .collect();
    let elevation: Vec<i16> = layout.squares.iter().map(|s| s.elevation_ft).collect();
    Ok(Layers {
        movement: Grid(movement),
        climb: Grid(climb_masks(&elevation, w, h)),
        cover: Grid(cover),
        obscured: Grid(obscured),
        elevation_ft: Grid(elevation),
        water_depth_ft: Grid(water),
        bridged,
    })
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn trunk(at: (f32, f32), w: u32, h: u32) -> Option<(u32, u32)> {
    let (x, y) = (at.0.floor(), at.1.floor());
    #[allow(clippy::cast_precision_loss)]
    let inside = x >= 0.0 && y >= 0.0 && x < w as f32 && y < h as f32;
    inside.then_some((x as u32, y as u32))
}

fn climb_masks(elev: &[i16], w: u32, h: u32) -> Vec<u8> {
    let (wi, hi) = (i64::from(w), i64::from(h));
    let at = |x: i64, y: i64| elev.get(usize::try_from(y * wi + x).unwrap_or(usize::MAX));
    let mut out = vec![0u8; elev.len()];
    for y in 0..hi {
        for x in 0..wi {
            let Some(&e) = at(x, y) else { continue };
            let mut m = 0u8;
            for (bit, (dx, dy)) in DIRS.iter().enumerate() {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= wi || ny >= hi {
                    continue;
                }
                if let Some(&ne) = at(nx, ny) {
                    if (i32::from(ne) - i32::from(e)).abs() >= CLIMB_STEP_FT {
                        m |= 1 << bit;
                    }
                }
            }
            if let Some(slot) = out.get_mut(usize::try_from(y * wi + x).unwrap_or(usize::MAX)) {
                *slot = m;
            }
        }
    }
    out
}

/// Canopy and sight-blocking prop polygons, in placement order.
#[must_use]
pub fn vision_blockers(placed: &[Placed<'_>]) -> Vec<VisionBlocker> {
    placed
        .iter()
        .filter_map(|p| {
            let a = p.asset;
            let [x0, y0, x1, y1] = p.rect;
            let obscurement = if a.blocks_sight {
                Obscurement::Heavy
            } else {
                Obscurement::Light
            };
            if a.layer == Layer::Canopy {
                // An octagon inscribed in the footprint approximates the crown.
                let (cx, cy) = ((x1 - x0) * 0.29, (y1 - y0) * 0.29);
                let polygon = vec![
                    [x0 + cx, y0],
                    [x1 - cx, y0],
                    [x1, y0 + cy],
                    [x1, y1 - cy],
                    [x1 - cx, y1],
                    [x0 + cx, y1],
                    [x0, y1 - cy],
                    [x0, y0 + cy],
                ];
                Some(VisionBlocker {
                    kind: BlockerKind::Canopy,
                    asset: a.id.clone(),
                    obscurement,
                    polygon,
                })
            } else if a.blocks_sight {
                Some(VisionBlocker {
                    kind: BlockerKind::Prop,
                    asset: a.id.clone(),
                    obscurement,
                    polygon: vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]],
                })
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footprints_cover_squares_by_centre() {
        let v: Vec<_> = covered([1.0, 1.0, 3.0, 2.0], 10, 10).collect();
        assert_eq!(v, vec![(1, 1), (2, 1)]);
        let v: Vec<_> = covered([-1.0, -1.0, 0.6, 0.6], 10, 10).collect();
        assert_eq!(v, vec![(0, 0)]);
    }

    #[test]
    fn climb_masks_flag_ten_foot_steps_both_ways() {
        let m = climb_masks(&[0, 10, 5], 3, 1);
        assert_eq!(m[0], 1 << 2);
        assert_eq!(m[1], 1 << 6);
        assert_eq!(m[2], 0);
    }
}
