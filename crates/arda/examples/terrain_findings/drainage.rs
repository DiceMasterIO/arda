//! OI-03 measures: descent and reach directions, straight channel runs,
//! incision troughs and the valley-spacing spectrum.

use crate::fft::power2d;
use crate::grid::{bin24, Grid, LAND};
use crate::json::J;
use std::f64::consts::PI;

/// Minimum gradient for a descent sample: about 3 degrees (mm per cell).
pub const MIN_GRAD_MM: f64 = 5000.0;
/// Reach chord length, in downstream steps (1 km).
pub const REACH_STEPS: usize = 10;
/// Spectrum window side, cells (25.6 km).
pub const WIN: usize = 256;
/// A window is "regular" when its strongest bin is this many times the
/// azimuthal mean power at the same wavenumber (pure noise gives about 6-10).
pub const PEAK_RATIO_FLAG: f64 = 20.0;
/// Straight channel runs at least this long (cells) are reported as long.
pub const LONG_RUN_CELLS: usize = 200;

/// Directed 24-bin histogram of steepest-descent directions on land.
pub fn descent_hist(g: &Grid) -> [u64; 24] {
    let mut h = [0_u64; 24];
    let w = g.w;
    for y in 1..g.h.saturating_sub(1) {
        for x in 1..w - 1 {
            let i = y * w + x;
            if [i, i - 1, i + 1, i - w, i + w]
                .iter()
                .any(|&j| g.kind[j] != LAND)
            {
                continue;
            }
            let gx = f64::from(g.z[i + 1] - g.z[i - 1]) / 2.0;
            let gy = f64::from(g.z[i + w] - g.z[i - w]) / 2.0;
            if gx.hypot(gy) < MIN_GRAD_MM {
                continue;
            }
            h[bin24((-gy).atan2(-gx).to_degrees())] += 1;
        }
    }
    h
}

