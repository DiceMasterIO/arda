//! Boundary loops of a boolean mask on the cell grid (water on the left).

/// A closed boundary loop of unit edges.
pub struct Loop {
    /// Vertices; edge `i` runs from `pts[i]` to `pts[(i + 1) % len]`.
    pub pts: Vec<(i32, i32)>,
    /// A mask cell next to the loop's first edge.
    pub cell: (i32, i32),
}

/// Traces every boundary between mask cells and non-mask cells inside the
/// grid. Heading codes: 0 east, 1 south, 2 west, 3 north (y grows down).
pub fn trace(w: usize, h: usize, mask: &[bool]) -> Vec<Loop> {
    let vw = w + 1;
    let mut out = vec![0_u8; vw * (h + 1)];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if x + 1 < w && mask[i] != mask[i + 1] {
                // vertical edge at vertex column x+1; L (non-mask) on the right
                if mask[i + 1] {
                    out[y * vw + x + 1] |= 1 << 1; // south, west cell is L
                } else {
                    out[(y + 1) * vw + x + 1] |= 1 << 3; // north, east cell is L
                }
            }
            if y + 1 < h && mask[i] != mask[i + w] {
                if mask[i] {
                    out[(y + 1) * vw + x] |= 1; // east, south cell is L
                } else {
                    out[(y + 1) * vw + x + 1] |= 1 << 2; // west, north cell is L
                }
            }
        }
    }
    let step = |d: usize| -> (i32, i32) { [(1, 0), (0, 1), (-1, 0), (0, -1)][d] };
    // Curves cut by the grid edge are open: walk them from their true start.
    let mut has_in = vec![false; out.len()];
    for (v, &bits) in out.iter().enumerate() {
        for d in 0..4 {
            if bits & (1 << d) != 0 {
                let (dx, dy) = step(d);
                let (x, y) = ((v % vw) as i32 + dx, (v / vw) as i32 + dy);
                has_in[y as usize * vw + x as usize] = true;
            }
        }
    }
    let mut loops = Vec::new();
    let starts: Vec<usize> = (0..out.len())
        .filter(|&v| out[v] != 0 && !has_in[v])
        .chain(0..out.len())
        .collect();
    for v0 in starts {
        while out[v0] != 0 {
            let d0 = out[v0].trailing_zeros() as usize;
            let (x0, y0) = ((v0 % vw) as i32, (v0 / vw) as i32);
            let cell = match d0 {
                0 => (x0, y0 - 1),
                1 => (x0, y0),
                2 => (x0 - 1, y0),
                _ => (x0 - 1, y0 - 1),
            };
            let mut pts = vec![(x0, y0)];
            let (mut x, mut y, mut d) = (x0, y0, d0);
            out[v0] &= !(1 << d0);
            loop {
                let (dx, dy) = step(d);
                (x, y) = (x + dx, y + dy);
                let v = y as usize * vw + x as usize;
                let pick = [(d + 1) % 4, d, (d + 3) % 4]
                    .into_iter()
                    .find(|&c| out[v] & (1 << c) != 0);
                match pick {
                    Some(c) => {
                        out[v] &= !(1 << c);
                        pts.push((x, y));
                        d = c;
                    }
                    None => {
                        if (x, y) != (x0, y0) {
                            pts.push((x, y));
                        }
                        break;
                    }
                }
            }
            loops.push(Loop { pts, cell });
        }
    }
    loops
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_island_is_one_loop_of_perimeter_edges() {
        let (w, h) = (10, 10);
        let mut m = vec![false; w * h];
        for y in 3..7 {
            for x in 3..7 {
                m[y * w + x] = true;
            }
        }
        let l = trace(w, h, &m);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].pts.len(), 16);
    }
}
