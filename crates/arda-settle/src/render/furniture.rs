//! Page furniture: a neatline, a title cartouche with a north arrow, a
//! legend of every symbol, and a scale bar, set in whichever corner holds
//! the least land.

use super::font::{Case, Face, Fonts, Style};
use super::labels::{Placer, Rect};
use super::layers::{road_style, Symbols, INK};
use super::mask::Mask;
use super::View;
use crate::canvas::Canvas;
use crate::model::Tier;
use crate::roads::RoadClass;

/// What the cartouche says.
pub struct Title {
    /// Main title.
    pub title: String,
    /// Italic subtitle.
    pub subtitle: String,
    /// Small print (seed, size, counts).
    pub note: String,
}

/// Size of the furniture block at scale `s`: (width, title height, legend height).
fn block(s: f32) -> (f32, f32, f32) {
    (360.0 * s, 150.0 * s, 262.0 * s)
}

/// Chooses the corner whose furniture block covers the least land and
/// reserves it and the margins; returns the block's top-left corner.
pub fn place(c: &Canvas, v: &View, pl: &mut Placer<'_>, land: &[bool]) -> (f32, f32) {
    let s = v.s;
    let (w, h) = (c.width as f32, c.height as f32);
    let (bw, title_h, legend_h) = block(s);
    let margin = 44.0 * s;
    let bh = title_h + 14.0 * s + legend_h;
    let corners = [
        (margin, margin),
        (w - margin - bw, margin),
        (margin, h - margin - bh),
        (w - margin - bw, h - margin - bh),
    ];
    let land_under = |x: f32, y: f32| {
        let mut n = 0_u32;
        let mut yy = y;
        while yy < y + bh {
            let mut xx = x;
            while xx < x + bw {
                let (px, py) = (xx as usize, yy as usize);
                n += u32::from(land.get(py * c.width + px).copied().unwrap_or(false));
                xx += 8.0;
            }
            yy += 8.0;
        }
        n
    };
    let (x, y) = corners
        .iter()
        .copied()
        .min_by_key(|&(x, y)| land_under(x, y))
        .unwrap_or(corners[0]);
    pl.reserve(Rect::at(
        x - 6.0 * s,
        y - 6.0 * s,
        bw + 12.0 * s,
        bh + 12.0 * s,
    ));
    let edge = 26.0 * s;
    for r in [
        Rect::at(0.0, 0.0, w, edge),
        Rect::at(0.0, h - edge, w, edge),
        Rect::at(0.0, 0.0, edge, h),
        Rect::at(w - edge, 0.0, edge, h),
    ] {
        pl.reserve(r);
    }
    (x, y)
}

/// Draws the neatline, cartouche, legend and scale bar with the block at
/// `at` (from [`place`]).
pub fn draw(c: &mut Canvas, v: &View, fonts: &Fonts, at: (f32, f32), t: &Title) {
    let s = v.s;
    let (bw, title_h, legend_h) = block(s);
    neatline(c, s);
    let (x, y) = at;
    cartouche(c, v, fonts, (x, y, bw, title_h), t);
    legend(c, v, fonts, (x, y + title_h + 14.0 * s, bw, legend_h));
}

/// A paper margin with a heavy and a fine rule.
fn neatline(c: &mut Canvas, s: f32) {
    let (w, h) = (c.width as f32, c.height as f32);
    let mut paper = c.mask();
    let band = 14.0 * s;
    for y in 0..c.height {
        for x in 0..c.width {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let d = fx.min(fy).min(w - fx).min(h - fy);
            if d < band + 0.5 {
                paper.add(x as i64, y as i64, (band + 0.5 - d).clamp(0.0, 1.0));
            }
        }
    }
    c.paint(&paper, [236, 226, 202], 1.0);
    let mut rule = c.mask();
    rect_stroke(
        &mut rule,
        band,
        band,
        w - 2.0 * band,
        h - 2.0 * band,
        2.6 * s,
    );
    rect_stroke(
        &mut rule,
        band + 5.0 * s,
        band + 5.0 * s,
        w - 2.0 * (band + 5.0 * s),
        h - 2.0 * (band + 5.0 * s),
        0.9 * s,
    );
    c.paint(&rule, [70, 50, 34], 1.0);
}

fn rect_stroke(m: &mut Mask, x: f32, y: f32, w: f32, h: f32, lw: f32) {
    let p = [(x, y), (x + w, y), (x + w, y + h), (x, y + h), (x, y)];
    m.polyline(&p, lw);
}

