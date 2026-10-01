//! Soft snow cover (goal 49): lying snow is not a ground type with a
//! painted border but a cover over the ground beneath it. Near a snow
//! border the snow squares give a smooth, widely warped weight; rotated
//! noise breaks it into irregular patches with a translucent dusting at the
//! fringe, and the snow texture is laid over the ground the other keys
//! paint there, so grass and rock show through thin cover. Every term is a
//! function of the world position (and squares within [`REACH`]), so
//! windows and blocks keep joining pixel for pixel.
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use super::field::{centred, corners_world, smooth, Fields, Frame, WIDE_STEP};
use crate::layout::TacticalLayout;

/// The ground key snow covers with.
pub const SNOW: &str = "snow";
/// Squares around a border the soft cover reads: warp plus the bilinear
/// support stay inside it.
const REACH: i64 = 4;
/// Border warp amplitude and feature size, squares.
const WARP: f32 = 1.5;
const WARP_SCALE: f32 = 3.2;
/// Radius of the cross kernel that widens the snow weight, squares.
const SPREAD: f32 = 0.75;

/// The soft cover of one layout's snow.
pub struct Snow {
    /// The snow key's index among the layout's ground keys.
    pub key: usize,
    /// Per square: snow and other ground both lie within [`REACH`].
    soft: Vec<bool>,
    /// Per square: the commonest other key within [`REACH`].
    under: Vec<usize>,
    /// Warp x, warp y and patch noise, world-anchored.
    fields: Fields<3>,
    seed: u64,
}

impl Snow {
    /// The cover of `layout`, whose squares use keys `grid`; `None` when
    /// `key` (the snow key) is absent.
    #[must_use]
    pub fn new(
        layout: &TacticalLayout,
        grid: &[usize],
        key: Option<usize>,
        frame: &Frame,
        seed: u64,
    ) -> Option<Self> {
        let key = key?;
        let (w, h) = (i64::from(layout.width), i64::from(layout.height));
        let at = |x: i64, y: i64| grid[usize::try_from(y * w + x).unwrap_or(0)];
        let mut soft = vec![false; grid.len()];
        let mut under = vec![key; grid.len()];
        for y in 0..h {
            for x in 0..w {
                let mut counts: Vec<(usize, u32)> = Vec::new();
                let mut snow = false;
                for sy in (y - REACH).max(0)..=(y + REACH).min(h - 1) {
                    for sx in (x - REACH).max(0)..=(x + REACH).min(w - 1) {
                        let k = at(sx, sy);
                        if k == key {
                            snow = true;
                        } else if let Some(e) = counts.iter_mut().find(|e| e.0 == k) {
                            e.1 += 1;
                        } else {
                            counts.push((k, 1));
                        }
                    }
                }
                let i = usize::try_from(y * w + x).unwrap_or(0);
                // Ties go to the lower key index: first appearance.
                if let Some(&(k, _)) = counts.iter().max_by_key(|e| (e.1, usize::MAX - e.0)) {
                    soft[i] = snow;
                    under[i] = k;
                }
            }
        }
        let (cw, ch) = (layout.width * frame.ppsq, layout.height * frame.ppsq);
        let c = |salt: u64, u: f32, v: f32, s: f32| centred(seed ^ salt, u / s, v / s, 3);
        let fields = Fields::new(frame, cw, ch, WIDE_STEP / 2, |u, v| {
            [
                c(0x5_A0E1, u, v, WARP_SCALE),
                c(0x5_A0E2, u, v, WARP_SCALE),
                c(0x5_A0E3, u, v, 1.3),
            ]
        });
        Some(Self {
            key,
            soft,
            under,
            fields,
            seed,
        })
    }

    /// Whether square `i` takes the soft cover, and the ground under it.
    #[must_use]
    pub fn soft_at(&self, i: usize) -> Option<usize> {
        self.soft
            .get(i)
            .copied()
            .unwrap_or(false)
            .then(|| self.under[i])
    }

    /// Snow opacity at pixel `(px, py)` of a soft square, `[0, 1]`: one
    /// deep inside the snow, zero clear of it, ragged and partial between.
    #[must_use]
    // Pixel coordinates of a bounded canvas.
    #[allow(clippy::cast_precision_loss)]
    pub fn alpha(
        &self,
        layout: &TacticalLayout,
        grid: &[usize],
        frame: &Frame,
        px: u32,
        py: u32,
    ) -> f32 {
        let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
        let f = self.fields.at(fx, fy);
        let (u, v) = frame.world_sq(fx, fy);
        let (cu, cv) = (u + WARP * f[0], v + WARP * f[1]);
        let mut ws = 0.0;
        for (dx, dy, k) in [
            (0.0, 0.0, 0.4),
            (SPREAD, 0.0, 0.15),
            (-SPREAD, 0.0, 0.15),
            (0.0, SPREAD, 0.15),
            (0.0, -SPREAD, 0.15),
        ] {
            for (x, y, wt) in corners_world(layout, frame.origin, cu + dx, cv + dy) {
                let i = usize::try_from(y * i64::from(layout.width) + x).unwrap_or(0);
                if grid.get(i) == Some(&self.key) {
                    ws += k * wt;
                }
            }
        }
        let ws: f32 = ws.clamp(0.0, 1.0);
        // Patch noise acts only inside the border band, so solid snow
        // stays solid and clear ground stays clear.
        let fine = centred(self.seed ^ 0x5_A0E4, u / 0.42, v / 0.42, 2);
        let band = 4.0 * ws * (1.0 - ws);
        let t = ws + band * (0.34 * f[2] + 0.14 * fine);
        let core = smooth(((t - 0.3) / 0.42).clamp(0.0, 1.0));
        // A dusting of thin, speckled cover ahead of the patch edge.
        let dust = centred(self.seed ^ 0x5_A0E5, u / 0.2, v / 0.2, 2);
        let fringe = 0.5 * smooth(((t - 0.06) / 0.26).clamp(0.0, 1.0)) * (0.5 + 0.5 * dust);
        core.max(fringe).clamp(0.0, 1.0)
    }

    /// Thin snow reads grey-blue (slush and shadowed crust); full cover
    /// keeps the texture's own white. `a` is [`Self::alpha`].
    #[must_use]
    pub fn tint(white: [f32; 3], a: f32) -> [f32; 3] {
        const THIN: [f32; 3] = [0.8, 0.85, 0.94];
        let k = 0.6 * (1.0 - a);
        std::array::from_fn(|i| white[i] * (1.0 - k + k * THIN[i]))
    }
}
