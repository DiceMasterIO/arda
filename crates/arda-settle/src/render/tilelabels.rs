//! Map labels for the tile pyramid and the offline PNG: the gazetteer's
//! natural features (area names spread along the feature, in italic on
//! water) and the settlements, placed once per zoom level against the whole
//! level and then clipped to whatever window is drawn (logic/17 §labels).
//!
//! A level's layout is a pure function of the gazetteer, the settlements and
//! the zoom, so neighbouring tiles carry the same labels and a label that
//! crosses a tile edge is cut, not re-placed. Glyph coverage depends only on
//! global pixel coordinates, so tiles join exactly.
//!
//! Placement is greedy, most important first (rank, then kind, then id).
//! Settlement symbols are reserved first; a settlement name tries the eight
//! positions beside its symbol, an area name its feature's path (curved
//! text, letter-spaced to fill about 70% of it) or, with no path, the
//! anchor, nudged up or down until clear. Names that do not fit are left
//! out.

use super::font::{Case, Face, Fonts, Placed, Style};
use super::labels::{at, bend, cumulative, glyph_frame, Rect};
use super::mask::{chaikin, Mask};
use crate::canvas::Rgb;
use crate::error::SettleError;
use crate::gazetteer::{GazetteerFile, GazetteerItem, GazetteerList, PlaceKind};
use crate::model::Settlement;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

/// The finest overview zoom of the default pyramid (4,096 pixels on the
/// long edge).
const DEFAULT_BASE_ZOOM: u8 = 4;

/// Halo colour.
pub const PAPER: Rgb = [250, 245, 230];
/// Halo opacity.
pub const HALO_OPACITY: f32 = 0.8;
/// Halo radius, pixels.
const HALO_R: f32 = 1.7;
/// Pixels a window grows by so halos join across its edge.
const PAD: i64 = 6;
/// Water names are pale on a deep blue halo: they sit on sea and lake.
const WATER: Rgb = [226, 238, 250];
const WATER_HALO: Rgb = [18, 52, 100];
const INK: Rgb = [38, 28, 22];
const EARTH: Rgb = [92, 60, 40];
const WOOD: Rgb = [38, 82, 44];
const FEN: Rgb = [48, 98, 92];

/// What the finder knows about one thing to label.
#[derive(Debug, Clone)]
pub struct Item {
    /// Kind.
    pub kind: PlaceKind,
    /// Name.
    pub name: String,
    /// Anchor, metres.
    pub anchor: [i64; 2],
    /// Label path, metres.
    pub path: Option<Vec<[i64; 2]>>,
    /// Bounding box, metres.
    pub extent: [i64; 4],
    /// Importance, 1 to 5.
    pub rank: u8,
    /// First zoom at which it shows.
    pub min_zoom: u8,
    /// Tie-break: file position.
    pub order: u64,
}

/// One laid-out glyph.
#[derive(Debug, Clone, Copy)]
struct Glyph {
    placed: Placed,
    /// Pen position of an upright glyph; centre of a rotated one.
    at: (f32, f32),
    angle: f32,
}

/// One placed label.
#[derive(Debug, Clone)]
struct Label {
    face: Face,
    colour: Rgb,
    halo: Rgb,
    glyphs: Vec<Glyph>,
    /// Box of everything drawn, halo included.
    bounds: Rect,
}

/// A layout of one level.
#[derive(Debug, Clone, Default)]
pub struct Level {
    labels: Vec<Label>,
    /// The type scale the level was laid out at (0 for an empty level).
    unit: f32,
}

/// Coverage to composite: the halo, then the text.
pub struct Ink {
    /// Halo coverage.
    pub halo: Mask,
    /// Text coverage.
    pub text: Mask,
    /// Text colour.
    pub colour: Rgb,
    /// Halo colour.
    pub halo_colour: Rgb,
}

/// The labels of one world and their layouts, per zoom.
pub struct Labeler {
    fonts: Fonts,
    items: Vec<Item>,
    world_m: [f64; 2],
    longest_m: f64,
    /// The finest overview zoom: names are bigger at and above it
    /// ([`super::scale::level_unit`]).
    base_zoom: u8,
    levels: Mutex<BTreeMap<u8, Arc<Level>>>,
}

/// Spatial index of occupied boxes.
struct Taken {
    cell: f32,
    rects: Vec<Rect>,
    buckets: BTreeMap<(i64, i64), Vec<usize>>,
    world: (f32, f32),
}