/// A paper panel with a double rule.
fn panel(c: &mut Canvas, b: (f32, f32, f32, f32), s: f32) {
    let (x, y, w, h) = b;
    let mut fill = Mask::new(x as i64 - 2, y as i64 - 2, w as usize + 4, h as usize + 4);
    for py in 0..fill.h {
        for px in 0..fill.w {
            let (fx, fy) = (
                fill.x0 as f32 + px as f32 + 0.5,
                fill.y0 as f32 + py as f32 + 0.5,
            );
            let d = (fx - x).min(fy - y).min(x + w - fx).min(y + h - fy);
            fill.a[py * fill.w + px] = (d + 0.5).clamp(0.0, 1.0);
        }
    }
    c.paint(&fill, [244, 236, 214], 0.93);
    let mut rule = Mask::new(x as i64 - 4, y as i64 - 4, w as usize + 8, h as usize + 8);
    rect_stroke(&mut rule, x, y, w, h, 1.8 * s);
    rect_stroke(
        &mut rule,
        x + 4.0 * s,
        y + 4.0 * s,
        w - 8.0 * s,
        h - 8.0 * s,
        0.7 * s,
    );
    c.paint(&rule, [70, 50, 34], 1.0);
}

fn text(
    c: &mut Canvas,
    f: &Fonts,
    st: &Style,
    s: &str,
    x: f32,
    baseline: f32,
    col: crate::canvas::Rgb,
) {
    let w = f.width(st, s);
    let mut m = Mask::new(
        x as i64 - 2,
        (baseline - st.size) as i64 - 2,
        w as usize + 6,
        (st.size * 1.4) as usize + 6,
    );
    f.draw(&mut m, st, s, x, baseline);
    c.paint(&m, col, 1.0);
}

fn cartouche(c: &mut Canvas, v: &View, f: &Fonts, b: (f32, f32, f32, f32), t: &Title) {
    let s = v.s;
    panel(c, b, s);
    let (x, y, w, _) = b;
    let title = Style::new(Face::DisplayBold, 38.0 * s)
        .tracked(0.06)
        .cased(Case::SmallCaps);
    let sub = Style::new(Face::Italic, 17.0 * s);
    let note = Style::new(Face::Regular, 12.5 * s);
    let cx = |st: &Style, s: &str| x + (w - f.width(st, s)) / 2.0;
    text(
        c,
        f,
        &title,
        &t.title,
        cx(&title, &t.title),
        y + 58.0 * s,
        [60, 36, 26],
    );
    text(
        c,
        f,
        &sub,
        &t.subtitle,
        cx(&sub, &t.subtitle),
        y + 90.0 * s,
        INK,
    );
    text(
        c,
        f,
        &note,
        &t.note,
        cx(&note, &t.note),
        y + 124.0 * s,
        [90, 70, 52],
    );
    // North arrow in the top right of the panel.
    let (ax, ay) = (x + w - 30.0 * s, y + 44.0 * s);
    let mut m = Mask::new(ax as i64 - 30, ay as i64 - 40, 60, 80);
    m.triangle(
        (ax, ay - 18.0 * s),
        (ax - 6.0 * s, ay + 6.0 * s),
        (ax, ay + 1.0 * s),
    );
    let mut hollow = Mask::new(ax as i64 - 30, ay as i64 - 40, 60, 80);
    hollow.triangle(
        (ax, ay - 18.0 * s),
        (ax + 6.0 * s, ay + 6.0 * s),
        (ax, ay + 1.0 * s),
    );
    c.paint(&hollow, [150, 120, 90], 1.0);
    c.paint(&m, [60, 36, 26], 1.0);
    let n = Style::new(Face::Bold, 13.0 * s);
    text(
        c,
        f,
        &n,
        "N",
        ax - f.width(&n, "N") / 2.0,
        ay + 20.0 * s,
        [60, 36, 26],
    );
}

