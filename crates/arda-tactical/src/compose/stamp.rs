//! Sprite stamping (goal 62): every prop, wall piece and canopy drawn onto
//! the canvas and its lighting buffers, in draw order.
//!
//! The canvas is cut into bands of rows and the bands are stamped in
//! parallel. Each band replays every sprite in draw order, clipped to its
//! own rows, so each pixel sees exactly the sequence of writes a serial
//! pass would make: the output is byte-identical to stamping one sprite
//! after the other. Canopy silhouettes land `off` pixels below and right of
//! their source; a band writes the silhouette rows it owns, reading the
//! source rows above it, and the write is a maximum, which is
//! order-independent.

use super::lighting::{Buffers, Casts};
use super::terrain::{self, Terrain};
use crate::raster::{blend_px, to_u8, Rgba};
use rayon::prelude::*;
use std::sync::Arc;

/// Rows per band.
const BAND: usize = 16;

/// What a stamped sprite contributes to the lighting buffers.
#[derive(Clone, Copy)]
pub struct Stamp {
    /// Height above the ground in 1/256 ft, if it sweeps a shadow.
    pub height: Option<i32>,
    /// Whether it darkens its surroundings (ambient occlusion).
    pub occludes: bool,
    /// Offset of a canopy's cast silhouette in pixels, if it is a canopy.
    pub crown: Option<i64>,
}

/// One sprite to stamp: its image, top-left pixel and contribution.
pub struct Op {
    /// The scaled, turned sprite.
    pub sprite: Arc<Rgba>,
    /// Top-left pixel column.
    pub ox: i64,
    /// Top-left pixel row.
    pub oy: i64,
    /// What it adds to the lighting buffers.
    pub stamp: Stamp,
}

/// Mutable rows `y0..y0 + rows` of every buffer a stamp writes.
struct Band<'a> {
    y0: usize,
    img: &'a mut [u8],
    water: &'a mut [u8],
    occluders: &'a mut [u8],
    heights: &'a mut [i32],
    tops: &'a mut [u8],
    silhouettes: &'a mut [u8],
}

/// Stamps `ops` in order onto `img` and the buffers, in parallel bands.
pub fn stamp_all(
    img: &mut Rgba,
    buf: &mut Buffers,
    casts: &mut Casts,
    terrain: &Terrain<'_>,
    ops: &[Op],
) {
    let w = img.width as usize;
    if w == 0 || ops.is_empty() {
        return;
    }
    let n = BAND * w;
    let mut bands: Vec<Band<'_>> = img
        .data
        .chunks_mut(n * 4)
        .zip(buf.water.chunks_mut(n))
        .zip(buf.occluders.chunks_mut(n))
        .zip(buf.height_map.chunks_mut(n))
        .zip(casts.tops.chunks_mut(n))
        .zip(casts.silhouettes.chunks_mut(n))
        .enumerate()
        .map(
            |(b, (((((img, water), occluders), heights), tops), silhouettes))| Band {
                y0: b * BAND,
                img,
                water,
                occluders,
                heights,
                tops,
                silhouettes,
            },
        )
        .collect();
    let (cw, ch) = (i64::from(img.width), i64::from(img.height));
    bands
        .par_iter_mut()
        .for_each(|band| band.stamp(ops, terrain, cw, ch));
}

