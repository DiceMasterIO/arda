//! Small iterative radix-2 FFT (no dependency), plus a 2D transform.

use std::f64::consts::PI;

/// In-place forward FFT of a power-of-two length signal.
pub fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * PI / len as f64;
        let (wr, wi) = (ang.cos(), ang.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0, 0.0);
            for k in 0..len / 2 {
                let (a, b) = (start + k, start + k + len / 2);
                let tr = re[b] * cr - im[b] * ci;
                let ti = re[b] * ci + im[b] * cr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                let nr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = nr;
            }
        }
        len <<= 1;
    }
}

/// Power spectrum `|F|^2` of a real `n x n` field (row-major, `n` a power of two).
pub fn power2d(field: &[f64], n: usize) -> Vec<f64> {
    let mut re = field.to_vec();
    let mut im = vec![0.0; n * n];
    for y in 0..n {
        fft(&mut re[y * n..(y + 1) * n], &mut im[y * n..(y + 1) * n]);
    }
    let (mut cr, mut ci) = (vec![0.0; n], vec![0.0; n]);
    for x in 0..n {
        for y in 0..n {
            cr[y] = re[y * n + x];
            ci[y] = im[y * n + x];
        }
        fft(&mut cr, &mut ci);
        for y in 0..n {
            re[y * n + x] = cr[y];
            im[y * n + x] = ci[y];
        }
    }
    re.iter().zip(&im).map(|(a, b)| a * a + b * b).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_cosine_has_one_peak_pair() {
        let n = 64;
        let f: Vec<f64> = (0..n * n)
            .map(|i| (2.0 * PI * 5.0 * (i % n) as f64 / n as f64).cos())
            .collect();
        let p = power2d(&f, n);
        assert!(p[5] > 1e6 && p[n - 5] > 1e6);
        assert!(p[6] < 1e-6);
    }
}
