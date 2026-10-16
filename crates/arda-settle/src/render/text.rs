//! What the overlay says and in which type: realm, settlement, river and
//! peak labels in placement order, and the cartouche's title.

use super::font::{Case, Face, Style};
use super::furniture::Title;
use super::labels::Rect;
use super::labels::{Ink, Line, Placer};
use super::layers::{shade, symbol_r, Anchors, INK};
use super::mask::chaikin;
use super::palette::Palette;
use super::tilelabels::{Labeler, HALO_OPACITY};
use super::{Society, View};
use crate::canvas::Rgb;
use crate::gazetteer::PlaceKind;
use crate::model::Tier;
use crate::output::NamesFile;

/// Italic brown of glosses and heights.
const GLOSS: Rgb = [104, 78, 56];
/// How far a realm's own colour is darkened for its name (1.0 = as is).
const REALM_INK: f32 = 0.62;
/// Water blue.
const WATER: Rgb = [28, 68, 128];
/// Peaks named, highest first.
const MAX_PEAKS: usize = 12;
/// Rivers shorter than this go unnamed, metres.
const RIVER_MIN_M: u64 = 12_000;

/// A peak to mark and name.
pub struct Peak {
    /// Pixel position.
    pub at: (f32, f32),
    /// Name.
    pub name: String,
    /// Height, metres.
    pub height_m: i32,
}

/// The highest peaks.
#[must_use]
pub fn peaks(v: &View, names: &NamesFile) -> Vec<Peak> {
    let mut list: Vec<_> = names.mountains.iter().collect();
    list.sort_by(|a, b| b.height_m.cmp(&a.height_m).then(a.name.cmp(&b.name)));
    list.into_iter()
        .take(MAX_PEAKS)
        .map(|m| Peak {
            at: v.px(m.at_m),
            name: m.name.clone(),
            height_m: m.height_m,
        })
        .collect()
}

fn line(text: &str, style: Style, color: Rgb) -> Line {
    Line {
        text: text.to_string(),
        style,
        color,
    }
}