impl Band<'_> {
    fn rows(&self, w: i64) -> i64 {
        i64::try_from(self.water.len()).unwrap_or(0) / w.max(1)
    }

    fn stamp(&mut self, ops: &[Op], terrain: &Terrain<'_>, cw: i64, ch: i64) {
        let y0 = i64::try_from(self.y0).unwrap_or(0);
        let y1 = y0 + self.rows(cw);
        for op in ops {
            let sh = i64::from(op.sprite.height);
            let (lo, hi) = (op.oy.max(y0), (op.oy + sh).min(y1));
            for y in lo..hi {
                self.row(op, y, y0, terrain, cw);
            }
            if let Some(off) = op.stamp.crown {
                // Silhouette rows y here come from source rows y - off.
                let (lo, hi) = ((op.oy + off).max(y0), (op.oy + off + sh).min(y1));
                for y in lo..hi {
                    self.silhouette_row(op, y - off, off, y0, cw, ch);
                }
            }
        }
    }

    /// Source row `y` (canvas coordinates, inside this band) of `op`.
    fn row(&mut self, op: &Op, y: i64, y0: i64, terrain: &Terrain<'_>, cw: i64) {
        let s = op.stamp;
        let sy = u32::try_from(y - op.oy).unwrap_or(0);
        let local = y - y0;
        for sx in 0..op.sprite.width {
            let x = op.ox + i64::from(sx);
            if x < 0 || x >= cw {
                continue;
            }
            let px = op.sprite.get(sx, sy);
            if px[3] == 0 {
                continue;
            }
            let i = usize::try_from(local * cw + x).unwrap_or(0);
            blend_px(&mut self.img[i * 4..i * 4 + 4], px);
            let a = u32::from(px[3]);
            self.water[i] = to_u8(u32::from(self.water[i]) * (255 - a) / 255);
            if s.occludes {
                self.occluders[i] = self.occluders[i].max(px[3]);
            }
            if s.crown.is_some() {
                self.tops[i] = self.tops[i].max(px[3]);
            }
            if let Some(hgt) = s.height {
                if px[3] >= 128 {
                    #[allow(clippy::cast_precision_loss)] // canvas pixels
                    let base = terrain::to_units(terrain.height(x as f32 + 0.5, y as f32 + 0.5));
                    self.heights[i] = self.heights[i].max(base + hgt);
                }
            }
        }
    }

    /// The silhouette cast by source row `y` of a canopy onto row `y + off`,
    /// which lies inside this band.
    fn silhouette_row(&mut self, op: &Op, y: i64, off: i64, y0: i64, cw: i64, ch: i64) {
        if y < 0 || y >= ch {
            return;
        }
        let sy = u32::try_from(y - op.oy).unwrap_or(0);
        let local = y + off - y0;
        for sx in 0..op.sprite.width {
            let x = op.ox + i64::from(sx);
            if x < 0 || x >= cw || x + off >= cw {
                continue;
            }
            let a = op.sprite.get(sx, sy)[3];
            if a == 0 {
                continue;
            }
            let j = usize::try_from(local * cw + x + off).unwrap_or(0);
            self.silhouettes[j] = self.silhouettes[j].max(a);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::field::Frame;
    use crate::layout::TacticalLayout;

    /// The one-sprite-at-a-time stamp the bands replace (goal 50).
    fn serial(img: &mut Rgba, buf: &mut Buffers, casts: &mut Casts, t: &Terrain<'_>, ops: &[Op]) {
        let (cw, ch) = (i64::from(img.width), i64::from(img.height));
        for op in ops {
            let s = op.stamp;
            for sy in 0..op.sprite.height {
                let y = op.oy + i64::from(sy);
                if y < 0 || y >= ch {
                    continue;
                }
                for sx in 0..op.sprite.width {
                    let x = op.ox + i64::from(sx);
                    if x < 0 || x >= cw {
                        continue;
                    }
                    let px = op.sprite.get(sx, sy);
                    if px[3] == 0 {
                        continue;
                    }
                    let (ux, uy) = (u32::try_from(x).unwrap(), u32::try_from(y).unwrap());
                    img.blend(ux, uy, px);
                    let i = usize::try_from(y * cw + x).unwrap();
                    let a = u32::from(px[3]);
                    buf.water[i] = to_u8(u32::from(buf.water[i]) * (255 - a) / 255);
                    if s.occludes {
                        buf.occluders[i] = buf.occluders[i].max(px[3]);
                    }
                    if let Some(off) = s.crown {
                        casts.tops[i] = casts.tops[i].max(px[3]);
                        let (sx2, sy2) = (x + off, y + off);
                        if sx2 < cw && sy2 < ch {
                            let j = usize::try_from(sy2 * cw + sx2).unwrap();
                            casts.silhouettes[j] = casts.silhouettes[j].max(px[3]);
                        }
                    }
                    if let Some(hgt) = s.height {
                        if px[3] >= 128 {
                            #[allow(clippy::cast_precision_loss)]
                            let base = terrain::to_units(t.height(x as f32 + 0.5, y as f32 + 0.5));
                            buf.height_map[i] = buf.height_map[i].max(base + hgt);
                        }
                    }
                }
            }
        }
    }

    fn sprite(w: u32, h: u32, k: u32) -> Arc<Rgba> {
        let mut img = Rgba::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let v = (x * 37 + y * 11 + k * 5) % 256;
                let a = [0, 90, 200, 255][((x + y + k) % 4) as usize];
                let c = u8::try_from(v).unwrap();
                img.set(x, y, [c, 255 - c, c / 2, a]);
            }
        }
        Arc::new(img)
    }

    #[test]
    fn banded_stamps_equal_serial_stamps() {
        let mut layout = TacticalLayout::new("t", 5, 4, "grass");
        for (i, sq) in layout.squares.iter_mut().enumerate() {
            sq.elevation_ft = i16::try_from(i % 7).unwrap();
        }
        let frame = Frame::of(&layout, 16);
        let terrain = Terrain::new(&layout, &frame, 3);
        let (w, h) = (80, 64);
        let ops: Vec<Op> = (0..40)
            .map(|k: u32| Op {
                sprite: sprite(5 + k % 23, 3 + k % 29, k),
                ox: i64::from(k * 13 % 90) - 10,
                oy: i64::from(k * 29 % 80) - 12,
                stamp: Stamp {
                    height: k
                        .is_multiple_of(3)
                        .then_some(256 * i32::try_from(k).unwrap()),
                    occludes: k.is_multiple_of(2),
                    crown: (k % 4 == 1).then_some(i64::from(k % 9)),
                },
            })
            .collect();
        let fresh = || {
            let mut img = Rgba::filled(w, h, [40, 90, 30, 255]);
            img.data[7] = 7;
            let mut buf = Buffers::new(w, h);
            buf.water.iter_mut().for_each(|v| *v = 200);
            (img, buf, Casts::new(w, h))
        };
        let (mut a_img, mut a_buf, mut a_casts) = fresh();
        stamp_all(&mut a_img, &mut a_buf, &mut a_casts, &terrain, &ops);
        let (mut b_img, mut b_buf, mut b_casts) = fresh();
        serial(&mut b_img, &mut b_buf, &mut b_casts, &terrain, &ops);
        assert_eq!(a_img, b_img);
        assert_eq!(a_buf.water, b_buf.water);
        assert_eq!(a_buf.occluders, b_buf.occluders);
        assert_eq!(a_buf.height_map, b_buf.height_map);
        assert_eq!(a_casts.tops, b_casts.tops);
        assert_eq!(a_casts.silhouettes, b_casts.silhouettes);
    }
}
