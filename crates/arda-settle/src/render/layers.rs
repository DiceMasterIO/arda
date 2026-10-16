//! Map layers under the labels: land use, realm tints and borders, roads
//! by class with casings, and settlement, mine, pass and peak symbols.

use super::labels::{Placer, Rect};
use super::mask::{chaikin, Mask};
use super::palette::Palette;
pub use super::roadlayer::{
    hamlet_ids, paint_roads, road_lines, road_style, Line, RoadLines, RoadLook, RoadStyle, CLASSES,
};
use super::View;
use crate::canvas::{Canvas, Rgb};
use crate::landuse::code;
use crate::model::Tier;
use crate::output::{RealmsFile, RoadsFile, SettlementsFile};
use std::collections::BTreeSet;

/// Ink for symbols and ordinary text.
pub const INK: Rgb = [38, 28, 22];
/// Paper for halos and furniture.
pub const PAPER: Rgb = [250, 245, 230];
/// Red of towns and cities.
pub const TOWN_RED: Rgb = [176, 38, 32];

/// A darker shade of a colour for text and lines.
#[must_use]
pub fn shade(c: Rgb, k: f32) -> Rgb {
    c.map(|v| (f32::from(v) * k).round().clamp(0.0, 255.0) as u8)
}

/// How a land-use code tints the relief: `(multiply, colour, opacity)`.
#[must_use]
pub fn land_tone(code_: u8) -> Option<(bool, Rgb, f32)> {
    match code_ {
        code::ARABLE | code::ORCHARD => Some((true, [246, 226, 160], 0.55)),
        code::FALLOW => Some((true, [240, 226, 180], 0.45)),
        code::MEADOW => Some((true, [214, 236, 170], 0.35)),
        code::BUILT => Some((false, [150, 92, 70], 0.55)),
        _ => None,
    }
}

/// Farmland and built ground as quiet tints over the relief.
pub fn land_use(c: &mut Canvas, v: &View, codes: &[u8]) {
    for py in 0..c.height {
        for px in 0..c.width {
            let i = v.cell_at(px, py);
            let (x, y) = (px as i64, py as i64);
            if let Some((multiply, colour, alpha)) = land_tone(codes[i]) {
                if multiply {
                    c.multiply(x, y, colour, alpha);
                } else {
                    c.blend(x, y, colour, alpha);
                }
            }
        }
    }
}

/// Chamfer distance (in pixels) from every pixel to the nearest `seed`
/// pixel, capped at `cap`.
fn distance(w: usize, h: usize, seed: impl Fn(usize) -> bool, cap: f32) -> Vec<f32> {
    let mut d: Vec<f32> = (0..w * h)
        .map(|i| if seed(i) { 0.0 } else { cap })
        .collect();
    let (a, b) = (1.0_f32, std::f32::consts::SQRT_2);
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let mut m = d[i];
            if x > 0 {
                m = m.min(d[i - 1] + a);
            }
            if y > 0 {
                m = m.min(d[i - w] + a);
                if x > 0 {
                    m = m.min(d[i - w - 1] + b);
                }
                if x + 1 < w {
                    m = m.min(d[i - w + 1] + b);
                }
            }
            d[i] = m;
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            let mut m = d[i];
            if x + 1 < w {
                m = m.min(d[i + 1] + a);
            }
            if y + 1 < h {
                m = m.min(d[i + w] + a);
                if x + 1 < w {
                    m = m.min(d[i + w + 1] + b);
                }
                if x > 0 {
                    m = m.min(d[i + w - 1] + b);
                }
            }
            d[i] = m;
        }
    }
    d
}

/// Per-pixel realm ids.
#[must_use]
pub fn realm_pixels(c: &Canvas, v: &View, map: &[u16]) -> Vec<u16> {
    (0..c.height)
        .flat_map(|py| (0..c.width).map(move |px| (px, py)))
        .map(|(px, py)| map[v.cell_at(px, py)])
        .collect()
}

/// Whether pixel `i` of a `w`-wide realm raster touches another realm.
fn on_border(rp: &[u16], w: usize, h: usize, i: usize) -> bool {
    let r = rp[i];
    if r == 0 {
        return false;
    }
    let (x, y) = (i % w, i / w);
    [(1_i64, 0_i64), (-1, 0), (0, 1), (0, -1)]
        .iter()
        .any(|&(dx, dy)| {
            let (nx, ny) = (x as i64 + dx, y as i64 + dy);
            nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h && {
                let o = rp[ny as usize * w + nx as usize];
                o != 0 && o != r
            }
        })
}

