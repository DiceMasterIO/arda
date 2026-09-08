//! Fixed-point physical channel coverage (`logic/04`, area realism R8–R9).

use crate::RenderError;

pub(crate) const Q: i64 = 1 << 20;
const COORD_LIMIT: i64 = 1 << 40;
const MAX_VERTICES: usize = 128;
const MAX_PIECES: usize = 4096;
pub(crate) type Point = (i64, i64);
pub(crate) type Polygon = Vec<Point>;

pub(crate) struct WorkBudget {
    remaining: u64,
}

impl WorkBudget {
    pub(crate) const fn new(limit: u64) -> Self {
        Self { remaining: limit }
    }
    pub(crate) fn charge(&mut self, units: u64) -> Result<(), RenderError> {
        self.remaining = self
            .remaining
            .checked_sub(units)
            .ok_or_else(|| invalid("channel coverage work budget exhausted"))?;
        Ok(())
    }
}

fn invalid(reason: &'static str) -> RenderError {
    RenderError::ChannelGeometry { reason }
}

fn validate(poly: &[Point]) -> Result<(), RenderError> {
    if poly.len() > MAX_VERTICES {
        return Err(invalid("polygon exceeds 128 vertices"));
    }
    if poly.iter().any(|&(x, y)| {
        !(-COORD_LIMIT..=COORD_LIMIT).contains(&x) || !(-COORD_LIMIT..=COORD_LIMIT).contains(&y)
    }) {
        return Err(invalid("channel coordinate exceeds fixed-point bounds"));
    }
    Ok(())
}

fn cross(a: Point, b: Point, p: Point) -> i128 {
    i128::from(b.0 - a.0) * i128::from(p.1 - a.1) - i128::from(b.1 - a.1) * i128::from(p.0 - a.0)
}

fn signed_area(poly: &[Point]) -> i128 {
    (0..poly.len())
        .map(|i| {
            let a = poly[i];
            let b = poly[(i + 1) % poly.len()];
            i128::from(a.0) * i128::from(b.1) - i128::from(a.1) * i128::from(b.0)
        })
        .sum()
}

pub(crate) fn area2(poly: &[Point]) -> i128 {
    signed_area(poly).abs()
}

fn nearest(n: i128, d: i128) -> i128 {
    let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
    if n >= 0 {
        (n + d / 2) / d
    } else {
        -((-n + d / 2) / d)
    }
}

fn half_plane(
    poly: &[Point],
    a: Point,
    b: Point,
    inside: bool,
    roundings: &mut u64,
    work: &mut WorkBudget,
) -> Result<Polygon, RenderError> {
    let mut out = Vec::new();
    let sign = if inside { 1 } else { -1 };
    for i in 0..poly.len() {
        work.charge(1)?;
        let p = poly[i];
        let n = poly[(i + 1) % poly.len()];
        let cp = cross(a, b, p) * sign;
        let cn = cross(a, b, n) * sign;
        if cp >= 0 {
            out.push(p);
        }
        if (cp >= 0) != (cn >= 0) {
            // Coordinates are bounded by 2^40; this product is below 2^124.
            let den = cp - cn;
            let dx = i64::try_from(nearest(i128::from(n.0 - p.0) * cp, den))
                .map_err(|_| invalid("intersection x exceeds i64"))?;
            let dy = i64::try_from(nearest(i128::from(n.1 - p.1) * cp, den))
                .map_err(|_| invalid("intersection y exceeds i64"))?;
            out.push((p.0 + dx, p.1 + dy));
            *roundings += 1;
        }
        if out.len() > MAX_VERTICES {
            return Err(invalid("clipped polygon exceeds 128 vertices"));
        }
    }
    out.dedup();
    if out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    work.charge(out.len() as u64)?;
    if out.len() < 3 || area2(&out) == 0 {
        out.clear();
    }
    Ok(out)
}

pub(crate) fn intersection(
    subject: &[Point],
    clip: &[Point],
    roundings: &mut u64,
    work: &mut WorkBudget,
) -> Result<Polygon, RenderError> {
    work.charge((subject.len() + clip.len() + 1) as u64)?;
    validate(subject)?;
    validate(clip)?;
    let mut p = subject.to_vec();
    for i in 0..clip.len() {
        p = half_plane(
            &p,
            clip[i],
            clip[(i + 1) % clip.len()],
            true,
            roundings,
            work,
        )?;
    }
    Ok(p)
}

