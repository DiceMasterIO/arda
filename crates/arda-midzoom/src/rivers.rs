//! Saved rivers over relief tiles (goal 28): each channel edge is drawn
//! along the formed overview's own centreline (cell centre plus thalweg
//! and meander offsets, relaxed along main stems, then a B-spline over the
//! chain; logic/04 §atlas-formed rivers) as anti-aliased tapered capsules,
//! so the stored rivers stay exactly where the overview draws them.

use crate::fixed::{isqrt128, ONE};
use crate::pyramid::Pyramid;
use crate::tile::Rgba;
use crate::world::ReliefWorld;
use crate::MidzoomError;
use arda_core::{GlobalCell, FINE_FRAME_OFFSET_UM};
use arda_render::{
    formed_river_centreline, formed_river_rgb, formed_source_width, AtlasTerrain,
    FormedRiverNetwork,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Light-band colour for channels below the banded discharges (the
/// overview's light band).
const LIGHT: [u8; 3] = [66, 142, 166];

/// Drawn width over physical width, Q12: the overview's ×4 symbol at
/// 25 m/px easing to ×1.5 once pixels are a few metres.
fn width_gain_q12(pixel_um: i64) -> i64 {
    (pixel_um * ONE / 6_250_000).clamp(6_144, 4 * ONE)
}

fn owner(node: GlobalCell) -> (i32, i32) {
    (
        i32::try_from(node.x / 512).unwrap_or(0),
        i32::try_from(node.y / 512).unwrap_or(0),
    )
}

/// Paints every saved channel edge that reaches the window.
///
/// # Errors
/// An area failed to load.
pub fn paint_rivers(
    rw: &ReliefWorld,
    pyramid: &Pyramid,
    z: u32,
    origin: (i64, i64),
    areas: &BTreeMap<(i32, i32), Arc<AtlasTerrain>>,
    out: &mut Rgba,
) -> Result<(), MidzoomError> {
    let pixel_um = pyramid.pixel_um(z);
    let (w, h) = (i64::from(out.width), i64::from(out.height));
    let x0 = pyramid.centre_um(z, origin.0) - FINE_FRAME_OFFSET_UM;
    let y0 = pyramid.centre_um(z, origin.1) - FINE_FRAME_OFFSET_UM;
    let x1 = pyramid.centre_um(z, origin.0 + w - 1) - FINE_FRAME_OFFSET_UM;
    let y1 = pyramid.centre_um(z, origin.1 + h - 1) - FINE_FRAME_OFFSET_UM;
    let mut edges = BTreeSet::new();
    for &(ax, ay) in areas.keys() {
        for e in &rw.channel_edges(ax, ay)?.edges {
            edges.insert((
                e.from,
                e.to,
                e.from_width_dm,
                e.to_width_dm,
                e.discharge.raw(),
            ));
        }
    }
    // The overview's chain: main-stem neighbours, sources and relaxation.
    let network = FormedRiverNetwork::new(edges.iter().map(|&(f, t, _, _, q)| (f, t, q)));
    let raw = |node: GlobalCell| -> Result<(i64, i64), MidzoomError> {
        let key = owner(node);
        match areas.get(&key) {
            Some(t) => Ok(t.river_vertex_um(node)),
            None => Ok(rw.context(key.0, key.1)?.river_vertex_um(node)),
        }
    };
    let mut raw_cache: BTreeMap<GlobalCell, (i64, i64)> = BTreeMap::new();
    let mut vertex = |node: GlobalCell| -> Result<(i64, i64), MidzoomError> {
        let (up, down) = network.main_stem(node);
        for n in [Some(node), up, down].into_iter().flatten() {
            if let std::collections::btree_map::Entry::Vacant(slot) = raw_cache.entry(n) {
                slot.insert(raw(n)?);
            }
        }
        Ok(network
            .relaxed(node, |n| raw_cache.get(&n).copied())
            .unwrap_or((0, 0)))
    };
    let gain = width_gain_q12(pixel_um);
    let mut canvas = Canvas {
        cover: vec![0_i64; usize::try_from(w * h).unwrap_or(0)],
        colour: vec![LIGHT; usize::try_from(w * h).unwrap_or(0)],
        w,
        h,
    };
    for &(from, to, wf, wt, discharge) in &edges {
        let half = |dm: u32| (i64::from(dm) * 100_000 * gain / ONE / 2).max(pixel_um * 3 / 5);
        let (ha, hb) = (half(wf), half(wt));
        let a = vertex(from)?;
        let b = vertex(to)?;
        let reach = ha.max(hb) + pixel_um + 60_000_000;
        if a.0.max(b.0) + reach < x0
            || a.0.min(b.0) - reach > x1
            || a.1.max(b.1) + reach < y0
            || a.1.min(b.1) - reach > y1
        {
            continue;
        }
        let (before, after) = network.neighbours(from, to);
        let before = before.map(&mut vertex).transpose()?;
        let after = after.map(&mut vertex).transpose()?;
        let ha = if network.is_source(from) {
            formed_source_width(ha).max(pixel_um * 3 / 5)
        } else {
            ha
        };
        let rgb = formed_river_rgb(discharge).unwrap_or(LIGHT);
        let points = formed_river_centreline(before, a, b, after, SPAN_STEPS);
        let steps = i64::from(SPAN_STEPS);
        for (k, pair) in (1_i64..).zip(points.windows(2)) {
            let h0 = ha + (hb - ha) * (k - 1) / steps;
            let h1 = ha + (hb - ha) * k / steps;
            canvas.capsule(pyramid, z, origin, (pair[0], pair[1]), (h0, h1), rgb);
        }
    }
    let Canvas { cover, colour, .. } = canvas;
    for (k, (&c, rgb)) in cover.iter().zip(&colour).enumerate() {
        if c == 0 || out.pixels[k * 4 + 3] == 0 {
            continue;
        }
        for (dst, &src) in out.pixels[k * 4..k * 4 + 3].iter_mut().zip(rgb) {
            let base = i64::from(*dst);
            let v = base + (i64::from(src) - base) * c / ONE;
            *dst = u8::try_from(v.clamp(0, 255)).unwrap_or(255);
        }
    }
    Ok(())
}

/// Sub-segments per channel edge.
const SPAN_STEPS: u32 = 8;

/// Per-pixel best river coverage (Q12) and its colour.
struct Canvas {
    cover: Vec<i64>,
    colour: Vec<[u8; 3]>,
    w: i64,
    h: i64,
}

impl Canvas {
    /// Adds an anti-aliased capsule tapering from `half.0` to `half.1`.
    fn capsule(
        &mut self,
        pyramid: &Pyramid,
        z: u32,
        origin: (i64, i64),
        (a, b): ((i64, i64), (i64, i64)),
        half: (i64, i64),
        rgb: [u8; 3],
    ) {
        let pixel_um = pyramid.pixel_um(z).max(1);
        let reach = half.0.max(half.1) + pixel_um;
        let x0 = pyramid.centre_um(z, origin.0) - FINE_FRAME_OFFSET_UM;
        let y0 = pyramid.centre_um(z, origin.1) - FINE_FRAME_OFFSET_UM;
        let px = |v: i64, o: i64, n: i64| ((v - o).div_euclid(pixel_um)).clamp(-1, n);
        let cx0 = px(a.0.min(b.0) - reach, x0, self.w).max(0);
        let cx1 = px(a.0.max(b.0) + reach, x0, self.w).min(self.w - 1);
        let cy0 = px(a.1.min(b.1) - reach, y0, self.h).max(0);
        let cy1 = px(a.1.max(b.1) + reach, y0, self.h).min(self.h - 1);
        for py in cy0..=cy1 {
            let cy = pyramid.centre_um(z, origin.1 + py) - FINE_FRAME_OFFSET_UM;
            for pxl in cx0..=cx1 {
                let cx = pyramid.centre_um(z, origin.0 + pxl) - FINE_FRAME_OFFSET_UM;
                let (d, t) = distance(a, b, (cx, cy));
                let hw = half.0 + (half.1 - half.0) * t / ONE;
                let c = ((hw - d) * ONE / pixel_um + ONE / 2).clamp(0, ONE);
                let k = usize::try_from(py * self.w + pxl).unwrap_or(0);
                if c > self.cover[k] {
                    self.cover[k] = c;
                    self.colour[k] = rgb;
                }
            }
        }
    }
}

/// Distance (µm) from `p` to segment `a→b` and the clamped parameter (Q12).
fn distance(a: (i64, i64), b: (i64, i64), p: (i64, i64)) -> (i64, i64) {
    let (dx, dy) = (i128::from(b.0 - a.0), i128::from(b.1 - a.1));
    let (px, py) = (i128::from(p.0 - a.0), i128::from(p.1 - a.1));
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0 {
        0
    } else {
        ((px * dx + py * dy) * i128::from(ONE) / len2).clamp(0, i128::from(ONE))
    };
    let qx = px - dx * t / i128::from(ONE);
    let qy = py - dy * t / i128::from(ONE);
    (isqrt128(qx * qx + qy * qy), i64::try_from(t).unwrap_or(0))
}
