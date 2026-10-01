//! Streams finished overview rows through the oblique look
//! ([`super::oblique`]): each output row is gathered from the rows south of
//! it, so only the next `reach` rows are ever held.

use super::oblique::{shade, ObliqueRelief, ONE};
use rayon::prelude::*;
use std::io::{self, Write};

/// Fixed-point iterations solving `y = Y + shift(y)`; the shift's slope is
/// at most `tilt · slope(400 m)`, well below 1, so this converges to a
/// fraction of a pixel.
const SOLVE_STEPS: usize = 6;

/// An [`io::Write`] adapter that warps and shades whole RGB rows.
pub(super) struct ObliqueRows<'a> {
    src: Source<'a>,
    out: &'a mut dyn Write,
    pending: Vec<u8>,
    emitted: u32,
    /// Rows past an output row that must have arrived before it is drawn.
    reach: u32,
}

/// The rows received so far and the geometry to draw from them.
struct Source<'a> {
    relief: &'a ObliqueRelief,
    width: usize,
    height: u32,
    /// Rows `[first, received)`.
    rows: Vec<u8>,
    first: u32,
    received: u32,
    /// Column centres in saved cells, Q16.
    columns: Vec<i64>,
}

impl<'a> ObliqueRows<'a> {
    /// An adapter for a `width × height` image of the world `relief` covers.
    pub(super) fn new(
        relief: &'a ObliqueRelief,
        width: u32,
        height: u32,
        out: &'a mut dyn Write,
    ) -> Self {
        let (cw, ch) = relief.cells();
        let columns = (0..i64::from(width))
            .map(|x| (2 * x + 1) * i64::from(cw) * 65_536 / (2 * i64::from(width)))
            .collect();
        let max_px = relief.max_surface_mm() * relief.tilt_q12() / ONE * i64::from(height)
            / (i64::from(ch) * 100_000).max(1);
        Self {
            src: Source {
                relief,
                width: usize::try_from(width).unwrap_or(0),
                height,
                rows: Vec::new(),
                first: 0,
                received: 0,
                columns,
            },
            out,
            pending: Vec::new(),
            emitted: 0,
            reach: u32::try_from(max_px + 4).unwrap_or(u32::MAX),
        }
    }

    /// Draws every output row whose sources have arrived.
    fn drain(&mut self) -> io::Result<()> {
        let src = &mut self.src;
        let ready = if src.received >= src.height {
            src.height
        } else {
            src.received.saturating_sub(self.reach)
        };
        if ready <= self.emitted {
            return Ok(());
        }
        let shared: &Source<'_> = src;
        let rows: Vec<Vec<u8>> = (self.emitted..ready)
            .into_par_iter()
            .map(|y| shared.draw(y))
            .collect();
        for row in rows {
            self.out.write_all(&row)?;
        }
        self.emitted = ready;
        // Keep one row above the next output row for the cubic.
        let keep = self.emitted.saturating_sub(1).max(src.first);
        let drop = usize::try_from(keep - src.first).unwrap_or(0) * src.width * 3;
        src.rows.drain(..drop);
        src.first = keep;
        Ok(())
    }

    /// Draws the remaining rows; every input row must have been written.
    pub(super) fn finish(mut self) -> io::Result<()> {
        if self.src.received != self.src.height || !self.pending.is_empty() {
            return Err(io::Error::other("oblique rows: incomplete image"));
        }
        self.drain()
    }
}