/// The classic realm look: a soft tint over each realm, deepening into a
/// ribbon along its land borders.
pub fn realm_tints(c: &mut Canvas, v: &View, rp: &[u16], palette: &Palette) {
    let (w, h) = (c.width, c.height);
    let ribbon = 16.0 * v.s;
    let d = distance(w, h, |i| on_border(rp, w, h, i), ribbon + 1.0);
    for i in 0..w * h {
        let r = rp[i];
        if r == 0 {
            continue;
        }
        let t = (1.0 - d[i] / ribbon).max(0.0);
        let (x, y) = ((i % w) as i64, (i / w) as i64);
        let col = palette.colour(u64::from(r));
        c.blend(x, y, col, 0.13);
        if t > 0.0 {
            c.multiply(x, y, tint(col), 0.62 * t * t);
        }
    }
}

/// Label anchors: per realm id, a page pixel.
pub type Anchors = [(u16, (f32, f32))];

/// Per realm, the pixel deepest inside it, away from borders and the sea
/// alike (where its label goes).
#[must_use]
pub fn realm_anchors(w: usize, h: usize, rp: &[u16]) -> Vec<(u16, (f32, f32))> {
    let edge = |i: usize| {
        let r = rp[i];
        let (x, y) = (i % w, i / w);
        r == 0
            || x == 0
            || y == 0
            || x + 1 == w
            || y + 1 == h
            || rp[i - 1] != r
            || rp[i + 1] != r
            || rp[i - w] != r
            || rp[i + w] != r
    };
    let deep = distance(w, h, edge, 1.0e9);
    let mut best: std::collections::BTreeMap<u16, (f32, usize)> = std::collections::BTreeMap::new();
    for (i, &r) in rp.iter().enumerate() {
        if r != 0 && best.get(&r).is_none_or(|b| deep[i] > b.0) {
            best.insert(r, (deep[i], i));
        }
    }
    best.into_iter()
        .map(|(r, (_, i))| (r, ((i % w) as f32 + 0.5, (i / w) as f32 + 0.5)))
        .collect()
}

/// A light version of a colour for multiplying over relief.
fn tint(c: Rgb) -> Rgb {
    c.map(|v| (255.0 - (255.0 - f32::from(v)) * 0.9).round() as u8)
}

/// The classic realm borders: dashed lines over a pale casing, smoothed.
pub fn realm_borders(c: &mut Canvas, v: &View, realms: &RealmsFile) {
    let mut seen: BTreeSet<Vec<[i64; 2]>> = BTreeSet::new();
    let mut casing = c.mask();
    let mut dash = c.mask();
    for r in &realms.realms {
        for line in &r.borders {
            let mut key = line.clone();
            if key.first() > key.last() {
                key.reverse();
            }
            if !seen.insert(key) {
                continue;
            }
            let pts: Vec<(f32, f32)> = line.iter().map(|&m| v.px(m)).collect();
            let smooth = chaikin(&pts, 3);
            casing.polyline(&smooth, 4.2 * v.s);
            let mut phase = 0.0;
            dash.dashed(&smooth, 1.9 * v.s, 9.0 * v.s, 4.5 * v.s, &mut phase);
        }
    }
    c.paint(&casing, PAPER, 0.3);
    c.paint(&dash, [84, 24, 70], 1.0);
}

/// Symbol radius of a tier at scale 1 (the label's clearance).
#[must_use]
pub const fn symbol_r(t: Tier) -> f32 {
    match t {
        Tier::Hamlet => 1.4,
        Tier::Village => 2.6,
        Tier::Town => 4.4,
        Tier::City => 7.0,
    }
}

/// The four inks a settlement symbol is drawn in.
pub struct Symbols {
    /// Paper halo.
    pub halo: Mask,
    /// Dark ink.
    pub ink: Mask,
    /// Town red.
    pub red: Mask,
    /// Paper inside city walls.
    pub paper: Mask,
}

