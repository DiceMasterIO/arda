//! The water layer and its shoreline (goal 62, vocabulary I15).
//!
//! Water covers squares with `water_depth_ft > 0`. Its edge uses the ground
//! warp at a lower amplitude with a 1.5-pixel anti-aliased threshold, so the
//! bank line is clean. Inside, depth is interpolated on a wider warp:
//! - shallow texture below 5 ft, deep at 5 ft or more (the SRD wade/swim
//!   line), with a meandering margin between them;
//! - deeper water darkens further, down to 12 ft;
//! - near the bank the water lightens into sunlit shallows and a broken
//!   line of foam laps the edge.
//!
//! On the land side a narrow band of wet soil darkens the bank.
// Pixel, grid and cell indices are bounded by the canvas size (at most
// 1024 ppsq × a few hundred squares), so these conversions cannot lose
// meaningful range.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]

use super::field::{centred, corners_world, Fields, Frame, STEP};
use super::sample::{sample, CellTable, TextureSet};
use crate::layout::TacticalLayout;
use crate::noise::{fbm, hash_str};
use crate::raster::Rgba;
use rayon::prelude::*;

/// Water-edge warp amplitude in squares.
const EDGE_WARP: f32 = 0.3;
/// Depth warp amplitude in squares.
const DEPTH_WARP: f32 = 0.6;
/// Depth at which deep water takes over, in feet.
const DEEP_FT: f32 = 5.0;
/// Width of the wet-soil band on land, in squares.
const WET_BAND: f32 = 0.2;

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Paints the water layer and returns its coverage mask (0–255 per pixel).
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn paint_water(
    canvas: &mut Rgba,
    layout: &TacticalLayout,
    textures: &TextureSet,
    seed: u64,
    ppsq: u32,
) -> Vec<u8> {
    let (w, h) = (canvas.width as usize, canvas.height as usize);
    let mut mask = vec![0u8; w * h];
    if layout.squares.iter().all(|s| s.water_depth_ft == 0) {
        return mask;
    }
    let (Some(shallow), Some(deep)) = (
        textures.get(super::WATER_SHALLOW),
        textures.get(super::WATER_DEEP),
    ) else {
        return mask;
    };
    let frame = Frame::of(layout, ppsq);
    let (lw, lh) = (i64::from(layout.width), i64::from(layout.height));
    // Squares within reach of water; everything else is skipped.
    let wet_near: Vec<bool> = (0..lh)
        .flat_map(|y| (0..lw).map(move |x| (x, y)))
        .map(|(x, y)| {
            (-1..=1).any(|dy| (-1..=1).any(|dx| layout.square(x + dx, y + dy).water_depth_ft > 0))
        })
        .collect();
    let c = |salt: u64, u: f32, v: f32, scale: f32| centred(seed ^ salt, u / scale, v / scale, 3);
    // Edge warp x, y, depth warp x, y, depth roughness, cell warp x, y.
    let fields = Fields::new(&frame, canvas.width, canvas.height, STEP, |u, v| {
        [
            c(0xA7E4 ^ 0x3A, u, v, 1.6),
            c(0xA7E4 ^ 0x3B, u, v, 1.6),
            c(0xDE9 ^ 0x3A, u, v, 1.6),
            c(0xDE9 ^ 0x3B, u, v, 1.6),
            c(0xDEA, u, v, 0.6),
            c(0xCE3, u, v, 1.2),
            c(0xCE4, u, v, 1.2),
        ]
    });
    let s_cells = CellTable::new(
        shallow,
        hash_str(seed, super::WATER_SHALLOW),
        &frame,
        layout.width,
        layout.height,
    );
    let d_cells = CellTable::new(
        deep,
        hash_str(seed, super::WATER_DEEP),
        &frame,
        layout.width,
        layout.height,
    );
    let s = ppsq as f32;
    let foam_seed = seed ^ 0xF0A3;
    canvas
        .data
        .par_chunks_mut(w * 4)
        .zip(mask.par_chunks_mut(w))
        .enumerate()
        .for_each(|(py, (row, mrow))| {
            let py32 = u32::try_from(py).unwrap_or(0);
            #[allow(clippy::needless_range_loop)] // px also indexes the colour row
            for px in 0..w {
                let px32 = u32::try_from(px).unwrap_or(0);
                let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
                let (u, v) = (fx / s, fy / s);
                let own = super::field::own_square(layout, u, v);
                if !wet_near[usize::try_from(own.1 * lw + own.0).unwrap_or(0)] {
                    continue;
                }
                let fl = fields.at(fx, fy);
                let origin = frame.origin;
                let (uw, vw) = frame.world_sq(fx, fy);
                let (ex, ey) = (uw + EDGE_WARP * fl[0], vw + EDGE_WARP * fl[1]);
                let wet: f32 = corners_world(layout, origin, ex, ey)
                    .iter()
                    .filter(|c| layout.square(c.0, c.1).water_depth_ft > 0)
                    .map(|c| c.2)
                    .sum();
                let o = px * 4;
                // Signed distance to the bank in pixels (positive in water).
                let dist = (wet - 0.5) * s;
                let alpha = (dist / 1.5 + 0.5).clamp(0.0, 1.0);
                if alpha <= 0.0 {
                    if wet > 0.5 - WET_BAND {
                        let k = 1.0 - 0.3 * smooth((wet - (0.5 - WET_BAND)) / WET_BAND);
                        for c in &mut row[o..o + 3] {
                            *c = super::field::byte(f32::from(*c) * k);
                        }
                    }
                    continue;
                }
                let (dx, dy) = (uw + DEPTH_WARP * fl[2], vw + DEPTH_WARP * fl[3]);
                let d: f32 = corners_world(layout, origin, dx, dy)
                    .iter()
                    .map(|c| c.2 * f32::from(layout.square(c.0, c.1).water_depth_ft))
                    .sum();
                let t = smooth((d + 0.8 * fl[4] - (DEEP_FT - 1.3)) / 1.3);
                let cw = (0.35 * fl[5], 0.35 * fl[6]);
                let a = sample(shallow, &s_cells, &frame, px32, py32, cw);
                let b = sample(deep, &d_cells, &frame, px32, py32, cw);
                let mut c = [0.0f32; 3];
                for i in 0..3 {
                    c[i] = a[i] + (b[i] - a[i]) * t;
                }
                let darker = 1.0 - 0.045 * (d - 6.0).clamp(0.0, 6.0);
                // Sunlit shallows along the bank.
                let near = 1.0 - smooth(dist / (0.35 * s));
                let lift = [140.0, 162.0, 140.0];
                for i in 0..3 {
                    c[i] *= darker;
                    c[i] += (lift[i] - c[i]) * 0.3 * near;
                }
                // Broken foam lapping the edge.
                if dist < 9.0 {
                    let (gx, gy) = frame.world_sq(fx, fy);
                    let n = fbm(foam_seed, gx * 9.0, gy * 9.0, 2, None);
                    let band = (1.0 - (dist - 3.0).abs() / 3.0).clamp(0.0, 1.0);
                    let f = band * smooth((n - 0.38) * 5.0) * 0.8;
                    for (i, fc) in [228.0, 238.0, 232.0].iter().enumerate() {
                        c[i] += (fc - c[i]) * f;
                    }
                }
                let ab = super::field::byte(alpha * 255.0);
                let px_c = [
                    super::field::byte(c[0]),
                    super::field::byte(c[1]),
                    super::field::byte(c[2]),
                ];
                blend(&mut row[o..o + 4], px_c, ab);
                mrow[px] = ab;
            }
        });
    mask
}

/// Source-over blend of an opaque colour at alpha `a` onto an opaque pixel.
fn blend(dst: &mut [u8], c: [u8; 3], a: u8) {
    let a = u32::from(a);
    for i in 0..3 {
        let v = (u32::from(c[i]) * a + u32::from(dst[i]) * (255 - a) + 127) / 255;
        dst[i] = crate::raster::to_u8(v);
    }
    dst[3] = 255;
}
