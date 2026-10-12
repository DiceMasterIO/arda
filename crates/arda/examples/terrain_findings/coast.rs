//! OI-04 / OI-05 boundary measures over traced loops: chord directions,
//! turning angles, stair-step runs and box-counting dimension.

use crate::grid::{bin12, shares};
use crate::json::J;
use crate::trace::Loop;

/// Chord scales in unit edges: 1 km, 5 km, 20 km.
pub const SCALES: [usize; 3] = [10, 50, 200];
/// Edges in runs at least this long count as long straight stretches (800 m).
pub const LONG_RUN: usize = 8;
/// A stretch of at least this many consecutive repeating runs is a stair.
pub const STAIR_REPEATS: usize = 6;
/// Flag thresholds, fixed before the real worlds were measured.
pub const DIR_RATIO_FLAG: f64 = 1.4;
/// Grid-boundness: the rotated control must stay below this ratio.
pub const CONTROL_RATIO_MAX: f64 = 1.2;
// The stair share (periodic or exact-diagonal runs) is reported as the
// 100 m quantisation signature only: a rasterised circle scores 0.32 on it,
// so it cannot separate quantisation from real angularity and is not a flag.

/// Accumulated boundary statistics.
#[derive(Default)]
pub struct Acc {
    pub edges: u64,
    pub chord: [[u64; 12]; 3],
    pub turn: [[u64; 12]; 3],
    pub run_hist: [u64; 7],
    pub long_edges: u64,
    pub periodic_edges: u64,
    pub diag1_edges: u64,
    pub longest_run: usize,
    pub longest_at: (i32, i32),
    pub verts: Vec<(i32, i32)>,
}

fn run_bin(n: usize) -> usize {
    match n {
        1 => 0,
        2 => 1,
        3..=4 => 2,
        5..=8 => 3,
        9..=16 => 4,
        17..=32 => 5,
        _ => 6,
    }
}

fn ang(a: (i32, i32), b: (i32, i32)) -> f64 {
    f64::from(b.1 - a.1)
        .atan2(f64::from(b.0 - a.0))
        .to_degrees()
}

/// Measures all loops of at least `min_edges` edges.
pub fn measure(loops: &[Loop], min_edges: usize) -> Acc {
    let mut acc = Acc::default();
    for l in loops.iter().filter(|l| l.pts.len() >= min_edges) {
        let p = &l.pts;
        let n = p.len();
        acc.edges += n as u64;
        acc.verts.extend(p.iter().copied());
        for (s, &m) in SCALES.iter().enumerate() {
            if n < 4 * m {
                continue;
            }
            let mut i = 0;
            while i < n {
                let (a, b, c) = (p[i], p[(i + m) % n], p[(i + 2 * m) % n]);
                acc.chord[s][bin12(ang(a, b))] += 1;
                let mut t = (ang(b, c) - ang(a, b)).rem_euclid(360.0);
                if t > 180.0 {
                    t = 360.0 - t;
                }
                acc.turn[s][((t / 15.0) as usize).min(11)] += 1;
                i += (m / 2).max(1);
            }
        }
        runs(&mut acc, p);
    }
    acc
}

/// Length, heading and start vertex of one straight run.
type Run = (usize, (i32, i32), (i32, i32));

fn runs(acc: &mut Acc, p: &[(i32, i32)]) {
    let n = p.len();
    let dir = |i: usize| -> (i32, i32) {
        let (a, b) = (p[i], p[(i + 1) % n]);
        (b.0 - a.0, b.1 - a.1)
    };
    let Some(first) = (0..n).find(|&i| dir(i) != dir((i + n - 1) % n)) else {
        return;
    };
    let mut r: Vec<Run> = Vec::new();
    let mut i = 0;
    while i < n {
        let at = (first + i) % n;
        let d = dir(at);
        let mut len = 1;
        while i + len < n && dir((first + i + len) % n) == d {
            len += 1;
        }
        r.push((len, d, p[at]));
        i += len;
    }
    for &(len, _, at) in &r {
        acc.run_hist[run_bin(len)] += len as u64;
        if len >= LONG_RUN {
            acc.long_edges += len as u64;
        }
        if len > acc.longest_run {
            acc.longest_run = len;
            acc.longest_at = at;
        }
    }
    let k = r.len();
    let eq: Vec<bool> = (0..k.saturating_sub(2))
        .map(|j| r[j].0 == r[j + 2].0 && r[j].1 == r[j + 2].1)
        .collect();
    let mut covered = vec![false; k];
    let mut j = 0;
    while j < eq.len() {
        if !eq[j] {
            j += 1;
            continue;
        }
        let mut e = j;
        while e < eq.len() && eq[e] {
            e += 1;
        }
        if e - j >= STAIR_REPEATS {
            covered[j..e + 2].iter_mut().for_each(|c| *c = true);
        }
        j = e;
    }
    for (idx, &(len, _, _)) in r.iter().enumerate() {
        if covered[idx] {
            if len == 1 {
                acc.diag1_edges += 1;
            } else {
                acc.periodic_edges += len as u64;
            }
        }
    }
}