impl Symbols {
    /// Empty masks over a rectangle.
    #[must_use]
    pub fn new(x0: i64, y0: i64, w: usize, h: usize) -> Self {
        Self {
            halo: Mask::new(x0, y0, w, h),
            ink: Mask::new(x0, y0, w, h),
            red: Mask::new(x0, y0, w, h),
            paper: Mask::new(x0, y0, w, h),
        }
    }

    /// Draws one symbol: hamlet and village dots, a red town disc, a walled
    /// city (a ring of wall with eight bastions round a red core), and a
    /// ring round a realm seat.
    pub fn draw(&mut self, p: (f32, f32), tier: Tier, seat: bool, s: f32) {
        let r = symbol_r(tier) * s;
        match tier {
            Tier::Hamlet => self.ink.disc(p, r),
            Tier::Village => {
                self.halo.disc(p, r + 1.1 * s);
                self.ink.disc(p, r);
            }
            Tier::Town => {
                self.halo.disc(p, r + 1.4 * s);
                self.ink.disc(p, r);
                self.red.disc(p, r - 1.2 * s);
            }
            Tier::City => {
                self.halo.disc(p, r + 2.0 * s);
                self.ink.ring_between(p, r - 1.7 * s, r);
                for k in 0..8 {
                    let a = k as f32 * std::f32::consts::FRAC_PI_4;
                    let q = (p.0 + a.cos() * (r - 0.8 * s), p.1 + a.sin() * (r - 0.8 * s));
                    self.ink.disc(q, 1.6 * s);
                }
                self.paper.disc(p, r - 1.7 * s);
                self.red.disc(p, r - 3.0 * s);
            }
        }
        if seat {
            self.halo.ring_between(p, r + 1.5 * s, r + 4.2 * s);
            self.ink.ring_between(p, r + 2.3 * s, r + 3.3 * s);
        }
    }

    /// Composites the symbols.
    pub fn paint(&self, c: &mut Canvas) {
        self.paint_at(c, (0, 0));
    }

    /// Composites symbols drawn in the coordinates of a larger image onto
    /// a canvas that is its window starting at pixel `origin`.
    pub fn paint_at(&self, c: &mut Canvas, origin: (i64, i64)) {
        c.paint_at(&self.halo, PAPER, 0.9, origin);
        c.paint_at(&self.ink, INK, 1.0, origin);
        c.paint_at(&self.paper, PAPER, 1.0, origin);
        c.paint_at(&self.red, TOWN_RED, 1.0, origin);
    }
}

/// One settlement symbol to draw: pixel position, tier, realm seat.
#[derive(Debug, Clone, Copy)]
pub struct Sym {
    /// Centre, page pixels.
    pub at: (f32, f32),
    /// Size class.
    pub tier: Tier,
    /// Whether it seats a realm.
    pub seat: bool,
}

/// Every settlement's symbol, small first (so big ones overdraw); with
/// `hamlets` false (world scale, [`super::scale::shows_hamlets`]) cities,
/// towns and villages only.
#[must_use]
pub fn symbol_list(v: &View, st: &SettlementsFile, seats: &[u64], hamlets: bool) -> Vec<Sym> {
    let mut list: Vec<_> = st
        .settlements
        .iter()
        .filter(|x| hamlets || x.tier != Tier::Hamlet)
        .collect();
    list.sort_by_key(|x| (x.tier, x.id));
    list.into_iter()
        .map(|x| Sym {
            at: v.px([x.x_m, x.y_m]),
            tier: x.tier,
            seat: seats.contains(&x.id.get()),
        })
        .collect()
}

/// Paints the symbols meeting a canvas that is the window starting at
/// pixel `origin` of the symbols' own coordinates, at scale `s`. No
/// placer is involved: labels, when drawn, reserve boxes on their own.
pub fn paint_symbols(c: &mut Canvas, list: &[Sym], s: f32, origin: (i64, i64)) {
    let mut sym = Symbols::new(origin.0, origin.1, c.width, c.height);
    let rect = [
        origin.0 as f32,
        origin.1 as f32,
        (origin.0 + c.width as i64) as f32,
        (origin.1 + c.height as i64) as f32,
    ];
    for x in list {
        // Halo and seat ring reach at most 7 px past the radius.
        let reach = (symbol_r(x.tier) + 7.0) * s;
        if x.at.0 + reach < rect[0]
            || x.at.0 - reach > rect[2]
            || x.at.1 + reach < rect[1]
            || x.at.1 - reach > rect[3]
        {
            continue;
        }
        sym.draw(x.at, x.tier, x.seat, s);
    }
    sym.paint_at(c, origin);
}

