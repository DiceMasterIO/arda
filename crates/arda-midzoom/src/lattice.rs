//! World-map colours on a world-anchored lattice (goal 49): the relief
//! shader's colour at the points `origin + (i, j) · step`, so a tactical
//! render can be tinted toward the world map at its own location. Like
//! relief tiles, every value is a pure function of the global position (the
//! window only bounds the work), so lattices of neighbouring renders agree
//! wherever they overlap.

use crate::refine::refine_nodes_for;
use crate::source::FINE_UM;
use crate::tile::{shade_pixel, MARGIN_NODES};
use crate::water::WindowWater;
use crate::world::{ReliefWorld, AREA_UM};
use crate::MidzoomError;
use arda_core::FINE_FRAME_OFFSET_UM;
use std::collections::BTreeMap;

/// Largest lattice side, points.
pub const MAX_LATTICE_SIDE: usize = 512;

/// World-map colours of a lattice, row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lattice {
    /// Points across and down.
    pub size: (usize, usize),
    /// Colour at each point.
    pub colours: Vec<[u8; 3]>,
    /// True on land, false on sea and lake water.
    pub land: Vec<bool>,
}

/// Colours the `size` lattice whose first point is `origin` (world
/// micrometres, north-west origin) with spacing `step_um`. Points past the
/// world's far edges take the colour of the last point inside it.
///
/// # Errors
/// An empty, oversized or too fine lattice, or a read or shading failure.
pub fn world_lattice(
    rw: &ReliefWorld,
    origin: (i64, i64),
    step_um: i64,
    size: (usize, usize),
) -> Result<Lattice, MidzoomError> {
    let (w, h) = size;
    if w == 0 || h == 0 || w > MAX_LATTICE_SIDE || h > MAX_LATTICE_SIDE || step_um < 1_000_000 {
        return Err(MidzoomError::Window(format!(
            "{w}x{h} lattice at {step_um} um"
        )));
    }
    let (aw, ah) = rw.areas();
    let (world_w, world_h) = (i64::from(aw) * AREA_UM, i64::from(ah) * AREA_UM);
    let span = |i: usize| i64::try_from(i).unwrap_or(0) * step_um;
    let at = |o: i64, i: usize, max: i64| (o + span(i)).clamp(0, max - 1);
    // About 10 m relief (39.0625 m / 4), with waves shorter than four
    // lattice steps filtered out.
    let n = 4;
    let s = FINE_UM / n;
    let (lx0, ly0) = (
        at(origin.0, 0, world_w) - FINE_FRAME_OFFSET_UM,
        at(origin.1, 0, world_h) - FINE_FRAME_OFFSET_UM,
    );
    let (lx1, ly1) = (
        at(origin.0, w - 1, world_w) - FINE_FRAME_OFFSET_UM,
        at(origin.1, h - 1, world_h) - FINE_FRAME_OFFSET_UM,
    );
    let i0 = lx0.div_euclid(s) - MARGIN_NODES;
    let j0 = ly0.div_euclid(s) - MARGIN_NODES;
    let nw = usize::try_from(lx1.div_euclid(s) + MARGIN_NODES + 1 - i0)
        .map_err(|_| MidzoomError::Window("lattice width".into()))?;
    let nh = usize::try_from(ly1.div_euclid(s) + MARGIN_NODES + 1 - j0)
        .map_err(|_| MidzoomError::Window("lattice height".into()))?;
    let heights = refine_nodes_for(rw.terrain(), (i0, j0, nw, nh), n, 4 * step_um)?;
    // The water geometry relief tiles draw with (logic/17 §water), so the
    // grade's land and water match the tactical map's.
    let water = WindowWater::gather_um(
        rw,
        (
            at(origin.0, 0, world_w),
            at(origin.1, 0, world_h),
            at(origin.0, w - 1, world_w),
            at(origin.1, h - 1, world_h),
        ),
        step_um,
    )?;
    let mut contexts = BTreeMap::new();
    let mut colours = Vec::with_capacity(w * h);
    let mut land = Vec::with_capacity(w * h);
    for j in 0..h {
        let wy = at(origin.1, j, world_h);
        for i in 0..w {
            let wx = at(origin.0, i, world_w);
            let key = (
                i32::try_from(wx / AREA_UM).unwrap_or(0),
                i32::try_from(wy / AREA_UM).unwrap_or(0),
            );
            let terrain = match contexts.get(&key) {
                Some(t) => std::sync::Arc::clone(t),
                None => {
                    let t = rw.context(key.0, key.1)?;
                    contexts.insert(key, std::sync::Arc::clone(&t));
                    t
                }
            };
            let (rgb, dry) = shade_pixel(&terrain, &heights, (&water, wx, wy), step_um)?;
            colours.push(rgb);
            land.push(dry);
        }
    }
    Ok(Lattice {
        size,
        colours,
        land,
    })
}