impl Taken {
    fn range(&self, r: &Rect) -> (i64, i64, i64, i64) {
        let f = |v: f32| (v / self.cell).floor() as i64;
        (f(r.x0), f(r.y0), f(r.x1), f(r.y1))
    }

    fn reserve(&mut self, r: Rect) {
        let (a, b, c, d) = self.range(&r);
        let k = self.rects.len();
        self.rects.push(r);
        for y in b..=d {
            for x in a..=c {
                self.buckets.entry((x, y)).or_default().push(k);
            }
        }
    }

    fn free(&self, r: &Rect) -> bool {
        if r.x0 < 0.0 || r.y0 < 0.0 || r.x1 > self.world.0 || r.y1 > self.world.1 {
            return false;
        }
        let (a, b, c, d) = self.range(r);
        for y in b..=d {
            for x in a..=c {
                if let Some(list) = self.buckets.get(&(x, y)) {
                    if list.iter().any(|&k| hits(&self.rects[k], r)) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

fn hits(a: &Rect, b: &Rect) -> bool {
    a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1
}

impl Labeler {
    /// Labels for a gazetteer and its settlements. `world_m` is the world's
    /// width and height; `longest_m` the side that spans 256 pixels at
    /// zoom 0.
    ///
    /// # Errors
    /// [`SettleError::Png`] if the embedded fonts fail to parse.
    pub fn new(
        file: &GazetteerFile,
        settlements: &[Settlement],
        world_m: [f64; 2],
        longest_m: f64,
    ) -> Result<Self, SettleError> {
        let list = crate::gazetteer::list(file, settlements, &crate::gazetteer::Filter::default());
        Self::from_list(&list, world_m, longest_m)
    }

    /// Labels for an already assembled list.
    ///
    /// # Errors
    /// [`SettleError::Png`] if the embedded fonts fail to parse.
    pub fn from_list(
        list: &GazetteerList,
        world_m: [f64; 2],
        longest_m: f64,
    ) -> Result<Self, SettleError> {
        let items = list
            .features
            .iter()
            .enumerate()
            .map(|(k, i)| match i {
                GazetteerItem::Feature(f) => Item {
                    kind: f.kind,
                    name: f.name.clone(),
                    anchor: f.anchor_m,
                    path: f.path_m.clone(),
                    extent: f.extent_m,
                    rank: f.rank,
                    min_zoom: f.min_zoom,
                    order: k as u64,
                },
                GazetteerItem::Point(p) => Item {
                    kind: p.kind,
                    name: p.name.clone(),
                    anchor: p.at_m,
                    path: None,
                    extent: [p.at_m[0], p.at_m[1], p.at_m[0], p.at_m[1]],
                    rank: p.rank,
                    min_zoom: p.min_zoom,
                    order: k as u64,
                },
            })
            .collect();
        Ok(Self {
            fonts: Fonts::new()?,
            items,
            world_m,
            longest_m,
            base_zoom: DEFAULT_BASE_ZOOM,
            levels: Mutex::new(BTreeMap::new()),
        })
    }

    /// Sets the finest overview zoom of the pyramid these labels serve, so
    /// the coarse levels' larger names line up with its symbols.
    #[must_use]
    pub fn with_base_zoom(mut self, base_zoom: u8) -> Self {
        self.base_zoom = base_zoom;
        self
    }

    /// Metres per pixel at pyramid zoom `z`.
    #[must_use]
    pub fn metres_per_px(&self, z: u8) -> f64 {
        self.longest_m / (256.0 * f64::from(1_u32 << u32::from(z.min(24))))
    }

    /// Whether anything is labelled.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The layout of pyramid level `z`, made once.
    pub fn level(&self, z: u8) -> Arc<Level> {
        let mut levels = self.levels.lock().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(levels.entry(z).or_insert_with(|| {
            let unit = super::scale::level_unit(z, self.base_zoom);
            Arc::new(self.lay_out(1.0 / self.metres_per_px(z), unit, z, &[], |_| true))
        }))
    }

    /// The layout for a whole image `width_px` pixels across that spans the
    /// world's width, for exports drawn on one image (the opt-in names of
    /// `/v1/overview.png`): the pyramid level whose scale is nearest the
    /// image's (a 4,096 pixel long edge is level 4), so it shows the same
    /// names at the same size as that level's tiles.
    #[must_use]
    pub fn level_for_width(&self, width_px: u32) -> Level {
        let px_per_m = f64::from(width_px) / self.world_m[0];
        let long_px = (self.longest_m * px_per_m).max(256.0);
        let z = (long_px / 256.0).log2().round().clamp(0.0, 24.0) as u8;
        let unit = super::scale::level_unit(z, self.base_zoom);
        self.lay_out(px_per_m, unit, z, &[], |_| true)
    }

    /// A layout at `px_per_m` for the offline page: every item `keep`
    /// accepts, around the boxes in `blocked`.
    pub fn level_at(
        &self,
        px_per_m: f64,
        unit: f32,
        blocked: &[Rect],
        keep: impl Fn(&Item) -> bool,
    ) -> Level {
        self.lay_out(px_per_m, unit, u8::MAX, blocked, keep)
    }

    /// The coverage of `level` inside the window at `origin` of `size`
    /// pixels, halos reaching in from outside included.
    #[must_use]
    pub fn inks(&self, level: &Level, origin: (i64, i64), size: (usize, usize)) -> Vec<Ink> {
        let (x0, y0) = (origin.0 - PAD, origin.1 - PAD);
        let (x1, y1) = (
            origin.0 + i64::try_from(size.0).unwrap_or(0) + PAD,
            origin.1 + i64::try_from(size.1).unwrap_or(0) + PAD,
        );
        let win = Rect::at(x0 as f32, y0 as f32, (x1 - x0) as f32, (y1 - y0) as f32);
        let mut out = Vec::new();
        for l in level.labels.iter().filter(|l| hits(&l.bounds, &win)) {
            let (bx0, by0) = (
                (l.bounds.x0.floor() as i64).max(x0),
                (l.bounds.y0.floor() as i64).max(y0),
            );
            let (bx1, by1) = (
                (l.bounds.x1.ceil() as i64).min(x1),
                (l.bounds.y1.ceil() as i64).min(y1),
            );
            let mut m = Mask::new(
                bx0,
                by0,
                usize::try_from(bx1 - bx0).unwrap_or(0),
                usize::try_from(by1 - by0).unwrap_or(0),
            );
            for g in &l.glyphs {
                if g.angle == 0.0 {
                    self.fonts.glyph(&mut m, l.face, &g.placed, g.at.0, g.at.1);
                } else {
                    self.fonts.glyph_rotated(
                        &mut m,
                        l.face,
                        &g.placed,
                        g.at.0,
                        g.at.1,
                        g.angle,
                        g.placed.size * 0.3,
                    );
                }
            }
            out.push(Ink {
                halo: m.dilate(HALO_R * level.unit.max(1.0)),
                text: m,
                colour: l.colour,
                halo_colour: l.halo,
            });
        }
        out
    }

    /// Draws `level`'s labels into an RGBA window (`size.0 × size.1`
    /// pixels at `origin`). Pixels with no alpha take the ink's alpha.
    pub fn draw_rgba(
        &self,
        level: &Level,
        origin: (i64, i64),
        size: (usize, usize),
        rgba: &mut [u8],
    ) {
        let inks = self.inks(level, origin, size);
        for i in &inks {
            blend(rgba, origin, size, &i.halo, i.halo_colour, HALO_OPACITY);
        }
        for i in &inks {
            blend(rgba, origin, size, &i.text, i.colour, 1.0);
        }
    }
}

fn blend(rgba: &mut [u8], origin: (i64, i64), size: (usize, usize), m: &Mask, c: Rgb, op: f32) {
    for y in 0..m.h {
        for x in 0..m.w {
            let a = m.a[y * m.w + x] * op;
            if a <= 0.0 {
                continue;
            }
            let (px, py) = (m.x0 + x as i64 - origin.0, m.y0 + y as i64 - origin.1);
            if px < 0 || py < 0 || px >= size.0 as i64 || py >= size.1 as i64 {
                continue;
            }
            let k = (py as usize * size.0 + px as usize) * 4;
            let a = a.min(1.0);
            let old_a = f32::from(rgba[k + 3]) / 255.0;
            // Over transparent ground the ink is its own colour.
            let t = if old_a <= 0.0 { 1.0 } else { a };
            for (ch, &v) in c.iter().enumerate() {
                let old = f32::from(rgba[k + ch]);
                rgba[k + ch] = (old + (f32::from(v) - old) * t).round().clamp(0.0, 255.0) as u8;
            }
            rgba[k + 3] = (old_a.max(a) * 255.0).round() as u8;
        }
    }
}

/// How one kind of label looks.
struct Look {
    face: Face,
    case: Case,
    colour: Rgb,
    size: f32,
    /// Widest letter-spacing, em.
    spread: f32,
}

fn look(kind: PlaceKind, rank: u8, unit: f32) -> Look {
    let big = [27.0, 22.0, 18.0, 15.0, 12.5][usize::from(rank.clamp(1, 5)) - 1];
    let l = |face, case, colour, size, spread| Look {
        face,
        case,
        colour,
        size: size * unit,
        spread,
    };
    match kind {
        PlaceKind::Sea => l(Face::Italic, Case::AsIs, WATER, big, 0.9),
        PlaceKind::Bay | PlaceKind::Lake => l(Face::Italic, Case::AsIs, WATER, big, 0.5),
        PlaceKind::River => l(Face::Italic, Case::AsIs, WATER, big, 0.15),
        PlaceKind::Mountain => l(Face::Display, Case::SmallCaps, EARTH, big * 1.05, 0.7),
        PlaceKind::Forest => l(Face::Italic, Case::AsIs, WOOD, big, 0.6),
        PlaceKind::Marsh => l(Face::Italic, Case::AsIs, FEN, big, 0.5),
        PlaceKind::Island => l(Face::Regular, Case::AsIs, INK, big, 0.4),
        PlaceKind::Cape => l(Face::Italic, Case::AsIs, EARTH, big, 0.0),
        PlaceKind::City => l(Face::Bold, Case::Upper, INK, 17.0, 0.0),
        PlaceKind::Town => l(Face::Medium, Case::AsIs, INK, 14.0, 0.0),
        PlaceKind::Village => l(Face::Regular, Case::AsIs, INK, 12.0, 0.0),
        _ => l(Face::Regular, Case::AsIs, INK, 10.5, 0.0),
    }
}

/// The clearance a settlement's symbol takes. At page size the label's own
/// radii; above it (the coarse levels) the radii of the overview symbols
/// drawn at the same scale (`layers::symbol_r`).
fn symbol_r(kind: PlaceKind, unit: f32) -> f32 {
    if unit > 1.0 {
        let page = match kind {
            PlaceKind::City => 7.0,
            PlaceKind::Town => 4.4,
            PlaceKind::Village => 2.6,
            _ => 1.4,
        };
        return page * unit;
    }
    match kind {
        PlaceKind::City => 4.5,
        PlaceKind::Town => 3.6,
        PlaceKind::Village => 2.8,
        _ => 2.0,
    }
}

fn order_of(kind: PlaceKind) -> u8 {
    match kind {
        PlaceKind::Sea => 0,
        PlaceKind::Mountain => 1,
        PlaceKind::City => 2,
        PlaceKind::Town => 3,
        PlaceKind::Island | PlaceKind::Bay | PlaceKind::Lake | PlaceKind::Forest => 4,
        PlaceKind::River | PlaceKind::Marsh | PlaceKind::Cape => 5,
        _ => 6,
    }
}

impl Labeler {
    fn lay_out(
        &self,
        px_per_m: f64,
        unit: f32,
        z: u8,
        blocked: &[Rect],
        keep: impl Fn(&Item) -> bool,
    ) -> Level {
        let world = (
            (self.world_m[0] * px_per_m) as f32,
            (self.world_m[1] * px_per_m) as f32,
        );
        let mut taken = Taken {
            cell: 96.0,
            rects: Vec::new(),
            buckets: BTreeMap::new(),
            world,
        };
        for r in blocked {
            taken.reserve(*r);
        }
        let mut shown: Vec<&Item> = self
            .items
            .iter()
            .filter(|i| i.min_zoom <= z && keep(i))
            .collect();
        shown.sort_by_key(|i| (i.rank, order_of(i.kind), i.order));
        let px = |p: [i64; 2]| {
            (
                (p[0] as f64 * px_per_m) as f32,
                (p[1] as f64 * px_per_m) as f32,
            )
        };
        for i in shown.iter().filter(|i| i.kind.is_settlement()) {
            let (x, y) = px(i.anchor);
            taken.reserve(Rect::around((x, y), symbol_r(i.kind, unit)));
        }
        let mut labels = Vec::new();
        for i in shown {
            let placed = if i.kind.is_settlement() || i.kind == PlaceKind::Cape {
                self.beside(i, px(i.anchor), unit, &mut taken)
            } else {
                self.spread(i, &px, (px_per_m, unit), &mut taken)
            };
            labels.extend(placed);
        }
        Level { labels, unit }
    }

    fn style(&self, i: &Item, size: f32, tracking: f32, unit: f32) -> (Style, Look) {
        let lk = look(i.kind, i.rank, unit);
        (
            Style::new(lk.face, size).tracked(tracking).cased(lk.case),
            lk,
        )
    }

    /// A name beside a point: the eight positions around it.
    fn beside(&self, i: &Item, at_px: (f32, f32), unit: f32, taken: &mut Taken) -> Option<Label> {
        let lk = look(i.kind, i.rank, unit);
        let style = Style::new(lk.face, lk.size).cased(lk.case);
        let (glyphs, w) = self.fonts.layout(&style, &i.name);
        let cap = self.fonts.cap_height(&style);
        let h = cap + self.fonts.descent(&style) * 0.55 + lk.size * 0.12;
        let r = symbol_r(i.kind, unit);
        let g = r + 2.0 + lk.size * 0.12;
        let d = g * 0.72;
        let (ax, ay) = at_px;
        let mid = ay - h / 2.0;
        let spots = [
            (ax + g, mid),
            (ax + d, ay - d - h),
            (ax + d, ay + d),
            (ax - g - w, mid),
            (ax - d - w, ay - d - h),
            (ax - d - w, ay + d),
            (ax - w / 2.0, ay - g - h),
            (ax - w / 2.0, ay + g),
        ];
        let pad = HALO_R * 0.6;
        let (x, y) = spots.into_iter().find(|&(x, y)| {
            taken.free(&Rect::at(x - pad, y - pad, w + 2.0 * pad, h + 2.0 * pad))
        })?;
        taken.reserve(Rect::at(x - pad, y - pad, w + 2.0 * pad, h + 2.0 * pad));
        let baseline = y + cap + lk.size * 0.06;
        let run: Vec<Glyph> = glyphs
            .iter()
            .map(|p| Glyph {
                placed: *p,
                at: (x + p.x, baseline),
                angle: 0.0,
            })
            .collect();
        Some(label(lk.face, lk.colour, run, Rect::at(x, y, w, h)))
    }

    /// An area name spread along its path, or across its extent.
    fn spread(
        &self,
        i: &Item,
        px: &dyn Fn([i64; 2]) -> (f32, f32),
        (px_per_m, unit): (f64, f32),
        taken: &mut Taken,
    ) -> Option<Label> {
        let lk = look(i.kind, i.rank, unit);
        let ext = (
            (i.extent[2] - i.extent[0]) as f64 * px_per_m,
            (i.extent[3] - i.extent[1]) as f64 * px_per_m,
        );
        let natural = |size: f32| {
            self.fonts
                .width(&Style::new(lk.face, size).cased(lk.case), &i.name)
        };
        let n = i.name.chars().count().max(2) as f32;
        let path: Option<Vec<(f32, f32)>> = i.path.as_ref().map(|p| {
            let pts: Vec<(f32, f32)> = p.iter().map(|&m| px(m)).collect();
            if i.kind == PlaceKind::River {
                chaikin(&pts, 3)
            } else {
                chaikin(&pts, 2)
            }
        });
        let reach = |len: f32| -> Option<(f32, f32)> {
            // Size and tracking so the name fills about 70% of `len`.
            let mut size = lk.size;
            if natural(size) > len * 0.95 {
                size *= len * 0.95 / natural(size);
                if size < lk.size * 0.6 {
                    return None;
                }
            }
            let free = (len * 0.7 - natural(size)).max(0.0);
            Some((size, (free / (n - 1.0) / size).min(lk.spread)))
        };
        if let Some(path) = path.filter(|p| p.len() >= 2) {
            let (cum, total) = cumulative(&path);
            if let Some((size, tracking)) = reach(total) {
                let lift = if i.kind == PlaceKind::River {
                    size * 0.55
                } else {
                    0.0
                };
                let (style, _) = self.style(i, size, tracking, unit);
                if let Some(l) = self.along(&style, &lk, &i.name, (&path, &cum, total), lift, taken)
                {
                    return Some(l);
                }
            }
        }
        if i.kind == PlaceKind::River || i.kind == PlaceKind::Mountain {
            return None;
        }
        // Across the extent, centred on the anchor.
        let (ax, ay) = px(i.anchor);
        let width = ext.0 as f32;
        let fit = reach(width.max(1.0));
        let (size, tracking) = match fit {
            Some(f) => f,
            None => return self.beside(i, (ax, ay), unit, taken),
        };
        let (style, _) = self.style(i, size, tracking, unit);
        let (glyphs, w) = self.fonts.layout(&style, &i.name);
        let cap = self.fonts.cap_height(&style);
        for k in [0.0_f32, -1.0, 1.0, -2.0, 2.0] {
            let y = ay + k * size * 1.3;
            let baseline = y + cap / 2.0;
            let x = ax - w / 2.0;
            let boxes: Vec<Rect> = glyphs
                .iter()
                .map(|p| {
                    Rect::at(
                        x + p.x,
                        baseline - cap,
                        p.advance.max(size * 0.3),
                        cap * 1.3,
                    )
                })
                .collect();
            if boxes.iter().all(|b| taken.free(b)) {
                for b in &boxes {
                    taken.reserve(*b);
                }
                let run = glyphs
                    .iter()
                    .map(|p| Glyph {
                        placed: *p,
                        at: (x + p.x, baseline),
                        angle: 0.0,
                    })
                    .collect();
                let bounds = Rect::at(x, baseline - cap, w, cap * 1.4);
                return Some(label(lk.face, lk.colour, run, bounds));
            }
        }
        None
    }

    /// Curved text along a path, on the straightest free stretch near its
    /// middle.
    fn along(
        &self,
        style: &Style,
        lk: &Look,
        text: &str,
        (path, cum, total): (&[(f32, f32)], &[f32], f32),
        lift: f32,
        taken: &mut Taken,
    ) -> Option<Label> {
        let (glyphs, len) = self.fonts.layout(style, text);
        if total < len * 1.02 || glyphs.is_empty() {
            return None;
        }
        let size = style.size;
        let boxes_at = |start: f32, flip: bool| -> Vec<Rect> {
            glyphs
                .iter()
                .map(|g| {
                    let (c, _) = glyph_frame(path, cum, start, len, g, flip, lift);
                    Rect::around(c, size * 0.45)
                })
                .collect()
        };
        let mut best: Option<(f32, f32, bool)> = None;
        for k in 0..=24 {
            let t = k as f32 / 24.0;
            let start = (total - len) * (0.1 + 0.8 * t);
            let b = bend(path, cum, start, start + len);
            let score = b + (t - 0.5).abs() * 0.4;
            if b > 1.2 || best.is_some_and(|x| x.0 <= score) {
                continue;
            }
            let (p, q) = (at(path, cum, start), at(path, cum, start + len));
            let flip = q.0 < p.0;
            if boxes_at(start, flip).iter().all(|r| taken.free(r)) {
                best = Some((score, start, flip));
            }
        }
        let (_, start, flip) = best?;
        let boxes = boxes_at(start, flip);
        let mut run = Vec::new();
        for (g, b) in glyphs.iter().zip(&boxes) {
            let (c, ang) = glyph_frame(path, cum, start, len, g, flip, lift);
            run.push(Glyph {
                placed: *g,
                at: c,
                angle: if ang == 0.0 { 1e-6 } else { ang },
            });
            taken.reserve(*b);
        }
        let bounds = boxes.iter().fold(boxes[0], |a, b| Rect {
            x0: a.x0.min(b.x0),
            y0: a.y0.min(b.y0),
            x1: a.x1.max(b.x1),
            y1: a.y1.max(b.y1),
        });
        Some(label(lk.face, lk.colour, run, bounds))
    }
}

fn label(face: Face, colour: Rgb, glyphs: Vec<Glyph>, b: Rect) -> Label {
    let halo = if colour == WATER { WATER_HALO } else { PAPER };
    let m = HALO_R + 4.0;
    Label {
        face,
        colour,
        halo,
        glyphs,
        bounds: Rect {
            x0: b.x0 - m - 8.0,
            y0: b.y0 - m - 8.0,
            x1: b.x1 + m + 8.0,
            y1: b.y1 + m + 8.0,
        },
    }
}
