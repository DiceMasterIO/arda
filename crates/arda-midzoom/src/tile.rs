//! Relief tiles: refined heights shaded by the formed Atlas shader
//! (logic/17 §render). Each pixel's colour is a pure function of its
//! global position, so any window — a 256-px tile or a larger crop —
//! reproduces the same pixels and tiles join exactly.

use crate::fixed::{organic_q12, smooth, ONE};
use crate::pyramid::{Pyramid, TILE_PX};
use crate::refine::refine_nodes_for;
use crate::source::FINE_UM;
use crate::water::WindowWater;
use crate::world::{ReliefWorld, AREA_UM};
use crate::{HeightTile, MidzoomError};
use arda_core::FINE_FRAME_OFFSET_UM;
use arda_render::{formed_river_rgb, AtlasTerrain, ReliefGeometry, ReliefSurface};
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Refined-scale concavity ring, in refined nodes.
const RING_NODES: i64 = 2;
/// Weight of the refined concavity in the shader's valley/crest terms, Q12.
const RING_WEIGHT_Q12: i64 = 8_192;
/// Refined nodes kept around the pixels (spline support plus the ring).
pub(crate) const MARGIN_NODES: i64 = RING_NODES + 3;

/// A rendered RGBA window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba {
    /// Columns.
    pub width: u32,
    /// Rows.
    pub height: u32,
    /// Row-major RGBA.
    pub pixels: Vec<u8>,
}

/// Renders relief tile `(z, x, y)`.
///
/// # Errors
/// The address is not a relief tile, or a read or shading step failed.
pub fn render_tile(
    rw: &ReliefWorld,
    pyramid: &Pyramid,
    z: u32,
    x: u32,
    y: u32,
) -> Result<Rgba, MidzoomError> {
    pyramid.check(z, x, y)?;
    render_window(
        rw,
        pyramid,
        z,
        (
            i64::from(x) * i64::from(TILE_PX),
            i64::from(y) * i64::from(TILE_PX),
        ),
        (TILE_PX, TILE_PX),
    )
}

/// Renders the pixel window at `origin` of size `size` at level `z` (any
/// level; relief detail appears where the lattice resolves it).
///
/// # Errors
/// A read or shading step failed, or the window is too large.
pub fn render_window(
    rw: &ReliefWorld,
    pyramid: &Pyramid,
    z: u32,
    origin: (i64, i64),
    size: (u32, u32),
) -> Result<Rgba, MidzoomError> {
    let (w, h) = (i64::from(size.0), i64::from(size.1));
    let n = pyramid.subdivisions(z);
    let s = FINE_UM / n;
    let lattice = |p: i64| pyramid.centre_um(z, p) - FINE_FRAME_OFFSET_UM;
    let (lx0, ly0) = (lattice(origin.0), lattice(origin.1));
    let (lx1, ly1) = (lattice(origin.0 + w - 1), lattice(origin.1 + h - 1));
    let i0 = lx0.div_euclid(s) - MARGIN_NODES;
    let j0 = ly0.div_euclid(s) - MARGIN_NODES;
    let nw = usize::try_from(lx1.div_euclid(s) + MARGIN_NODES + 1 - i0)
        .map_err(|_| MidzoomError::Window("width".into()))?;
    let nh = usize::try_from(ly1.div_euclid(s) + MARGIN_NODES + 1 - j0)
        .map_err(|_| MidzoomError::Window("height".into()))?;
    // Waves shorter than four pixels would alias into hatching.
    let heights = refine_nodes_for(rw.terrain(), (i0, j0, nw, nh), n, 4 * pyramid.pixel_um(z))?;
    let (world_w, world_h) = pyramid.world_um();
    let areas = areas_touching(rw, pyramid, z, origin, (w, h))?;
    let water = WindowWater::gather(rw, pyramid, z, origin, (w, h))?;
    let pixel_um = pyramid.pixel_um(z);
    let rows: Vec<Result<Vec<u8>, MidzoomError>> = (0..h)
        .into_par_iter()
        .map(|py| {
            let wy = pyramid.centre_um(z, origin.1 + py);
            let mut row = vec![0_u8; usize::try_from(w).unwrap_or(0) * 4];
            for px in 0..w {
                let wx = pyramid.centre_um(z, origin.0 + px);
                if wx >= world_w || wy >= world_h {
                    continue;
                }
                let key = (
                    i32::try_from(wx / AREA_UM).unwrap_or(0),
                    i32::try_from(wy / AREA_UM).unwrap_or(0),
                );
                let terrain = areas
                    .get(&key)
                    .ok_or_else(|| MidzoomError::Window("pixel area missing".into()))?;
                let (rgb, _) = shade_pixel(terrain, &heights, (&water, wx, wy), pixel_um)?;
                let at = usize::try_from(px).unwrap_or(0) * 4;
                row[at..at + 3].copy_from_slice(&rgb);
                row[at + 3] = 255;
            }
            Ok(row)
        })
        .collect();
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(usize::try_from(w * h * 4).unwrap_or(0))
        .map_err(|_| MidzoomError::ResourceLimit("relief window"))?;
    for r in rows {
        pixels.extend(r?);
    }
    Ok(Rgba {
        width: size.0,
        height: size.1,
        pixels,
    })
}