/// Every settlement's symbol, small first (hamlets only with `hamlets`);
/// reserves their boxes.
pub fn settlements(
    c: &mut Canvas,
    v: &View,
    (st, seats): (&SettlementsFile, &[u64]),
    hamlets: bool,
    pl: &mut Placer<'_>,
) {
    let list = symbol_list(v, st, seats, hamlets);
    paint_symbols(c, &list, v.s, (0, 0));
    for x in &list {
        pl.reserve(Rect::around(x.at, symbol_r(x.tier) * v.s + v.s));
    }
}

/// Crossed picks at mines, pass marks on roads, and peak triangles; each
/// reserves its box.
pub fn features(
    c: &mut Canvas,
    v: &View,
    codes: &[u8],
    roads: &RoadsFile,
    peaks: &[(f32, f32)],
    pl: &mut Placer<'_>,
) {
    let s = v.s;
    let (mut halo, mut ink) = (c.mask(), c.mask());
    // One mark per mine, where its first cell lies.
    let mut mines: Vec<(f32, f32)> = Vec::new();
    for (i, &k) in codes.iter().enumerate() {
        if k != code::MINE_QUARRY {
            continue;
        }
        let m = [
            i64::try_from(i % v.gw).unwrap_or(0) * 100 + 50,
            i64::try_from(i / v.gw).unwrap_or(0) * 100 + 50,
        ];
        let p = v.px(m);
        if mines
            .iter()
            .all(|q| (q.0 - p.0).hypot(q.1 - p.1) > 12.0 * s)
        {
            mines.push(p);
        }
    }
    let r = 3.4 * s;
    for &p in &mines {
        for (a, b) in [((-r, -r), (r, r)), ((-r, r), (r, -r))] {
            halo.segment((p.0 + a.0, p.1 + a.1), (p.0 + b.0, p.1 + b.1), 3.6 * s);
            ink.segment((p.0 + a.0, p.1 + a.1), (p.0 + b.0, p.1 + b.1), 1.3 * s);
        }
        ink.segment(
            (p.0 - r - 1.2 * s, p.1 - r + 1.2 * s),
            (p.0 - r + 1.2 * s, p.1 - r - 1.2 * s),
            1.3 * s,
        );
        ink.segment(
            (p.0 + r - 1.2 * s, p.1 - r - 1.2 * s),
            (p.0 + r + 1.2 * s, p.1 - r + 1.2 * s),
            1.3 * s,
        );
        pl.reserve(Rect::around(p, r + 1.0));
    }
    // A pass is a pair of brackets facing each other across the road: )(.
    for p in &roads.passes {
        let q = v.px([p.x_m, p.y_m]);
        for side in [-1.0_f32, 1.0] {
            let arc: Vec<(f32, f32)> = (0..=6)
                .map(|k| {
                    let t = (k as f32 / 6.0 - 0.5) * 2.0;
                    (q.0 + side * (2.2 + 1.6 * t * t) * s, q.1 + t * 4.2 * s)
                })
                .collect();
            halo.polyline(&arc, 3.4 * s);
            ink.polyline(&arc, 1.3 * s);
        }
        pl.reserve(Rect::around(q, 5.0 * s));
    }
    for &p in peaks {
        let (a, b, cc) = (
            (p.0, p.1 - 4.6 * s),
            (p.0 - 4.2 * s, p.1 + 2.8 * s),
            (p.0 + 4.2 * s, p.1 + 2.8 * s),
        );
        halo.triangle(
            (a.0, a.1 - 1.6 * s),
            (b.0 - 1.5 * s, b.1 + 0.9 * s),
            (cc.0 + 1.5 * s, cc.1 + 0.9 * s),
        );
        ink.triangle(a, b, cc);
        pl.reserve(Rect::around(p, 5.0 * s));
    }
    c.paint(&halo, PAPER, 0.85);
    c.paint(&ink, [70, 46, 30], 1.0);
}
