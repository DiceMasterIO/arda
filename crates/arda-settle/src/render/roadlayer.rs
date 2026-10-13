//! Roads by class with casings (the road layer of the overlay): smoothed
//! centrelines prepared once, then painted onto any window of the image.

use super::mask::{chaikin, Mask};
use super::View;
use crate::canvas::{Canvas, Rgb};
use crate::output::RoadsFile;
use crate::roads::RoadClass;

use super::layers::INK;

/// Road styles: (casing width, casing colour, fill width, fill colour,
/// dash on/off) at scale 1.
pub fn road_style(class: RoadClass) -> (f32, Rgb, f32, Rgb, Option<(f32, f32)>) {
    match class {
        RoadClass::Highway => (4.4, [74, 30, 20], 2.6, [206, 64, 40], None),
        RoadClass::Road => (3.2, [86, 56, 34], 1.9, [248, 222, 158], None),
        RoadClass::Track => (0.0, INK, 1.15, [112, 82, 56], None),
        RoadClass::Footpath | RoadClass::None => (0.0, INK, 0.95, [112, 82, 56], Some((3.0, 2.6))),
    }
}

/// One smoothed road centreline in page pixels, with its bounding box.
#[derive(Debug, Clone)]
pub struct Line {
    /// Smoothed points.
    pub pts: Vec<(f32, f32)>,
    /// `[x0, y0, x1, y1]` of the points.
    pub bbox: [f32; 4],
}

/// Road centrelines by class, drawn lowest class first.
#[derive(Debug, Clone, Default)]
pub struct RoadLines {
    /// Footpaths, tracks, roads and highways, in that order.
    pub by_class: [Vec<Line>; 4],
}

/// Road classes in drawing order (`RoadLines::by_class` index).
pub const CLASSES: [RoadClass; 4] = [
    RoadClass::Footpath,
    RoadClass::Track,
    RoadClass::Road,
    RoadClass::Highway,
];

/// The smoothed centrelines of every road in page pixels.
#[must_use]
pub fn road_lines(v: &View, roads: &RoadsFile) -> RoadLines {
    let mut out = RoadLines::default();
    for (k, &class) in CLASSES.iter().enumerate() {
        out.by_class[k] = roads
            .roads
            .iter()
            .filter(|r| r.class == class)
            .flat_map(|r| r.segments.iter())
            .map(|seg| {
                let pts = chaikin(&seg.iter().map(|&m| v.px(m)).collect::<Vec<_>>(), 3);
                let mut bbox = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
                for p in &pts {
                    bbox = [
                        bbox[0].min(p.0),
                        bbox[1].min(p.1),
                        bbox[2].max(p.0),
                        bbox[3].max(p.1),
                    ];
                }
                Line { pts, bbox }
            })
            .collect();
    }
    out
}

/// Roads by class: lanes first, then every casing, then fills from road
/// to highway, so junctions merge.
pub fn roads(c: &mut Canvas, v: &View, roads: &RoadsFile) {
    paint_roads(c, &road_lines(v, roads), v.s, (0, 0), &CLASSES);
}

/// Paints the `classes` of prepared `lines` at symbol scale `s` onto a
/// canvas that is the window of the line space starting at pixel `origin`.
/// Lines keep their own coordinates (masks are placed in them), so every
/// window of one image gets the very pixels the whole image gets.
pub fn paint_roads(
    c: &mut Canvas,
    lines: &RoadLines,
    s: f32,
    origin: (i64, i64),
    classes: &[RoadClass],
) {
    let mut m = Mask::new(origin.0, origin.1, c.width, c.height);
    let rect = [
        origin.0 as f32,
        origin.1 as f32,
        (origin.0 + c.width as i64) as f32,
        (origin.1 + c.height as i64) as f32,
    ];
    let pad = 6.0 * s + 2.0;
    let near = |l: &&Line| {
        l.bbox[0] <= rect[2] + pad
            && l.bbox[2] >= rect[0] - pad
            && l.bbox[1] <= rect[3] + pad
            && l.bbox[3] >= rect[1] - pad
    };
    let lines_of = |class: RoadClass| -> Vec<&Line> {
        CLASSES
            .iter()
            .position(|&k| k == class)
            .filter(|_| classes.contains(&class))
            .map_or_else(Vec::new, |k| {
                lines.by_class[k].iter().filter(near).collect()
            })
    };
    for class in [RoadClass::Footpath, RoadClass::Track] {
        let (_, _, fw, fc, dash) = road_style(class);
        m.clear();
        for l in lines_of(class) {
            match dash {
                Some((on, off)) => {
                    let mut ph = 0.0;
                    m.dashed(&l.pts, fw * s, on * s, off * s, &mut ph);
                }
                None => m.polyline(&l.pts, fw * s),
            }
        }
        c.paint_at(&m, fc, 0.9, origin);
    }
    let trunk = [RoadClass::Road, RoadClass::Highway];
    let paths: Vec<Vec<&Line>> = trunk.iter().map(|&k| lines_of(k)).collect();
    for (k, &class) in trunk.iter().enumerate() {
        let (cw, cc, _, _, _) = road_style(class);
        m.clear();
        for l in &paths[k] {
            m.polyline(&l.pts, cw * s);
        }
        c.paint_at(&m, cc, 0.95, origin);
    }
    for (k, &class) in trunk.iter().enumerate() {
        let (_, _, fw, fc, _) = road_style(class);
        m.clear();
        for l in &paths[k] {
            m.polyline(&l.pts, fw * s);
        }
        c.paint_at(&m, fc, 1.0, origin);
    }
}