impl Source<'_> {
    /// Row centre `p` (pixels, Q8) in saved cells, Q16.
    fn row_cells(&self, p: i64) -> i64 {
        let (_, ch) = self.relief.cells();
        (2 * p + 256) * i64::from(ch) * 128 / i64::from(self.height)
    }

    /// Northward shift of the surface at `(x, p)`, pixels Q8.
    fn shift_q8(&self, cx: i64, p: i64) -> i64 {
        let (_, ch) = self.relief.cells();
        let mm = self.relief.shift_mm(cx, self.row_cells(p));
        mm * 256 * i64::from(self.height) / (i64::from(ch) * 100_000).max(1)
    }

    fn pixel(&self, x: usize, y: u32) -> [i64; 3] {
        let last = self.received.saturating_sub(1);
        let r = y.clamp(self.first, last);
        let at = usize::try_from(r - self.first).unwrap_or(0) * self.width * 3 + x * 3;
        self.rows
            .get(at..at + 3)
            .map_or([0; 3], |p| [p[0], p[1], p[2]].map(i64::from))
    }

    /// Output row `y`.
    fn draw(&self, y: u32) -> Vec<u8> {
        let mut out = vec![0_u8; self.width * 3];
        let base = i64::from(y) * 256;
        let max_row = i64::from(self.height) - 1;
        for (x, px) in out.as_chunks_mut::<3>().0.iter_mut().enumerate() {
            let cx = self.columns[x];
            let mut p = base;
            for _ in 0..SOLVE_STEPS {
                p = base + self.shift_q8(cx, p);
            }
            let p = p.min(max_row * 256);
            // Catmull-Rom across rows keeps the stretched flanks crisp.
            let t = (p & 255) * ONE / 256;
            let (t2, t3) = (t * t / ONE, t * t / ONE * t / ONE);
            let w = [
                (-t3 + 2 * t2 - t) / 2,
                (3 * t3 - 5 * t2 + 2 * ONE) / 2,
                (-3 * t3 + 4 * t2 + t) / 2,
                (t3 - t2) / 2,
            ];
            let r0 = p >> 8;
            let mut c = [0_i64; 3];
            for (k, wk) in w.iter().enumerate() {
                let row = (r0 - 1 + i64::try_from(k).unwrap_or(0)).clamp(0, max_row);
                let s = self.pixel(x, u32::try_from(row).unwrap_or(0));
                for ch in 0..3 {
                    c[ch] += s[ch] * wk;
                }
            }
            let c = c.map(|v| v / ONE);
            let sample = self.relief.sample(cx, self.row_cells(p));
            let c = shade(c, &sample);
            for (o, v) in px.iter_mut().zip(c) {
                *o = u8::try_from(v.clamp(0, 255)).unwrap_or(255);
            }
        }
        out
    }
}

impl Write for ObliqueRows<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.pending.extend_from_slice(buf);
        let row = self.src.width * 3;
        let whole = self.pending.len() / row.max(1) * row;
        if whole > 0 {
            self.src.rows.extend(self.pending.drain(..whole));
            self.src.received += u32::try_from(whole / row).unwrap_or(0);
            self.drain()?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::overview::oblique::DEFAULT_TILT_Q12;

    /// A 256 × 256-cell world (64² nodes): sea on the west half, a flat
    /// 1 km plateau on the east half.
    fn plateau() -> ObliqueRelief {
        let nodes = (64, 64);
        let s = (0..64 * 64)
            .map(|i| if i % 64 >= 32 { 1_000_000 } else { 0 })
            .collect();
        ObliqueRelief::from_nodes((256, 256), nodes, s, DEFAULT_TILT_Q12)
    }

    /// A 256 × 256-px image (100 m per pixel): grey with one white row.
    fn image(mark_row: usize) -> Vec<u8> {
        let mut rgb = vec![90_u8; 256 * 256 * 3];
        rgb[mark_row * 256 * 3..(mark_row + 1) * 256 * 3].fill(250);
        rgb
    }

    fn run(relief: &ObliqueRelief, rgb: &[u8], chunk_rows: usize) -> Vec<u8> {
        let mut out = Vec::new();
        let mut rows = ObliqueRows::new(relief, 256, 256, &mut out);
        for chunk in rgb.chunks(chunk_rows * 256 * 3) {
            rows.write_all(chunk).unwrap();
        }
        rows.finish().unwrap();
        out
    }

    #[test]
    fn output_does_not_depend_on_how_rows_arrive() {
        let r = plateau();
        let rgb = image(150);
        let whole = run(&r, &rgb, 256);
        assert_eq!(whole.len(), rgb.len());
        for chunk in [1, 7, 64] {
            assert_eq!(run(&r, &rgb, chunk), whole, "chunk {chunk}");
        }
    }

    #[test]
    fn sea_stays_exact_and_land_moves_north_by_tilt_times_height() {
        let r = plateau();
        let rgb = image(150);
        let out = run(&r, &rgb, 16);
        let at = |x: usize, y: usize| out[(y * 256 + x) * 3];
        // West (sea) columns are untouched, byte for byte.
        for y in 0..256 {
            for x in 0..100 {
                let i = (y * 256 + x) * 3;
                assert_eq!(out[i..i + 3], rgb[i..i + 3], "{x},{y}");
            }
        }
        // 1 km at tilt 0.5 is 500 m: five 100 m pixels north.
        let expected = 150 - 1_000_000 * DEFAULT_TILT_Q12 / ONE / 100_000;
        let brightest = (0..256).max_by_key(|&y| at(200, y)).unwrap();
        assert_eq!(i64::try_from(brightest).unwrap(), expected);
        assert!(at(200, 150) < 200, "the mark left its map row");
    }

    #[test]
    fn incomplete_images_are_refused() {
        let r = plateau();
        let mut out = Vec::new();
        let mut rows = ObliqueRows::new(&r, 256, 256, &mut out);
        rows.write_all(&vec![0; 255 * 256 * 3]).unwrap();
        assert!(rows.finish().is_err());
    }
}
