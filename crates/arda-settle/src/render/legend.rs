//! The legend panel of the offline page: one entry per symbol the page
//! actually draws, each swatch painted by the very code that paints the
//! map (roads through [`paint_roads`], the political border through
//! [`PoliticalLayer::swatch`]), so the key cannot drift from the map.

use super::font::{Case, Face, Fonts, Style};
use super::furniture::{panel, scale_bar, text};
use super::layers::{paint_roads, Line, RoadLines, RoadStyle, Symbols, CLASSES, INK};
use super::mask::Mask;
use super::political::PoliticalLayer;
use super::View;
use crate::canvas::{Canvas, Rgb};
use crate::model::Tier;
use crate::roads::RoadClass;

/// How realm borders are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderStyle {
    /// Each realm's colour as an inner band fading inward, and a dark
    /// dash-dot line over a pale halo ([`super::political`]).
    #[default]
    EdgeBand,
    /// A soft tint over each realm deepening into a ribbon at its border,
    /// and a dashed purple line.
    Ribbon,
}

/// What the page shows, and so what its legend lists.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key {
    /// Road look.
    pub roads: RoadStyle,
    /// Border look.
    pub borders: BorderStyle,
    /// Whether hamlets (and the tracks built to them) are drawn.
    pub hamlets: bool,
    /// Two neighbouring realm colours for the border swatch.
    pub realm_pair: [Rgb; 2],
}

/// The settlement entries: `(tier, seat, label)`.
#[must_use]
pub fn places(key: &Key) -> Vec<(Tier, bool, &'static str)> {
    let mut out = vec![
        (Tier::City, false, "City"),
        (Tier::Town, false, "Town"),
        (Tier::Village, false, "Village"),
    ];
    if key.hamlets {
        out.push((Tier::Hamlet, false, "Hamlet"));
    }
    out.push((Tier::Town, true, "Realm seat"));
    out
}

/// The road entries, most important first: tracks only with hamlets
/// (every track is built to a hamlet).
#[must_use]
pub fn roads(key: &Key) -> Vec<(RoadClass, &'static str)> {
    let mut out = vec![(RoadClass::Highway, "Highway"), (RoadClass::Road, "Road")];
    if key.hamlets {
        out.push((RoadClass::Track, "Track"));
    }
    out.push((RoadClass::Footpath, "Footpath"));
    out
}

/// A road swatch: one short line of `class` painted by the map's own road
/// painter.
pub fn road_swatch(
    c: &mut Canvas,
    style: RoadStyle,
    class: RoadClass,
    seg: [(f32, f32); 2],
    s: f32,
) {
    let mut lines = RoadLines::default();
    if let Some(k) = CLASSES.iter().position(|&k| k == class) {
        lines.by_class[k].push(Line::new(seg.to_vec()));
    }
    paint_roads(c, &lines, (s, style), (0, 0), &[class]);
}

/// Draws the legend panel in box `b` for what `key` says the page shows.
pub fn draw(c: &mut Canvas, v: &View, f: &Fonts, b: (f32, f32, f32, f32), key: &Key) {
    let s = v.s;
    panel(c, b, s);
    let (x, y, w, h) = b;
    let head = Style::new(Face::Bold, 15.0 * s)
        .tracked(0.12)
        .cased(Case::SmallCaps);
    let body = Style::new(Face::Regular, 13.0 * s);
    text(
        c,
        f,
        &head,
        "Legend",
        x + (w - f.width(&head, "Legend")) / 2.0,
        y + 28.0 * s,
        [60, 36, 26],
    );
    let col = [x + 20.0 * s, x + w / 2.0 + 8.0 * s];
    let row = |k: usize| y + 58.0 * s + k as f32 * 25.0 * s;
    let mut sym = Symbols::new(x as i64, y as i64, w as usize, h as usize);
    let places = places(key);
    for (k, &(tier, seat, label)) in places.iter().enumerate() {
        sym.draw((col[0] + 10.0 * s, row(k) - 4.5 * s), tier, seat, s);
        text(c, f, &body, label, col[0] + 28.0 * s, row(k), INK);
    }
    sym.paint(c);
    let roads = roads(key);
    for (k, &(class, label)) in roads.iter().enumerate() {
        let yy = row(k) - 4.5 * s;
        road_swatch(
            c,
            key.roads,
            class,
            [(col[1], yy), (col[1] + 26.0 * s, yy)],
            s,
        );
        text(c, f, &body, label, col[1] + 34.0 * s, row(k), INK);
    }
    // Border, then mines, passes and peaks under the roads.
    let border_row = roads.len();
    let yy = row(border_row) - 4.5 * s;
    let seg = [(col[1], yy), (col[1] + 26.0 * s, yy)];
    match key.borders {
        BorderStyle::EdgeBand => PoliticalLayer::swatch(c, seg, s, key.realm_pair),
        BorderStyle::Ribbon => {
            let mut m = Mask::new(x as i64, y as i64, w as usize, h as usize);
            m.dashed(&seg, 1.9 * s, 9.0 * s, 4.5 * s, &mut 0.0);
            c.paint(&m, [92, 30, 78], 0.95);
        }
    }
    text(
        c,
        f,
        &body,
        "Realm border",
        col[1] + 34.0 * s,
        row(border_row),
        INK,
    );
    let river_row = places.len();
    let italic = Style::new(Face::Italic, 13.0 * s);
    text(
        c,
        f,
        &italic,
        "River",
        col[0] + 28.0 * s,
        row(river_row),
        [40, 84, 140],
    );
    let mut water = Mask::new(x as i64, y as i64, w as usize, h as usize);
    let wave: Vec<(f32, f32)> = (0..=12)
        .map(|k| {
            let t = k as f32 / 12.0;
            (
                col[0] + t * 22.0 * s,
                row(river_row) - 4.5 * s + (t * 6.3).sin() * 2.0 * s,
            )
        })
        .collect();
    water.polyline(&wave, 1.6 * s);
    c.paint(&water, [60, 110, 170], 1.0);
    let mine_row = border_row + 1;
    text(
        c,
        f,
        &body,
        "Mine, pass, peak",
        col[1] + 34.0 * s,
        row(mine_row),
        INK,
    );
    let mut ink = Mask::new(x as i64, y as i64, w as usize, h as usize);
    let (mx, my) = (col[1] + 5.0 * s, row(mine_row) - 4.5 * s);
    let r = 3.4 * s;
    ink.segment((mx - r, my - r), (mx + r, my + r), 1.3 * s);
    ink.segment((mx - r, my + r), (mx + r, my - r), 1.3 * s);
    let px = col[1] + 15.0 * s;
    ink.polyline(
        &[
            (px - 2.2 * s, my - 4.0 * s),
            (px - 1.2 * s, my),
            (px - 2.2 * s, my + 4.0 * s),
        ],
        1.2 * s,
    );
    ink.polyline(
        &[
            (px + 2.2 * s, my - 4.0 * s),
            (px + 1.2 * s, my),
            (px + 2.2 * s, my + 4.0 * s),
        ],
        1.2 * s,
    );
    let tx = col[1] + 25.0 * s;
    ink.triangle(
        (tx, my - 4.6 * s),
        (tx - 4.2 * s, my + 2.8 * s),
        (tx + 4.2 * s, my + 2.8 * s),
    );
    c.paint(&ink, [70, 46, 30], 1.0);
    scale_bar(c, v, f, (x + 20.0 * s, y + h - 44.0 * s, w - 40.0 * s));
}