const N8: [(isize, isize); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

fn channel(g: &Grid, i: usize) -> bool {
    g.kind[i] == LAND && g.order[i] > 0
}

/// Directed 24-bin histogram of 1 km downstream chords of channel cells.
pub fn reach_hist(g: &Grid) -> [u64; 24] {
    let mut h = [0_u64; 24];
    let (w, hh) = (g.w as isize, g.h as isize);
    for y0 in 0..hh {
        for x0 in 0..w {
            let start = (y0 * w + x0) as usize;
            if !channel(g, start) {
                continue;
            }
            let (mut x, mut y) = (x0, y0);
            let mut ok = true;
            for _ in 0..REACH_STEPS {
                let here = g.z[(y * w + x) as usize];
                let mut best: Option<(i32, isize, isize)> = None;
                for (dx, dy) in N8 {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= w || ny >= hh {
                        continue;
                    }
                    let j = (ny * w + nx) as usize;
                    if channel(g, j) && g.z[j] < here && best.is_none_or(|b| g.z[j] < b.0) {
                        best = Some((g.z[j], nx, ny));
                    }
                }
                match best {
                    Some((_, nx, ny)) => (x, y) = (nx, ny),
                    None => {
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                h[bin24(((y - y0) as f64).atan2((x - x0) as f64).to_degrees())] += 1;
            }
        }
    }
    h
}

/// One maximal straight run of channel cells.
pub struct Run {
    pub cells: usize,
    pub metres: f64,
    pub x: usize,
    pub y: usize,
    pub dir: usize,
    pub trough_m: Option<f64>,
}

/// All straight runs (E, S, SE, SW) of at least `min` cells.
pub fn runs(g: &Grid, min: usize) -> Vec<Run> {
    let dirs: [(isize, isize); 4] = [(1, 0), (0, 1), (1, 1), (-1, 1)];
    let (w, h) = (g.w as isize, g.h as isize);
    let mut out = Vec::new();
    for (d, &(dx, dy)) in dirs.iter().enumerate() {
        for y in 0..h {
            for x in 0..w {
                if !channel(g, (y * w + x) as usize) {
                    continue;
                }
                let (px, py) = (x - dx, y - dy);
                if px >= 0 && py >= 0 && px < w && py < h && channel(g, (py * w + px) as usize) {
                    continue;
                }
                let (mut n, mut cx, mut cy) = (0_usize, x, y);
                while cx >= 0 && cy >= 0 && cx < w && cy < h && channel(g, (cy * w + cx) as usize) {
                    n += 1;
                    cx += dx;
                    cy += dy;
                }
                if n >= min {
                    let step = if d < 2 { 100.0 } else { 141.42 };
                    let trough_m = if d < 2 { trough(g, x, y, d, n) } else { None };
                    out.push(Run {
                        cells: n,
                        metres: n as f64 * step,
                        x: x as usize,
                        y: y as usize,
                        dir: d,
                        trough_m,
                    });
                }
            }
        }
    }
    out
}

/// Mean height of the ground 1 km either side of a run above its channel, m.
fn trough(g: &Grid, x: isize, y: isize, d: usize, n: usize) -> Option<f64> {
    let (w, h) = (g.w as isize, g.h as isize);
    let (px, py) = if d == 0 { (0, 10) } else { (10, 0) };
    let (mut sum, mut cnt) = (0.0, 0_usize);
    for k in 0..n as isize {
        let (cx, cy) = if d == 0 { (x + k, y) } else { (x, y + k) };
        let (ax, ay, bx, by) = (cx + px, cy + py, cx - px, cy - py);
        if ax < 0 || ay < 0 || bx < 0 || by < 0 || ax >= w || bx >= w || ay >= h || by >= h {
            continue;
        }
        let (a, b, c) = (
            (ay * w + ax) as usize,
            (by * w + bx) as usize,
            (cy * w + cx) as usize,
        );
        if g.kind[a] == LAND && g.kind[b] == LAND {
            sum += (f64::from(g.z[a]) + f64::from(g.z[b])) / 2.0 - f64::from(g.z[c]);
            cnt += 1;
        }
    }
    (cnt * 2 >= n).then(|| sum / cnt as f64 / 1000.0)
}

/// Run statistics as JSON (runs of at least 20 cells are kept for ranking).
pub fn runs_json(rs: &[Run]) -> J {
    let mut sorted: Vec<&Run> = rs.iter().collect();
    sorted.sort_by(|a, b| {
        b.metres
            .total_cmp(&a.metres)
            .then(a.y.cmp(&b.y))
            .then(a.x.cmp(&b.x))
    });
    let long = rs.iter().filter(|r| r.cells >= LONG_RUN_CELLS).count();
    let names = ["E", "S", "SE", "SW"];
    let troughs: Vec<f64> = rs
        .iter()
        .filter(|r| r.cells >= 50)
        .filter_map(|r| r.trough_m)
        .collect();
    let mean = troughs.iter().sum::<f64>() / troughs.len().max(1) as f64;
    J::obj()
        .with("runs_ge_20_cells", J::u(rs.len()))
        .with("runs_ge_20km", J::u(long))
        .with("longest_cells", J::u(sorted.first().map_or(0, |r| r.cells)))
        .with("longest_m", J::F(sorted.first().map_or(0.0, |r| r.metres)))
        .with(
            "top5",
            J::A(
                sorted
                    .iter()
                    .take(5)
                    .map(|r| {
                        J::obj()
                            .with("dir", J::S(names[r.dir].into()))
                            .with("cells", J::u(r.cells))
                            .with("x", J::u(r.x))
                            .with("y", J::u(r.y))
                    })
                    .collect(),
            ),
        )
        .with("axis_runs_ge_5km_with_trough", J::u(troughs.len()))
        .with("trough_mean_m", J::F(mean))
        .with(
            "trough_max_m",
            J::F(troughs.iter().copied().fold(0.0, f64::max)),
        )
}

/// Result of the spectral survey over mountain windows.
pub struct Spectrum {
    pub mountain_windows: usize,
    pub ratios: Vec<f64>,
    pub flagged: usize,
    pub cluster: usize,
    pub modal_km: f64,
    pub modal_deg: f64,
    pub worst: Option<(usize, usize, f64)>,
}

/// Peak-to-background ratio, wavelength (km) and wave direction (deg mod 180)
/// of the strongest bin of a `WIN x WIN` window.
pub fn window_peak(z: &[f64]) -> (f64, f64, f64) {
    let n = WIN;
    let mean = z.iter().sum::<f64>() / z.len() as f64;
    let c = (n as f64 - 1.0) / 2.0;
    let (mut sx, mut sy, mut sxx) = (0.0, 0.0, 0.0);
    for (i, v) in z.iter().enumerate() {
        let (x, y) = ((i % n) as f64 - c, (i / n) as f64 - c);
        sx += (v - mean) * x;
        sy += (v - mean) * y;
        sxx += x * x;
    }
    let (ax, ay) = (sx / sxx, sy / sxx);
    let field: Vec<f64> = z
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let (xi, yi) = (i % n, i / n);
            let hann = |k: usize| 0.5 - 0.5 * (2.0 * PI * k as f64 / (n as f64 - 1.0)).cos();
            (v - mean - ax * (xi as f64 - c) - ay * (yi as f64 - c)) * hann(xi) * hann(yi)
        })
        .collect();
    let p = power2d(&field, n);
    let signed = |k: usize| {
        if k <= n / 2 {
            k as isize
        } else {
            k as isize - n as isize
        }
    };
    let (mut sum, mut cnt) = ([0.0; 64], [0.0; 64]);
    let mut bins = Vec::new();
    for ky in 0..n {
        for kx in 0..=n / 2 {
            let (fx, fy) = (signed(kx), signed(ky));
            if !(fx > 0 || (fx == 0 && fy > 0)) {
                continue;
            }
            let r = ((fx * fx + fy * fy) as f64).sqrt();
            let ri = r.round() as usize;
            if (2..=25).contains(&ri) {
                sum[ri] += p[ky * n + kx];
                cnt[ri] += 1.0;
                bins.push((ri, fx, fy, r, p[ky * n + kx]));
            }
        }
    }
    let mut best = (0.0, 0.0, 0.0);
    for (ri, fx, fy, r, pw) in bins {
        let bg = sum[ri] / cnt[ri];
        let q = if bg > 0.0 { pw / bg } else { 0.0 };
        if q > best.0 {
            best = (
                q,
                WIN as f64 * 0.1 / r,
                (fy as f64).atan2(fx as f64).to_degrees().rem_euclid(180.0),
            );
        }
    }
    best
}

/// Surveys up to 96 evenly chosen mountain windows (all land, mean height
/// at least 800 m, relief at least 300 m).
pub fn spectrum(g: &Grid) -> Spectrum {
    let mut cands = Vec::new();
    for wy in 0..g.h / WIN {
        for wx in 0..g.w / WIN {
            let (mut lo, mut hi, mut sum, mut all) = (i32::MAX, i32::MIN, 0.0, true);
            'scan: for y in 0..WIN {
                for x in 0..WIN {
                    let i = (wy * WIN + y) * g.w + wx * WIN + x;
                    if g.kind[i] != LAND {
                        all = false;
                        break 'scan;
                    }
                    lo = lo.min(g.z[i]);
                    hi = hi.max(g.z[i]);
                    sum += f64::from(g.z[i]);
                }
            }
            if all && sum / (WIN * WIN) as f64 >= 800_000.0 && hi - lo >= 300_000 {
                cands.push((wx, wy));
            }
        }
    }
    let total = cands.len();
    let picked: Vec<(usize, usize)> = if total <= 96 {
        cands
    } else {
        (0..96).map(|k| cands[k * total / 96]).collect()
    };
    let mut peaks = Vec::new();
    let mut ratios = Vec::new();
    for &(wx, wy) in &picked {
        let mut z = vec![0.0; WIN * WIN];
        for y in 0..WIN {
            for x in 0..WIN {
                z[y * WIN + x] = f64::from(g.z[(wy * WIN + y) * g.w + wx * WIN + x]) / 1000.0;
            }
        }
        let pk = window_peak(&z);
        ratios.push(pk.0);
        peaks.push((wx, wy, pk));
    }
    let flagged: Vec<_> = peaks.iter().filter(|p| p.2 .0 >= PEAK_RATIO_FLAG).collect();
    let (mut cluster, mut modal) = (0, (0.0, 0.0));
    for a in &flagged {
        let n = flagged
            .iter()
            .filter(|b| {
                let dd = (a.2 .2 - b.2 .2).abs();
                (a.2 .1 - b.2 .1).abs() / a.2 .1 <= 0.2 && dd.min(180.0 - dd) <= 15.0
            })
            .count();
        if n > cluster {
            cluster = n;
            modal = (a.2 .1, a.2 .2);
        }
    }
    let worst = peaks
        .iter()
        .max_by(|a, b| a.2 .0.total_cmp(&b.2 .0))
        .map(|p| (p.0 * WIN + WIN / 2, p.1 * WIN + WIN / 2, p.2 .0));
    let mut sorted = ratios.clone();
    sorted.sort_by(f64::total_cmp);
    Spectrum {
        mountain_windows: total,
        ratios: sorted,
        flagged: flagged.len(),
        cluster,
        modal_km: modal.0,
        modal_deg: modal.1,
        worst,
    }
}

impl Spectrum {
    /// The world shows a shared lattice when at least 3 and a quarter of the
    /// surveyed windows share a flagged wavelength and direction.
    pub fn lattice(&self) -> bool {
        let n = self.ratios.len();
        n >= 3 && self.cluster >= 3 && self.cluster * 4 >= n
    }

    /// JSON summary.
    pub fn json(&self) -> J {
        let q = |p: usize| {
            self.ratios
                .get((self.ratios.len().max(1) - 1) * p / 100)
                .copied()
                .unwrap_or(0.0)
        };
        J::obj()
            .with("mountain_windows_total", J::u(self.mountain_windows))
            .with("windows_surveyed", J::u(self.ratios.len()))
            .with("peak_ratio_median", J::F(q(50)))
            .with("peak_ratio_p90", J::F(q(90)))
            .with("peak_ratio_max", J::F(q(100)))
            .with("windows_flagged", J::u(self.flagged))
            .with("largest_common_cluster", J::u(self.cluster))
            .with("cluster_wavelength_km", J::F(self.modal_km))
            .with("cluster_wave_dir_deg", J::F(self.modal_deg))
            .with("lattice", J::B(self.lattice()))
    }
}