fn subtract(
    subject: &[Point],
    clip: &[Point],
    roundings: &mut u64,
    work: &mut WorkBudget,
) -> Result<Vec<Polygon>, RenderError> {
    let mut remainder = subject.to_vec();
    let mut pieces = Vec::new();
    for i in 0..clip.len() {
        if remainder.is_empty() {
            break;
        }
        let a = clip[i];
        let b = clip[(i + 1) % clip.len()];
        let part = half_plane(&remainder, a, b, false, roundings, work)?;
        if !part.is_empty() {
            pieces.push(part);
        }
        remainder = half_plane(&remainder, a, b, true, roundings, work)?;
    }
    Ok(pieces)
}

pub(crate) struct Coverage {
    pub alpha: u16,
    pub roundings: u64,
    pub pieces: usize,
    pub maximum_discharge: u64,
}

pub(crate) fn coverage<'a>(
    polygons: impl IntoIterator<Item = (&'a Polygon, u64)>,
    x: u32,
    y: u32,
    work: &mut WorkBudget,
) -> Result<Coverage, RenderError> {
    coverage_limited(polygons, x, y, MAX_PIECES, work)
}

fn coverage_limited<'a>(
    polygons: impl IntoIterator<Item = (&'a Polygon, u64)>,
    x: u32,
    y: u32,
    limit: usize,
    work: &mut WorkBudget,
) -> Result<Coverage, RenderError> {
    if limit == 0 || limit > MAX_PIECES {
        return Err(invalid("invalid polygon union budget"));
    }
    if x >= 4096 || y >= 4096 {
        return Err(invalid("pixel leaves the maximum area raster"));
    }
    let (x, y) = (i64::from(x), i64::from(y));
    let rect = rectangle(x * Q, y * Q, (x + 1) * Q, (y + 1) * Q);
    let mut disjoint: Vec<Polygon> = Vec::new();
    let mut roundings = 0;
    let mut maximum_discharge = 0;
    let mut full = false;
    for (poly, discharge) in polygons {
        let clipped = intersection(poly, &rect, &mut roundings, work)?;
        if clipped.is_empty() {
            continue;
        }
        maximum_discharge = maximum_discharge.max(discharge);
        if full {
            continue;
        }
        work.charge(clipped.len() as u64)?;
        if area2(&clipped) >= 2 * i128::from(Q) * i128::from(Q) {
            full = true;
            disjoint.clear();
            continue;
        }
        let mut parts = vec![clipped];
        for old in &disjoint {
            let mut next = Vec::new();
            for p in &parts {
                work.charge(1)?;
                for part in subtract(p, old, &mut roundings, work)? {
                    if next.len() + disjoint.len() >= limit {
                        return Err(invalid("channel union exceeds 4096 pieces per pixel"));
                    }
                    next.push(part);
                }
            }
            parts = next;
            if parts.is_empty() {
                break;
            }
        }
        if disjoint.len() + parts.len() > limit {
            return Err(invalid("channel union exceeds 4096 pieces per pixel"));
        }
        disjoint.extend(parts);
    }
    work.charge(disjoint.iter().map(|p| p.len() as u64).sum())?;
    let total: i128 = disjoint.iter().map(|p| area2(p)).sum();
    let alpha = if full {
        u16::MAX
    } else {
        u16::try_from(nearest(total * 65_535, 2 * i128::from(Q) * i128::from(Q)).clamp(0, 65_535))
            .map_err(|_| invalid("coverage leaves u16 range"))?
    };
    Ok(Coverage {
        alpha,
        roundings,
        pieces: if full { 1 } else { disjoint.len() },
        maximum_discharge,
    })
}