fn legend(c: &mut Canvas, v: &View, f: &Fonts, b: (f32, f32, f32, f32)) {
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
    let places = [
        (Tier::City, false, "City"),
        (Tier::Town, false, "Town"),
        (Tier::Village, false, "Village"),
        (Tier::Hamlet, false, "Hamlet"),
        (Tier::Town, true, "Realm seat"),
    ];
    for (k, &(tier, seat, label)) in places.iter().enumerate() {
        sym.draw((col[0] + 10.0 * s, row(k) - 4.5 * s), tier, seat, s);
        text(c, f, &body, label, col[0] + 28.0 * s, row(k), INK);
    }
    sym.paint(c);
    for (k, (class, label)) in [
        (RoadClass::Highway, "Highway"),
        (RoadClass::Road, "Road"),
        (RoadClass::Track, "Track"),
        (RoadClass::Footpath, "Footpath"),
    ]
    .into_iter()
    .enumerate()
    {
        let (cw, cc, fw, fc, dash) = road_style(class);
        let yy = row(k) - 4.5 * s;
        let seg = [(col[1], yy), (col[1] + 26.0 * s, yy)];
        let mut m = Mask::new(x as i64, y as i64, w as usize, h as usize);
        if cw > 0.0 {
            m.polyline(&seg, cw * s);
            c.paint(&m, cc, 0.95);
            m.clear();
        }
        match dash {
            Some((on, off)) => m.dashed(&seg, fw * s, on * s, off * s, &mut 0.0),
            None => m.polyline(&seg, fw * s),
        }
        c.paint(&m, fc, 1.0);
        text(c, f, &body, label, col[1] + 34.0 * s, row(k), INK);
    }
    // Border, mine, pass, peak.
    let yy = row(4) - 4.5 * s;
    let mut m = Mask::new(x as i64, y as i64, w as usize, h as usize);
    m.dashed(
        &[(col[1], yy), (col[1] + 26.0 * s, yy)],
        1.9 * s,
        9.0 * s,
        4.5 * s,
        &mut 0.0,
    );
    c.paint(&m, [92, 30, 78], 0.95);
    text(c, f, &body, "Realm border", col[1] + 34.0 * s, row(4), INK);
    let italic = Style::new(Face::Italic, 13.0 * s);
    text(
        c,
        f,
        &italic,
        "River",
        col[0] + 28.0 * s,
        row(5),
        [40, 84, 140],
    );
    let mut water = Mask::new(x as i64, y as i64, w as usize, h as usize);
    let wave: Vec<(f32, f32)> = (0..=12)
        .map(|k| {
            let t = k as f32 / 12.0;
            (
                col[0] + t * 22.0 * s,
                row(5) - 4.5 * s + (t * 6.3).sin() * 2.0 * s,
            )
        })
        .collect();
    water.polyline(&wave, 1.6 * s);
    c.paint(&water, [60, 110, 170], 1.0);
    text(
        c,
        f,
        &body,
        "Mine, pass, peak",
        col[1] + 34.0 * s,
        row(5),
        INK,
    );
    let mut ink = Mask::new(x as i64, y as i64, w as usize, h as usize);
    let (mx, my) = (col[1] + 5.0 * s, row(5) - 4.5 * s);
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

/// Alternating 5 km blocks to a round length that fits `b.2` pixels.
fn scale_bar(c: &mut Canvas, v: &View, f: &Fonts, b: (f32, f32, f32)) {
    let s = v.s;
    let (x, y, w) = b;
    let px_per_km = v.px_per_km();
    let km = [
        100.0_f32, 50.0, 40.0, 30.0, 25.0, 20.0, 15.0, 10.0, 5.0, 2.0, 1.0,
    ]
    .into_iter()
    .find(|&k| k * px_per_km <= w * 0.85)
    .unwrap_or(1.0);
    let n = if km == 15.0 || km == 30.0 { 6 } else { 5 };
    let step = km / n as f32;
    let seg = step * px_per_km;
    let hgt = 5.0 * s;
    let x0 = x + (w - km * px_per_km) / 2.0;
    let mut dark = Mask::new(x as i64 - 4, y as i64 - 30, w as usize + 8, 60);
    let mut frame = Mask::new(x as i64 - 4, y as i64 - 30, w as usize + 8, 60);
    for k in 0..n {
        let xa = x0 + k as f32 * seg;
        if k % 2 == 0 {
            for yy in 0..(hgt.ceil() as i64) {
                dark.segment((xa, y + yy as f32), (xa + seg, y + yy as f32), 1.0);
            }
        }
    }
    rect_stroke(&mut frame, x0, y, km * px_per_km, hgt, 0.9 * s);
    c.paint(&dark, [60, 40, 28], 1.0);
    c.paint(&frame, [60, 40, 28], 1.0);
    let st = Style::new(Face::Regular, 11.5 * s);
    for (k, label) in [(0.0, "0".to_string()), (km, format!("{} km", km as u32))] {
        let tx = x0 + k * px_per_km - f.width(&st, &label) / 2.0;
        text(c, f, &st, &label, tx, y + hgt + 15.0 * s, INK);
    }
}