/// Box-counting dimension fitted over boxes of 4 to 128 cells.
pub fn box_dim(verts: &[(i32, i32)]) -> f64 {
    let (mut xs, mut ys) = (Vec::new(), Vec::new());
    for s in 2..=7_u32 {
        let size = 1_i32 << s;
        let mut keys: Vec<u64> = verts
            .iter()
            .map(|&(x, y)| (u64::from((x / size) as u32) << 32) | u64::from((y / size) as u32))
            .collect();
        keys.sort_unstable();
        keys.dedup();
        xs.push(-f64::from(s));
        ys.push((keys.len().max(1) as f64).log2());
    }
    let n = xs.len() as f64;
    let (mx, my) = (xs.iter().sum::<f64>() / n, ys.iter().sum::<f64>() / n);
    let num: f64 = xs.iter().zip(&ys).map(|(x, y)| (x - mx) * (y - my)).sum();
    let den: f64 = xs.iter().map(|x| (x - mx) * (x - mx)).sum();
    num / den
}

impl Acc {
    /// Combined axis+45 degree ratio at chord scale index `s`.
    pub fn ratio(&self, s: usize) -> f64 {
        shares(&self.chord[s]).ratio
    }

    /// Share of edges in periodic or exact-diagonal stairs.
    pub fn stair_share(&self) -> f64 {
        (self.periodic_edges + self.diag1_edges) as f64 / self.edges.max(1) as f64
    }

    /// Share of edges in runs of at least [`LONG_RUN`] cells.
    pub fn long_share(&self) -> f64 {
        self.long_edges as f64 / self.edges.max(1) as f64
    }

    /// JSON summary; `control` is the same measure on a rotated crop.
    pub fn json(&self, control: Option<&Acc>) -> J {
        let dim = if self.verts.len() > 64 {
            box_dim(&self.verts)
        } else {
            0.0
        };
        let mut scales = Vec::new();
        for (s, &m) in SCALES.iter().enumerate() {
            let sh = shares(&self.chord[s]);
            let sharp: u64 = self.turn[s][4..].iter().sum();
            let tot: u64 = self.turn[s].iter().sum();
            let mut o = J::obj()
                .with("chord_edges", J::u(m))
                .with("chord_hist_12", J::counts(&self.chord[s]))
                .with("axis_share", J::F(sh.axis))
                .with("diag_share", J::F(sh.diag))
                .with("ratio_vs_uniform", J::F(sh.ratio))
                .with("turn_hist_12", J::counts(&self.turn[s]))
                .with("turn_ge_60_share", J::F(sharp as f64 / tot.max(1) as f64));
            if let Some(c) = control {
                o = o.with("rotated_control_ratio", J::F(c.ratio(s)));
            }
            scales.push(o);
        }
        J::obj()
            .with("edges", J::I(i64::try_from(self.edges).unwrap_or(i64::MAX)))
            .with("scales", J::A(scales))
            .with(
                "run_length_hist_1_2_4_8_16_32_more",
                J::counts(&self.run_hist),
            )
            .with("long_run_edge_share", J::F(self.long_share()))
            .with("stair_edge_share", J::F(self.stair_share()))
            .with("longest_run_cells", J::u(self.longest_run))
            .with(
                "longest_run_at_xy",
                J::A(vec![
                    J::I(i64::from(self.longest_at.0)),
                    J::I(i64::from(self.longest_at.1)),
                ]),
            )
            .with("box_dimension", J::F(dim))
            .with("angular", J::B(self.angular(control)))
    }

    /// The pre-registered angularity rule: the combined axis+45 degree share of
    /// 5 km chords is at least 1.4 times uniform and the rotated control is not
    /// (so the excess is bound to the grid). Dimension and stairs are reported only.
    pub fn angular(&self, control: Option<&Acc>) -> bool {
        if self.edges < 200 {
            return false;
        }
        let enough = |a: &Acc| a.chord[1].iter().sum::<u64>() >= 100;
        enough(self)
            && self.ratio(1) >= DIR_RATIO_FLAG
            && control.is_none_or(|c| !enough(c) || c.ratio(1) <= CONTROL_RATIO_MAX)
    }
}