/// Light-band colour for channels below the banded discharges (the
/// overview's light band).
const LIGHT: [u8; 3] = [66, 142, 166];

/// Colour of one pixel at world micrometres `(wx, wy)`: the surface the
/// water geometry decides (logic/17 §water), lit with the refined lattice,
/// then any channel over land. The flag is true on land (false on sea and
/// lake water).
pub(crate) fn shade_pixel(
    terrain: &AtlasTerrain,
    heights: &HeightTile,
    (water, wx, wy): (&WindowWater, i64, i64),
    pixel_um: i64,
) -> Result<([u8; 3], bool), MidzoomError> {
    let (lx, ly) = (wx - FINE_FRAME_OFFSET_UM, wy - FINE_FRAME_OFFSET_UM);
    let missing = || MidzoomError::Window("refined support missing".into());
    let c = heights.sample(lx, ly).ok_or_else(missing)?;
    let r = RING_NODES * heights.spacing_um();
    let mut ring = 0;
    for (dx, dy) in [(r, 0), (-r, 0), (0, r), (0, -r)] {
        ring += heights
            .sample(lx + dx, ly + dy)
            .ok_or_else(missing)?
            .height_mm;
    }
    let concavity = (ring / 4 - c.height_mm) * RING_WEIGHT_Q12 / 4_096;
    let surface = water.surface(wx, wy);
    let rgb = terrain.relief_surface_colour(
        (lx, ly),
        pixel_um,
        ReliefGeometry {
            height_mm: i32::try_from(c.height_mm).unwrap_or(0),
            gradient_q12: c.gradient_q12,
            concavity_mm: concavity,
        },
        surface,
    )?;
    if surface != ReliefSurface::Land {
        return Ok((rgb, false));
    }
    let rgb = ground_tone(rgb, lx, ly, pixel_um);
    // logic/17 §rivers: channels over land, anti-aliased over one pixel.
    let Some((cover, discharge)) = water.river(wx, wy) else {
        return Ok((rgb, true));
    };
    let river = formed_river_rgb(discharge).unwrap_or(LIGHT);
    let rgb = std::array::from_fn(|k| {
        let base = i64::from(rgb[k]);
        let v = base + (i64::from(river[k]) - base) * cover / ONE;
        u8::try_from(v.clamp(0, 255)).unwrap_or(255)
    });
    Ok((rgb, true))
}

/// Ground-cover tone at tuft and clump scale (24 m and 11 m, rotated
/// noise), ±5 %, only once pixels resolve it.
fn ground_tone(rgb: [u8; 3], lx: i64, ly: i64, pixel_um: i64) -> [u8; 3] {
    let keep = ONE - smooth(4_000_000, 9_000_000, pixel_um);
    if keep == 0 {
        return rgb;
    }
    let n = (2 * organic_q12(0x746f_6e65, 1, lx, ly, 24_000_000)
        + organic_q12(0x746f_6e65, 2, lx, ly, 11_000_000))
        / 3
        - ONE / 2;
    let k = ONE + n / 7 * keep / ONE;
    rgb.map(|c| u8::try_from((i64::from(c) * k / ONE).clamp(0, 255)).unwrap_or(255))
}

/// Atlas contexts of every area the window touches.
pub(crate) fn areas_touching(
    rw: &ReliefWorld,
    pyramid: &Pyramid,
    z: u32,
    origin: (i64, i64),
    (w, h): (i64, i64),
) -> Result<BTreeMap<(i32, i32), Arc<AtlasTerrain>>, MidzoomError> {
    let (aw, ah) = rw.areas();
    let span = |a: i64, b: i64, max: i32| -> (i32, i32) {
        let lo = i32::try_from(a.max(0) / AREA_UM).unwrap_or(0);
        let hi = i32::try_from(b.max(0) / AREA_UM).unwrap_or(0);
        (lo.min(max - 1), hi.min(max - 1))
    };
    let (ax0, ax1) = span(
        pyramid.centre_um(z, origin.0),
        pyramid.centre_um(z, origin.0 + w - 1),
        aw,
    );
    let (ay0, ay1) = span(
        pyramid.centre_um(z, origin.1),
        pyramid.centre_um(z, origin.1 + h - 1),
        ah,
    );
    let mut out = BTreeMap::new();
    for ay in ay0..=ay1 {
        for ax in ax0..=ax1 {
            out.insert((ax, ay), rw.context(ax, ay)?);
        }
    }
    Ok(out)
}