/// A direction-free regular 32-gon inscribed in the terminal's physical width.
/// Integer unit vectors are rounded to Q; radial error is below 1/Q and the
/// ideal polygon's circle-area deficit is below 0.65%; final vertices round to Q20.
/// No invented channel step exists.
pub(crate) fn terminal_footprint(center: Point, width: i64) -> Result<Polygon, RenderError> {
    validate(&[center])?;
    if width <= 0 {
        return Err(invalid("terminal width is not positive"));
    }
    const OCTANT: [Point; 5] = [
        (1_048_576, 0),
        (1_028_428, 204_567),
        (968_758, 401_273),
        (871_859, 582_558),
        (741_455, 741_455),
    ];
    let mut polygon = Vec::with_capacity(32);
    for quadrant in 0..4 {
        for k in 0..8 {
            let (x, y) = if k <= 4 {
                OCTANT[k]
            } else {
                let (a, b) = OCTANT[8 - k];
                (b, a)
            };
            let (x, y) = match quadrant {
                0 => (x, y),
                1 => (-y, x),
                2 => (-x, -y),
                _ => (y, -x),
            };
            let dx = i64::try_from(nearest(
                i128::from(x) * i128::from(width),
                2 * i128::from(Q),
            ))
            .map_err(|_| invalid("terminal width exceeds fixed-point bounds"))?;
            let dy = i64::try_from(nearest(
                i128::from(y) * i128::from(width),
                2 * i128::from(Q),
            ))
            .map_err(|_| invalid("terminal width exceeds fixed-point bounds"))?;
            polygon.push((
                center
                    .0
                    .checked_add(dx)
                    .ok_or_else(|| invalid("terminal x overflow"))?,
                center
                    .1
                    .checked_add(dy)
                    .ok_or_else(|| invalid("terminal y overflow"))?,
            ));
        }
    }
    validate(&polygon)?;
    Ok(polygon)
}

pub(crate) fn strip(
    a: Point,
    b: Point,
    width_a: i64,
    width_b: i64,
) -> Result<(Polygon, [Point; 2], [Point; 2]), RenderError> {
    validate(&[a, b])?;
    if a == b || width_a < 0 || width_b < 0 {
        return Err(invalid("channel edge has zero length or negative width"));
    }
    let dx = (b.0 - a.0).signum();
    let dy = (b.1 - a.1).signum();
    if dx != 0 && dy != 0 && (b.0 - a.0).abs() != (b.1 - a.1).abs() {
        return Err(invalid("channel edge is not a saved D8 step"));
    }
    let inv = if dx != 0 && dy != 0 { 741_455 } else { Q };
    let normal = |width: i64| -> Result<Point, RenderError> {
        let magnitude = i64::try_from(i128::from(width) * i128::from(inv) / (2 * i128::from(Q)))
            .map_err(|_| invalid("channel normal exceeds i64"))?;
        Ok((-dy * magnitude, dx * magnitude))
    };
    let na = normal(width_a)?;
    let nb = normal(width_b)?;
    let a_cap = [(a.0 + na.0, a.1 + na.1), (a.0 - na.0, a.1 - na.1)];
    let b_cap = [(b.0 + nb.0, b.1 + nb.1), (b.0 - nb.0, b.1 - nb.1)];
    let mut p = vec![a_cap[0], b_cap[0], b_cap[1], a_cap[1]];
    validate(&p)?;
    if signed_area(&p) < 0 {
        p.reverse();
    }
    Ok((p, a_cap, b_cap))
}

