//! Fixed-point physical channel coverage (`logic/04`, area realism R8–R9).

use crate::RenderError;

mod shapes;
pub(crate) use shapes::{hull, rectangle, strip, strip_free, terminal_footprint};

pub(crate) const Q: i64 = 1 << 20;
const COORD_LIMIT: i64 = 1 << 40;
const MAX_VERTICES: usize = 128;
const MAX_PIECES: usize = 4096;
// Overview-only: subdivide before the union accumulates costly fragment lists.
const OVERVIEW_SUBDIVIDE_PIECES: usize = 64;
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
    if x >= crate::ImageQuality::MAX || y >= crate::ImageQuality::MAX {
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

// The opt-in world overview can place many wide saved strips in one fitted
// pixel. The ordinary exact union above may fragment even 14 overlapping
// polygons into 64 pieces. Recursively subdividing that pixel
// keeps the same Q20 clipping/union operation; a finite depth/work refusal
// remains instead of omitting channels or silently clamping coverage.
struct RectUnion {
    area2: i128,
    maximum_discharge: u64,
    roundings: u64,
    pieces: usize,
}
fn union_rect(
    polygons: &[(&Polygon, u64)],
    rect: &Polygon,
    work: &mut WorkBudget,
) -> Result<Option<RectUnion>, RenderError> {
    let mut disjoint: Vec<Polygon> = Vec::new();
    let mut roundings = 0_u64;
    let mut maximum_discharge = 0_u64;
    let mut full = false;
    let rect_area2 = area2(rect);
    for &(poly, discharge) in polygons {
        let clipped = intersection(poly, rect, &mut roundings, work)?;
        if clipped.is_empty() {
            continue;
        }
        maximum_discharge = maximum_discharge.max(discharge);
        if full {
            continue;
        }
        work.charge(clipped.len() as u64)?;
        if area2(&clipped) >= rect_area2 {
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
                    if next.len() + disjoint.len() >= OVERVIEW_SUBDIVIDE_PIECES {
                        return Ok(None);
                    }
                    next.push(part);
                }
            }
            parts = next;
            if parts.is_empty() {
                break;
            }
        }
        if disjoint.len() + parts.len() >= OVERVIEW_SUBDIVIDE_PIECES {
            return Ok(None);
        }
        disjoint.extend(parts);
    }
    work.charge(disjoint.iter().map(|p| p.len() as u64).sum())?;
    Ok(Some(RectUnion {
        area2: if full {
            rect_area2
        } else {
            disjoint.iter().map(|p| area2(p)).sum()
        },
        maximum_discharge,
        roundings,
        pieces: if full { 1 } else { disjoint.len() },
    }))
}
fn union_subdivided(
    polygons: &[(&Polygon, u64)],
    bounds: [i64; 4],
    depth: u8,
    work: &mut WorkBudget,
) -> Result<RectUnion, RenderError> {
    let [x0, y0, x1, y1] = bounds;
    let rect = rectangle(x0, y0, x1, y1);
    if let Some(result) = union_rect(polygons, &rect, work)? {
        return Ok(result);
    }
    if depth == 6 {
        return Err(invalid(
            "overview channel union exceeds bounded Q20 subdivision",
        ));
    }
    let mx = (x0 + x1) / 2;
    let my = (y0 + y1) / 2;
    let mut total = RectUnion {
        area2: 0,
        maximum_discharge: 0,
        roundings: 0,
        pieces: 0,
    };
    for quadrant in [
        [x0, y0, mx, my],
        [mx, y0, x1, my],
        [x0, my, mx, y1],
        [mx, my, x1, y1],
    ] {
        let part = union_subdivided(polygons, quadrant, depth + 1, work)?;
        total.area2 = total
            .area2
            .checked_add(part.area2)
            .ok_or_else(|| invalid("overview union area overflow"))?;
        total.maximum_discharge = total.maximum_discharge.max(part.maximum_discharge);
        total.roundings = total
            .roundings
            .checked_add(part.roundings)
            .ok_or_else(|| invalid("overview union work overflow"))?;
        total.pieces = total
            .pieces
            .checked_add(part.pieces)
            .ok_or_else(|| invalid("overview union piece count overflow"))?;
    }
    Ok(total)
}
/// Bounded deterministic Q20 polygon coverage for dense Atlas overview channels.
/// Segment intersections are rounded to Q20, so this is not an unrounded real-number union.
/// Subdivides only pixels whose ordinary fixed-point polygon union fragments
/// at its 64-piece overview trigger; all paths retain typed work/depth failures.
pub(crate) fn coverage_overview<'a>(
    polygons: impl IntoIterator<Item = (&'a Polygon, u64)>,
    x: u32,
    y: u32,
    work: &mut WorkBudget,
) -> Result<Coverage, RenderError> {
    if x >= crate::ImageQuality::MAX || y >= crate::ImageQuality::MAX {
        return Err(invalid("overview channel pixel exceeds supported axis"));
    }
    let input: Vec<_> = polygons.into_iter().collect();
    let x0 = i64::from(x) * Q;
    let y0 = i64::from(y) * Q;
    let result = union_subdivided(&input, [x0, y0, x0 + Q, y0 + Q], 0, work)?;
    let alpha = u16::try_from(
        nearest(result.area2 * 65_535, 2 * i128::from(Q) * i128::from(Q)).clamp(0, 65_535),
    )
    .map_err(|_| invalid("overview channel coverage leaves u16 range"))?;
    Ok(Coverage {
        alpha,
        roundings: result.roundings,
        pieces: result.pieces,
        maximum_discharge: result.maximum_discharge,
    })
}
#[cfg(test)]
mod overview_dense_tests {
    use super::*;
    fn check_fixture(raw: &str, expected_alpha: u16, expected_q: u64) {
        let json: serde_json::Value = serde_json::from_str(raw).unwrap();
        assert_eq!(json["q"].as_i64(), Some(Q));
        let x = u32::try_from(json["pixel"][0].as_u64().unwrap()).unwrap();
        let y = u32::try_from(json["pixel"][1].as_u64().unwrap()).unwrap();
        let polygons: Vec<(Polygon, u64)> = json["polygons"]
            .as_array()
            .unwrap()
            .iter()
            .map(|record| {
                let p = record["polygon"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|point| (point[0].as_i64().unwrap(), point[1].as_i64().unwrap()))
                    .collect();
                (p, record["discharge"].as_u64().unwrap())
            })
            .collect();
        let mut work = WorkBudget::new(250_000_000);
        let result =
            coverage_overview(polygons.iter().map(|(p, q)| (p, *q)), x, y, &mut work).unwrap();
        assert!(result.alpha.abs_diff(expected_alpha) <= 2);
        assert_eq!(result.maximum_discharge, expected_q);
    }
    #[test]
    fn saved_dense_14_polygon_pixels_match_independent_geos_to_q20_tolerance() {
        check_fixture(
            include_str!("overview/fixtures/pixel-142-17.json"),
            2429,
            166,
        );
        check_fixture(
            include_str!("overview/fixtures/pixel-301-474.json"),
            43178,
            88715,
        );
    }
}

#[cfg(test)]
mod tests;
