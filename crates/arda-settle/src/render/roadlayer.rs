//! Roads by class with casings (the road layer of the overlay): smoothed
//! centrelines prepared once, then painted onto any window of the image.

use super::mask::{chaikin, Mask};
use super::View;
use crate::canvas::{Canvas, Rgb};
use crate::output::{RoadsFile, SettlementsFile};
use crate::roads::RoadClass;
use std::collections::BTreeSet;

use super::layers::INK;

/// How roads are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoadStyle {
    /// Ivory roads in a thin dark sepia casing, width by class; tracks
    /// dashed and footpaths dotted in sepia over a faint pale halo (the
    /// political map, logic/16 §political-map).
    #[default]
    Ivory,
    /// Red highways and yellow roads in brown casings, solid brown tracks
    /// and dashed footpaths (the society overlay of overviews).
    Classic,
}

/// One class's look at scale 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoadLook {
    /// Casing width (0 for none), pixels.
    pub casing_w: f32,
    /// Casing colour.
    pub casing: Rgb,
    /// Fill width, pixels.
    pub fill_w: f32,
    /// Fill colour.
    pub fill: Rgb,
    /// Dash on/off, pixels; `None` is solid.
    pub dash: Option<(f32, f32)>,
    /// Whether a pale halo goes under the line (lanes of the ivory style).
    pub halo: bool,
}

/// Ivory fill of the cased style.
const IVORY: Rgb = [255, 250, 228];
/// Dark sepia of casings and lanes in the cased style.
const SEPIA: Rgb = [62, 38, 22];
/// Pale halo under lanes in the cased style.
pub const LANE_HALO: Rgb = [255, 248, 226];
/// Opacity of the lane halo.
pub const LANE_HALO_ALPHA: f32 = 0.22;
/// How much wider than its lane the halo is, at scale 1.
pub const LANE_HALO_EXTRA: f32 = 1.2;

/// The look of `class` in `style`.
#[must_use]
pub const fn road_style(style: RoadStyle, class: RoadClass) -> RoadLook {
    let (casing_w, casing, fill_w, fill, dash, halo) = match (style, class) {
        (RoadStyle::Classic, RoadClass::Highway) => {
            (4.4, [74, 30, 20], 2.6, [206, 64, 40], None, false)
        }
        (RoadStyle::Classic, RoadClass::Road) => {
            (3.2, [86, 56, 34], 1.9, [248, 222, 158], None, false)
        }
        (RoadStyle::Classic, RoadClass::Track) => (0.0, INK, 1.15, [112, 82, 56], None, false),
        (RoadStyle::Classic, RoadClass::Footpath | RoadClass::None) => {
            (0.0, INK, 0.95, [112, 82, 56], Some((3.0, 2.6)), false)
        }
        (RoadStyle::Ivory, RoadClass::Highway) => (6.0, SEPIA, 3.8, IVORY, None, false),
        (RoadStyle::Ivory, RoadClass::Road) => (3.2, SEPIA, 1.7, IVORY, None, false),
        (RoadStyle::Ivory, RoadClass::Track) => (0.0, SEPIA, 1.3, SEPIA, Some((5.5, 3.2)), true),
        (RoadStyle::Ivory, RoadClass::Footpath | RoadClass::None) => {
            (0.0, SEPIA, 1.2, SEPIA, Some((1.2, 2.8)), true)
        }
    };
    RoadLook {
        casing_w,
        casing,
        fill_w,
        fill,
        dash,
        halo,
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

impl Line {
    /// A line through `pts` (already smoothed), with its bounding box.
    #[must_use]
    pub fn new(pts: Vec<(f32, f32)>) -> Self {
        let mut bbox = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
        for p in &pts {
            bbox = [
                bbox[0].min(p.0),
                bbox[1].min(p.1),
                bbox[2].max(p.0),
                bbox[3].max(p.1),
            ];
        }
        Self { pts, bbox }
    }
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

/// The ids of every hamlet: roads built from one of them (every track)
/// serve only that hamlet and are left off with it at world scale
/// ([`super::scale::shows_hamlets`]).
#[must_use]
pub fn hamlet_ids(st: &SettlementsFile) -> BTreeSet<u64> {
    st.settlements
        .iter()
        .filter(|s| s.tier == crate::model::Tier::Hamlet)
        .map(|s| s.id.get())
        .collect()
}

/// The smoothed centrelines of every road in page pixels, without the
/// roads built from a settlement in `skip_from` (hamlets at world scale;
/// a road is built from the place it connects, so these serve nothing
/// else).
#[must_use]
pub fn road_lines(v: &View, roads: &RoadsFile, skip_from: &BTreeSet<u64>) -> RoadLines {
    let mut out = RoadLines::default();
    for (k, &class) in CLASSES.iter().enumerate() {
        out.by_class[k] = roads
            .roads
            .iter()
            .filter(|r| r.class == class && !skip_from.contains(&r.from))
            .flat_map(|r| r.segments.iter())
            .map(|seg| {
                Line::new(chaikin(
                    &seg.iter().map(|&m| v.px(m)).collect::<Vec<_>>(),
                    3,
                ))
            })
            .collect();
    }
    out
}

/// Paints the `classes` of prepared `lines` in `style` at symbol scale `s`
/// onto a canvas that is the window of the line space starting at pixel
/// `origin`: lanes first, then every casing, then fills from road to
/// highway, so junctions merge. Lines keep their own coordinates (masks
/// are placed in them), so every window of one image gets the very pixels
/// the whole image gets.
pub fn paint_roads(
    c: &mut Canvas,
    lines: &RoadLines,
    (s, style): (f32, RoadStyle),
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
    let lane = |m: &mut Mask, l: &Line, w: f32, dash: Option<(f32, f32)>| match dash {
        Some((on, off)) => {
            let mut ph = 0.0;
            m.dashed(&l.pts, w * s, on * s, off * s, &mut ph);
        }
        None => m.polyline(&l.pts, w * s),
    };
    for class in [RoadClass::Footpath, RoadClass::Track] {
        let look = road_style(style, class);
        let mine = lines_of(class);
        if look.halo {
            m.clear();
            let halo_dash = look.dash.map(|(on, off)| (on + 0.8, (off - 0.8).max(0.0)));
            for l in &mine {
                lane(&mut m, l, look.fill_w + LANE_HALO_EXTRA, halo_dash);
            }
            c.paint_at(&m, LANE_HALO, LANE_HALO_ALPHA, origin);
        }
        m.clear();
        for l in &mine {
            lane(&mut m, l, look.fill_w, look.dash);
        }
        let opacity = if look.halo { 0.95 } else { 0.9 };
        c.paint_at(&m, look.fill, opacity, origin);
    }
    let trunk = [RoadClass::Road, RoadClass::Highway];
    let paths: Vec<Vec<&Line>> = trunk.iter().map(|&k| lines_of(k)).collect();
    for (k, &class) in trunk.iter().enumerate() {
        let look = road_style(style, class);
        m.clear();
        for l in &paths[k] {
            m.polyline(&l.pts, look.casing_w * s);
        }
        c.paint_at(&m, look.casing, 0.95, origin);
    }
    for (k, &class) in trunk.iter().enumerate() {
        let look = road_style(style, class);
        m.clear();
        for l in &paths[k] {
            m.polyline(&l.pts, look.fill_w * s);
        }
        c.paint_at(&m, look.fill, 1.0, origin);
    }
}