/// Thousands with a comma.
fn thousands(n: i32) -> String {
    let s = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (k, ch) in s.chars().enumerate() {
        if k > 0 && (s.len() - k).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Every label, most important first: realms, cities, towns, rivers,
/// peaks, then villages as room allows.
pub fn labels(
    v: &View,
    soc: &Society,
    (anchors, palette): (&Anchors, &Palette),
    peaks: &[Peak],
    pl: &mut Placer<'_>,
) -> Vec<Ink> {
    let s = v.s;
    let mut inks = Vec::new();
    for &(id, at) in anchors {
        let Some(r) = soc.realms.realms.iter().find(|r| r.id == u64::from(id)) else {
            continue;
        };
        let col = shade(palette.colour(u64::from(id)), REALM_INK);
        let lines = [
            line(
                &r.name,
                Style::new(Face::Display, 42.0 * s)
                    .tracked(0.3)
                    .cased(Case::SmallCaps),
                col,
            ),
            line(&r.name_gloss, Style::new(Face::Italic, 16.0 * s), col),
        ];
        // Nudge the label up or down its realm's heartland until it is clear.
        let steps = [0.0, -1.0, 1.0, -2.0, 2.0, -3.0, 3.0];
        let placed = steps
            .iter()
            .find_map(|&k| pl.centred(&lines, (at.0, at.1 + k * 34.0 * s), false))
            .or_else(|| pl.centred(&lines, at, true));
        if let Some(mut ink) = placed {
            ink.opacity = 0.9;
            inks.push(ink);
        }
    }
    let mut towns: Vec<_> = soc
        .settlements
        .settlements
        .iter()
        .filter(|x| x.tier.is_urban())
        .collect();
    towns.sort_by_key(|x| (std::cmp::Reverse(x.population), x.id));
    for x in towns {
        let (name, gloss) = if x.tier == Tier::City {
            (
                Style::new(Face::Bold, 22.0 * s)
                    .tracked(0.06)
                    .cased(Case::Upper),
                Style::new(Face::Italic, 13.5 * s),
            )
        } else {
            (
                Style::new(Face::Medium, 17.0 * s),
                Style::new(Face::Italic, 12.5 * s),
            )
        };
        let lines = [line(&x.name, name, INK), line(&x.name_gloss, gloss, GLOSS)];
        let r = symbol_r(x.tier) * s;
        if let Some(ink) = pl.point(&lines, v.px([x.x_m, x.y_m]), r, x.tier == Tier::City) {
            inks.push(ink);
        }
    }
    let mut rivers: Vec<_> = soc
        .names
        .rivers
        .iter()
        .filter(|r| r.length_m >= RIVER_MIN_M && r.course_m.len() > 3)
        .collect();
    rivers.sort_by_key(|r| (std::cmp::Reverse(r.length_m), r.id));
    let river_style = Style::new(Face::Italic, 15.0 * s).tracked(0.12);
    for r in rivers {
        let path: Vec<(f32, f32)> = r.course_m.iter().map(|&m| v.px(m)).collect();
        let path = chaikin(&path, 3);
        let l = line(&r.name, river_style, WATER);
        if let Some(ink) = pl.along(&l, &path, 5.0 * s) {
            inks.push(ink);
        }
    }
    for p in peaks {
        let lines = [
            line(&p.name, Style::new(Face::Italic, 12.5 * s), [70, 46, 30]),
            line(
                &format!("{} m", thousands(p.height_m)),
                Style::new(Face::Regular, 10.5 * s),
                GLOSS,
            ),
        ];
        if let Some(ink) = pl.point(&lines, p.at, 4.6 * s, false) {
            inks.push(ink);
        }
    }
    let mut villages: Vec<_> = soc
        .settlements
        .settlements
        .iter()
        .filter(|x| x.tier == Tier::Village)
        .collect();
    villages.sort_by_key(|x| (std::cmp::Reverse(x.population), x.id));
    for x in villages {
        let lines = [line(&x.name, Style::new(Face::Regular, 13.0 * s), INK)];
        let r = symbol_r(x.tier) * s;
        if let Some(ink) = pl.point(&lines, v.px([x.x_m, x.y_m]), r, false) {
            inks.push(ink);
        }
    }
    inks
}

/// The gazetteer's natural features (seas, lakes, forests, ranges, ...),
/// laid out around the labels already placed and painted over the page.
/// Rivers and settlements keep the labels above.
pub fn features(c: &mut crate::canvas::Canvas, v: &View, soc: &Society, pl: &Placer<'_>) {
    let Some(file) = &soc.gazetteer else {
        return;
    };
    let world = [v.gw as f64 * 100.0, v.gh as f64 * 100.0];
    let Ok(l) = Labeler::new(file, &[], world, world[0].max(world[1])) else {
        return;
    };
    let px_per_m = f64::from(v.fw) / world[0];
    // The placed boxes are page pixels; the layout works in the full image.
    let blocked: Vec<_> = pl
        .taken()
        .iter()
        .map(|r| Rect {
            x0: r.x0 + v.ox,
            y0: r.y0 + v.oy,
            x1: r.x1 + v.ox,
            y1: r.y1 + v.oy,
        })
        .collect();
    let level = l.level_at(px_per_m, v.s, &blocked, |i| {
        i.kind != PlaceKind::River && !i.kind.is_settlement()
    });
    let origin = (v.ox as i64, v.oy as i64);
    let size = (v.pw as usize, v.ph as usize);
    let inks = l.inks(&level, origin, size);
    for i in &inks {
        let mut m = i.halo.clone();
        m.x0 -= origin.0;
        m.y0 -= origin.1;
        c.paint(&m, i.halo_colour, HALO_OPACITY);
    }
    for i in &inks {
        let mut m = i.text.clone();
        m.x0 -= origin.0;
        m.y0 -= origin.1;
        c.paint(&m, i.colour, 1.0);
    }
}

/// The cartouche's words.
#[must_use]
pub fn title(soc: &Society, seed: u64, v: &View) -> Title {
    let names: Vec<&str> = soc.realms.realms.iter().map(|r| r.name.as_str()).collect();
    let subtitle = match names.as_slice() {
        [] => "An unclaimed land".to_string(),
        [one] => format!("The realm of {one}"),
        [rest @ .., last] => format!("The realms of {} and {last}", rest.join(", ")),
    };
    let count = |t: Tier| {
        soc.settlements
            .settlements
            .iter()
            .filter(|x| x.tier == t)
            .count()
    };
    let plural =
        |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    let note = format!(
        "Seed {seed}  \u{b7}  {} \u{d7} {} km  \u{b7}  {}, {}, {}, {}",
        v.gw / 10,
        v.gh / 10,
        plural(count(Tier::City), "city", "cities"),
        plural(count(Tier::Town), "town", "towns"),
        plural(count(Tier::Village), "village", "villages"),
        plural(count(Tier::Hamlet), "hamlet", "hamlets"),
    );
    Title {
        title: "Realms & Roads".to_string(),
        subtitle,
        note,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heights_read_with_thousands() {
        assert_eq!(thousands(1234), "1,234");
        assert_eq!(thousands(987), "987");
        assert_eq!(thousands(12_345), "12,345");
    }
}
