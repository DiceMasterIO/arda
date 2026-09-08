// Arithmetic bounds: samples are i32, secants are at most 2^32-1,
// harmonic-tangent products fit i128, Hermite basis <=1000 and sums fit i64.
// Bounded interpolation returns the original four-corner i32 range.
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
// Bounded affine-preserving candidate, with no domain warp or noise changes.
// Symmetric average of x-then-y and y-then-x monotone Hermite interpolation.

fn rounded(n: i128, d: i128) -> i64 {
    let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
    if n < 0 {
        -((-n + d / 2) / d) as i64
    } else {
        ((n + d / 2) / d) as i64
    }
}

fn tangent(a: i64, b: i64) -> i64 {
    if a == 0 || b == 0 || a.signum() != b.signum() {
        0
    } else {
        rounded(2 * i128::from(a) * i128::from(b), i128::from(a + b))
    }
}

pub(crate) fn cubic(p: [i64; 4], r: i32) -> i64 {
    let r = i64::from(r);
    let r2 = r * r;
    let r3 = r2 * r;
    let m1 = tangent(p[1] - p[0], p[2] - p[1]);
    let m2 = tangent(p[2] - p[1], p[3] - p[2]);
    let n = (2 * r3 - 30 * r2 + 1000) * p[1]
        + (r3 - 20 * r2 + 100 * r) * m1
        + (-2 * r3 + 30 * r2) * p[2]
        + (r3 - 10 * r2) * m2;
    rounded(i128::from(n), 1000).clamp(p[1].min(p[2]), p[1].max(p[2]))
}

pub(crate) fn sample(x: i32, y: i32, at: impl Fn(i32, i32) -> i32) -> i32 {
    let (kx, ky, rx, ry) = (
        x.div_euclid(10),
        y.div_euclid(10),
        x.rem_euclid(10),
        y.rem_euclid(10),
    );
    let mut rows = [0; 4];
    let mut cols = [0; 4];
    for j in 0..4 {
        let mut row = [0; 4];
        let mut col = [0; 4];
        for i in 0..4 {
            row[i] = i64::from(at(kx + i as i32 - 1, ky + j as i32 - 1));
            col[i] = i64::from(at(kx + j as i32 - 1, ky + i as i32 - 1));
        }
        rows[j] = cubic(row, rx);
        cols[j] = cubic(col, ry);
    }
    rounded(i128::from(cubic(rows, ry) + cubic(cols, rx)), 2) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn translated_affine_planes_are_exact_at_every_phase() {
        for (a, b) in [
            (1000, 0),
            (0, 1000),
            (1000, 1000),
            (3000, 1000),
            (-3000, 1000),
            (1000, -3000),
        ] {
            for (tx, ty) in [(0, 0), (13, -27), (-19, 31)] {
                for y in -20..31 {
                    for x in -20..31 {
                        assert_eq!(
                            sample(x, y, |kx, ky| 300000
                                + a * (kx * 10 + tx)
                                + b * (ky * 10 + ty)),
                            300000 + a * (x + tx) + b * (y + ty)
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn nonlinear_surfaces_remain_bounded_symmetric_and_exact_at_nodes() {
        for field in 0..4 {
            let at = |x: i32, y: i32| match field {
                0 => x * x + y * y,
                1 => x * x - y * y,
                2 => {
                    if x < 0 {
                        0
                    } else {
                        100000
                    }
                }
                _ => {
                    if (x + y) % 2 == 0 {
                        i32::MIN
                    } else {
                        i32::MAX
                    }
                }
            };
            for y in -20_i32..31 {
                for x in -20_i32..31 {
                    let (kx, ky) = (x.div_euclid(10), y.div_euclid(10));
                    let corners = [
                        at(kx, ky),
                        at(kx + 1, ky),
                        at(kx, ky + 1),
                        at(kx + 1, ky + 1),
                    ];
                    let h = sample(x, y, at);
                    assert!(
                        (*corners.iter().min().unwrap()..=*corners.iter().max().unwrap())
                            .contains(&h)
                    );
                    assert_eq!(h, sample(y, x, |a, b| at(b, a)));
                    if x % 10 == 0 && y % 10 == 0 {
                        assert_eq!(h, at(kx, ky));
                    }
                }
            }
        }
    }
}
