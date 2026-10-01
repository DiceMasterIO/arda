//! Channel polygon primitives in Q20 fixed point: terminal footprints,
//! tapered strips, axis rectangles and convex hulls.

use super::{cross, invalid, nearest, signed_area, validate, Point, Polygon, Q};
use crate::RenderError;

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

/// A strip in any direction (recipe-5 thalweg vertices, logic/04
/// §atlas-formed rivers): exact integer unit normal from the edge length.
pub(crate) fn strip_free(
    a: Point,
    b: Point,
    width_a: i64,
    width_b: i64,
) -> Result<(Polygon, [Point; 2], [Point; 2]), RenderError> {
    validate(&[a, b])?;
    if a == b || width_a < 0 || width_b < 0 {
        return Err(invalid("channel edge has zero length or negative width"));
    }
    let (ex, ey) = (i128::from(b.0 - a.0), i128::from(b.1 - a.1));
    let length = i128::try_from((ex * ex + ey * ey).unsigned_abs().isqrt())
        .map_err(|_| invalid("channel length exceeds i128"))?;
    let normal = |width: i64| -> Result<Point, RenderError> {
        let w = i128::from(width);
        Ok((
            i64::try_from(-ey * w / (2 * length))
                .map_err(|_| invalid("channel normal exceeds i64"))?,
            i64::try_from(ex * w / (2 * length))
                .map_err(|_| invalid("channel normal exceeds i64"))?,
        ))
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