pub(crate) fn rectangle(x0: i64, y0: i64, x1: i64, y1: i64) -> Polygon {
    vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

pub(crate) fn hull(mut points: Vec<Point>) -> Result<Polygon, RenderError> {
    validate(&points)?;
    points.sort_unstable();
    points.dedup();
    if points.len() < 3 {
        return Ok(Vec::new());
    }
    let mut out: Polygon = Vec::new();
    for &p in &points {
        while out.len() >= 2 && cross(out[out.len() - 2], out[out.len() - 1], p) <= 0 {
            out.pop();
        }
        out.push(p);
    }
    let lower = out.len();
    for &p in points.iter().rev().skip(1) {
        while out.len() > lower && cross(out[out.len() - 2], out[out.len() - 1], p) <= 0 {
            out.pop();
        }
        out.push(p);
    }
    out.pop();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn integral(polygons: &[Polygon]) -> (f64, f64) {
        let mut total = 0_u64;
        let mut roundings = 0_u64;
        let mut touched = 0_u64;
        for y in 0..8 {
            for x in 0..8 {
                let c = coverage(
                    polygons.iter().map(|p| (p, 1000)),
                    x,
                    y,
                    &mut WorkBudget::new(250_000_000),
                )
                .unwrap();
                total += u64::from(c.alpha);
                roundings += c.roundings;
                touched += u64::from(c.alpha > 0);
            }
        }
        (
            total as f64 / 65_535.,
            4. * roundings as f64 / Q as f64 + touched as f64 / (2. * 65_535.),
        )
    }

    #[test]
    fn terminal_point_numeric_area_and_union_have_no_fictitious_step() {
        for width in [Q / 125, Q / 10, Q, 2 * Q] {
            let p = terminal_footprint((4 * Q, 4 * Q), width).unwrap();
            assert_eq!(p.len(), 32);
            let exact = area2(&p) as f64 / (2. * Q as f64 * Q as f64);
            let circle = std::f64::consts::PI * (width as f64 / Q as f64).powi(2) / 4.;
            assert!(exact > circle * 0.9934 && exact < circle * 0.9938);
            let (covered, tolerance) = integral(std::slice::from_ref(&p));
            assert!((covered - exact).abs() <= tolerance);
            let (duplicate, duplicate_tolerance) = integral(&[p.clone(), p]);
            assert!((duplicate - exact).abs() <= duplicate_tolerance);
        }
        let p = terminal_footprint((4 * Q, 4 * Q), Q).unwrap();
        let edge = strip((4 * Q, 4 * Q), (6 * Q, 4 * Q), Q, Q).unwrap().0;
        let (union, tolerance) = integral(&[p.clone(), edge]);
        let disk = area2(&p) as f64 / (2. * Q as f64 * Q as f64);
        assert!((union - (2. + disk / 2.)).abs() <= tolerance);
        assert!(terminal_footprint((0, 0), 0).is_err());
        assert!(terminal_footprint((COORD_LIMIT, 0), Q).is_err());
    }

    #[test]
    fn physical_subpixel_width_survives_without_a_visibility_mark() {
        for dm in [8, 10, 20, 100, 1000, 65_535, 230_650] {
            let width = dm * Q / 1000;
            let p = rectangle(Q, 4 * Q - width / 2, 7 * Q, 4 * Q + width / 2);
            let (got, tolerance) = integral(&[p]);
            let expected = 6. * (width as f64 / Q as f64).min(8.);
            assert!(got > 0.);
            assert!((got - expected).abs() <= tolerance + 0.00001);
        }
    }

    #[test]
    fn variable_width_and_diagonal_area_match_physical_geometry() {
        for (wa, wb) in [(Q / 100, Q / 100), (Q / 10, Q / 2), (Q / 2, Q)] {
            for (b, length) in [((7 * Q, Q), 6.), ((7 * Q, 7 * Q), 6. * 2_f64.sqrt())] {
                let (p, _, _) = strip((Q, Q), b, wa, wb).unwrap();
                let exact = area2(&p) as f64 / (2. * Q as f64 * Q as f64);
                let physical = length * (wa + wb) as f64 / (2. * Q as f64);
                let (got, tolerance) = integral(&[p]);
                assert!((exact - physical).abs() < 0.00003);
                assert!((got - exact).abs() <= tolerance);
            }
        }
    }

    #[test]
    fn widest_diagonal_channels_have_an_explicit_normal_approximation_bound() {
        let root_two = 2_f64.sqrt();
        let relative_normal_error = (root_two * 741_455. / Q as f64 - 1.).abs();
        for dm in [10_i64, 400, 65_535, 230_650] {
            for scale in [1_i64, 8] {
                let width = dm * scale * Q / 1000;
                let length = scale as f64 * root_two;
                let ideal = length * dm as f64 * scale as f64 / 1000.;
                let bound =
                    ideal * relative_normal_error + (2. * root_two + 1.) * length / Q as f64;
                for sign in [-1_i64, 1] {
                    let (polygon, _, _) =
                        strip((0, 0), (scale * Q, sign * scale * Q), width, width).unwrap();
                    let actual = area2(&polygon) as f64 / (2. * Q as f64 * Q as f64);
                    assert!(
                        (actual - ideal).abs() <= bound,
                        "width={dm}dm scale={scale} error={} bound={bound}",
                        (actual - ideal).abs()
                    );
                }
            }
        }
    }

    #[test]
    fn confluence_union_counts_overlap_once() {
        let a = rectangle(Q, 3 * Q, 7 * Q, 4 * Q);
        let b = rectangle(3 * Q, Q, 4 * Q, 7 * Q);
        let (got, tolerance) = integral(&[a.clone(), b.clone()]);
        assert!((got - 11.).abs() <= tolerance);
        assert_eq!(integral(&[b, a]).0, got);
        let (p, _, _) = strip((Q, Q), (7 * Q, 7 * Q), Q / 10, Q / 10).unwrap();
        assert_eq!(
            integral(std::slice::from_ref(&p)).0,
            integral(&[p.clone(), p]).0
        );
    }

    #[test]
    fn diagonal_crossing_and_bevel_have_independent_analytic_areas() {
        let width = Q / 2;
        let (a, _, _) = strip((Q, Q), (7 * Q, 7 * Q), width, width).unwrap();
        let (b, _, _) = strip((Q, 7 * Q), (7 * Q, Q), width, width).unwrap();
        let (got, tolerance) = integral(&[a, b]);
        assert!((got - (6. * 2_f64.sqrt() - 0.25)).abs() <= tolerance + 0.00003);
        let (a, _, a_cap) = strip((Q, 4 * Q), (4 * Q, 4 * Q), width, width).unwrap();
        let (b, b_cap, _) = strip((4 * Q, 4 * Q), (4 * Q, 7 * Q), width, width).unwrap();
        let join = hull(a_cap.into_iter().chain(b_cap).collect()).unwrap();
        let (got, tolerance) = integral(&[a, b, join]);
        assert!((got - (3. - 0.25 / 8.)).abs() <= tolerance);
    }

    #[test]
    fn seam_clips_partition_one_global_channel() {
        let (p, _, _) = strip((Q, Q), (7 * Q, 7 * Q), Q / 10, Q / 10).unwrap();
        let mut rounds = 0;
        let left = intersection(
            &p,
            &rectangle(0, 0, 4 * Q, 8 * Q),
            &mut rounds,
            &mut WorkBudget::new(250_000_000),
        )
        .unwrap();
        let right = intersection(
            &p,
            &rectangle(4 * Q, 0, 8 * Q, 8 * Q),
            &mut rounds,
            &mut WorkBudget::new(250_000_000),
        )
        .unwrap();
        assert_eq!(area2(&left) + area2(&right), area2(&p));
    }

    #[test]
    fn colour_uses_only_geometry_that_intersects_the_pixel() {
        let low = rectangle(0, 0, Q / 2, Q);
        let absent = rectangle(2 * Q, 2 * Q, 3 * Q, 3 * Q);
        let c = coverage(
            [(&low, 1000), (&absent, 1_000_000)],
            0,
            0,
            &mut WorkBudget::new(250_000_000),
        )
        .unwrap();
        assert_eq!(c.maximum_discharge, 1000);
        assert_eq!(c.alpha, 32_768);
    }

    #[test]
    fn bounds_and_fragment_limits_are_errors() {
        let too_far = rectangle(COORD_LIMIT, 0, COORD_LIMIT + 1, Q);
        assert!(coverage([(&too_far, 1)], 0, 0, &mut WorkBudget::new(250_000_000)).is_err());
        assert!(strip((0, 0), (Q, Q / 2), Q, Q).is_err());
        let polygons: Vec<_> = (0..4)
            .map(|i| rectangle(i * Q / 4, 0, (i * 2 + 1) * Q / 8, Q))
            .collect();
        assert!(coverage_limited(
            polygons.iter().map(|p| (p, 1)),
            0,
            0,
            3,
            &mut WorkBudget::new(250_000_000)
        )
        .is_err());
        assert!(coverage_limited(
            polygons.iter().map(|p| (p, 1)),
            0,
            0,
            4,
            &mut WorkBudget::new(250_000_000)
        )
        .is_ok());
    }
    #[test]
    fn exact_fraction_oracle_is_within_one_alpha_quantum() {
        #[derive(serde::Deserialize)]
        struct Case {
            polygons: Vec<Polygon>,
            nearest_alpha: u16,
            exact_area: f64,
        }
        let cases: Vec<Case> =
            serde_json::from_str(include_str!("coverage_fixtures.json")).unwrap();
        assert_eq!(cases.len(), 96);
        for (i, case) in cases.iter().enumerate() {
            let c = coverage(
                case.polygons.iter().map(|p| (p, 0)),
                0,
                0,
                &mut WorkBudget::new(250_000_000),
            )
            .unwrap();
            assert!(c.alpha.abs_diff(case.nearest_alpha) <= 1, "fixture {i}");
            assert!(
                (f64::from(c.alpha) / 65_535. - case.exact_area).abs() <= 1. / 65_535.,
                "fixture {i}"
            );
        }
    }

    #[test]
    fn disjoint_comparisons_stop_inside_the_coverage_budget() {
        let polygons: Vec<_> = (0..128)
            .map(|i| rectangle(i * Q / 128, 0, (i * 2 + 1) * Q / 256, Q))
            .collect();
        let mut work = WorkBudget::new(500);
        assert!(coverage(polygons.iter().map(|p| (p, 1)), 0, 0, &mut work).is_err());
        assert!(work.remaining < 100);
    }
}
