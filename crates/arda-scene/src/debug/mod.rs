//! A flat-colour debug view of a scene, for checking the data by eye.
//!
//! Legend:
//! - square fill: normal (cream), difficult (khaki, hatched), wade (light
//!   blue), swim (deep blue), impassable (grey); darker with elevation;
//! - obscurement: light (green tint), heavy (dark grey tint);
//! - cover pips in the square's top-left: one per degree (half, ¾, total);
//! - climb: short red bars on square edges that need climbing;
//! - regions: thin outlines (olive for difficult, blue for water);
//! - vision blockers: green canopy octagons, purple prop polygons;
//! - walls: black wall, cyan window, brown closed / green open door,
//!   orange gate, magenta secret door;
//! - lights: solid yellow bright radius, dashed orange dim radius;
//! - spawn hints: grey dots on open squares, teal rings by entrances, teal
//!   bars along edge exits.

mod draw;

use crate::error::SceneError;
use crate::types::{BlockerKind, CoverLevel, Movement, Obscurement, RegionKind, Scene, WallKind};
use arda_tactical::Rgba;
use draw::Pen;

/// Renders the debug view.
///
/// # Errors
/// `ppsq` outside 8–256, or an image too large.
#[allow(clippy::cast_precision_loss)]
pub fn scene_debug_image(scene: &Scene, ppsq: u32) -> Result<Rgba, SceneError> {
    if !(8..=256).contains(&ppsq) {
        return Err(SceneError::Options(format!("ppsq {ppsq} is outside 8–256")));
    }
    let (w, h) = (scene.width, scene.height);
    let (pw, ph) = (
        w.checked_mul(ppsq).filter(|v| *v <= 16384),
        h.checked_mul(ppsq).filter(|v| *v <= 16384),
    );
    let (Some(pw), Some(ph)) = (pw, ph) else {
        return Err(SceneError::Options("image larger than 16384 px".into()));
    };
    let mut img = Rgba::filled(pw, ph, [255, 255, 255, 255]);
    let mut pen = Pen(&mut img);
    let s = ppsq as f32;
    let px = |v: u32| v as f32 * s;

    for y in 0..h {
        for x in 0..w {
            let i = y as usize * w as usize + x as usize;
            let m = scene.movement.get(i).copied().unwrap_or_default();
            let (x0, y0) = (px(x), px(y));
            pen.rect(x0, y0, x0 + s, y0 + s, fill(m));
            let e = scene.elevation_ft.get(i).copied().unwrap_or(0);
            // Higher ground darker brown, lower ground bluer: 4 alpha per foot.
            let shade = u8::try_from((i32::from(e).abs() * 4).min(160)).unwrap_or(0);
            let tint = if e >= 0 { [90, 50, 10] } else { [20, 40, 90] };
            pen.rect(x0, y0, x0 + s, y0 + s, [tint[0], tint[1], tint[2], shade]);
            if m == Movement::Difficult {
                hatch(&mut pen, x0, y0, s);
            }
            match scene.obscured.get(i) {
                Some(Obscurement::Light) => pen.rect(x0, y0, x0 + s, y0 + s, [40, 140, 40, 50]),
                Some(Obscurement::Heavy) => pen.rect(x0, y0, x0 + s, y0 + s, [30, 30, 30, 120]),
                _ => {}
            }
            let pips = match scene.cover.get(i) {
                Some(CoverLevel::Half) => 1,
                Some(CoverLevel::ThreeQuarters) => 2,
                Some(CoverLevel::Total) => 3,
                _ => 0,
            };
            for p in 0..pips {
                let o = s * (0.08 + 0.16 * p as f32);
                pen.rect(
                    x0 + o,
                    y0 + s * 0.08,
                    x0 + o + s * 0.12,
                    y0 + s * 0.2,
                    [170, 20, 20, 255],
                );
            }
            let climb = scene.climb.get(i).copied().unwrap_or(0);
            climb_bars(&mut pen, climb, x0, y0, s);
        }
    }
    let t = (s / 32.0).max(1.0);
    for x in 0..=w {
        pen.line((px(x), 0.0), (px(x), px(h)), 1.0, [0, 0, 0, 40]);
    }
    for y in 0..=h {
        pen.line((0.0, px(y)), (px(w), px(y)), 1.0, [0, 0, 0, 40]);
    }
    for r in &scene.regions {
        let c = match r.kind {
            RegionKind::Difficult => [110, 110, 20, 230],
            RegionKind::ShallowWater => [40, 120, 200, 230],
            RegionKind::DeepWater => [10, 40, 120, 230],
        };
        for ring in &r.rings {
            let pts: Vec<(f32, f32)> = ring.iter().map(|p| (px(p[0]), px(p[1]))).collect();
            for k in 0..pts.len() {
                pen.line(pts[k], pts[(k + 1) % pts.len()], t * 1.5, c);
            }
        }
    }
    for b in &scene.vision_blockers {
        let pts: Vec<(f32, f32)> = b.polygon.iter().map(|p| (p[0] * s, p[1] * s)).collect();
        let (f, e) = match b.kind {
            BlockerKind::Canopy => ([30, 120, 30, 40], [20, 90, 20, 220]),
            BlockerKind::Prop => ([120, 40, 160, 60], [100, 20, 140, 230]),
        };
        pen.polygon(&pts, f, e, t * 1.5);
    }
    for wall in &scene.walls {
        let (c, th) = match (wall.kind, wall.open) {
            (WallKind::Wall, _) => ([20, 20, 20, 255], 4.0),
            (WallKind::Window, _) => ([0, 190, 220, 255], 3.0),
            (WallKind::Door, Some(true)) => ([40, 180, 40, 255], 3.0),
            (WallKind::Door, _) => ([140, 80, 20, 255], 4.0),
            (WallKind::Gate, _) => ([240, 130, 0, 255], 4.0),
            (WallKind::Secret, _) => ([220, 0, 200, 255], 4.0),
        };
        for pair in wall.points.windows(2) {
            let a = (px(pair[0][0]), px(pair[0][1]));
            let b = (px(pair[1][0]), px(pair[1][1]));
            let inset = if wall.kind.opens() { s * 0.12 } else { 0.0 };
            let (a, b) = shrink(a, b, inset);
            pen.line(a, b, t * th, c);
        }
    }
    for l in &scene.lights {
        let c = (l.x * s, l.y * s);
        pen.circle(
            c,
            f32::from(l.bright_ft) / 5.0 * s,
            t * 2.0,
            0.0,
            [230, 190, 0, 230],
        );
        pen.circle(
            c,
            f32::from(l.dim_ft) / 5.0 * s,
            t * 1.5,
            s * 0.3,
            [230, 120, 0, 200],
        );
        pen.disc(c, s * 0.12, [230, 170, 0, 255]);
    }
    let hints = &scene.spawn_hints;
    for q in &hints.open {
        pen.disc(
            (px(q.0) + s * 0.5, px(q.1) + s * 0.5),
            s * 0.06,
            [90, 90, 90, 200],
        );
    }
    for e in &hints.entrances {
        for q in &e.squares {
            let c = (px(q.0) + s * 0.5, px(q.1) + s * 0.5);
            pen.circle(c, s * 0.22, t * 1.5, 0.0, [0, 150, 140, 255]);
        }
    }
    for x in &hints.exits {
        let (a, b) = (x.from, x.to);
        let (mut x0, mut y0, mut x1, mut y1) = (px(a.0), px(a.1), px(b.0) + s, px(b.1) + s);
        let band = s * 0.15;
        match x.edge {
            crate::types::Edge::N => y1 = y0 + band,
            crate::types::Edge::S => y0 = y1 - band,
            crate::types::Edge::W => x1 = x0 + band,
            crate::types::Edge::E => x0 = x1 - band,
        }
        pen.rect(x0, y0, x1, y1, [0, 150, 140, 150]);
    }
    Ok(img)
}

/// Renders the debug view as PNG bytes.
///
/// # Errors
/// As [`scene_debug_image`], or PNG encoding failure.
pub fn scene_debug_png(scene: &Scene, ppsq: u32) -> Result<Vec<u8>, SceneError> {
    let img = scene_debug_image(scene, ppsq)?;
    draw::encode_png(&img).map_err(SceneError::Png)
}

fn fill(m: Movement) -> [u8; 4] {
    match m {
        Movement::Normal => [236, 230, 212, 255],
        Movement::Difficult => [214, 204, 150, 255],
        Movement::Wade => [160, 205, 235, 255],
        Movement::Swim => [60, 110, 190, 255],
        Movement::Impassable => [120, 115, 110, 255],
    }
}

#[allow(clippy::cast_precision_loss)]
fn hatch(pen: &mut Pen<'_>, x0: f32, y0: f32, s: f32) {
    for k in 1..4 {
        let o = s * k as f32 / 4.0;
        pen.line((x0 + o, y0), (x0, y0 + o), 1.0, [110, 100, 30, 150]);
        pen.line((x0 + s, y0 + o), (x0 + o, y0 + s), 1.0, [110, 100, 30, 150]);
    }
}

fn climb_bars(pen: &mut Pen<'_>, mask: u8, x0: f32, y0: f32, s: f32) {
    let c = [220, 30, 30, 230];
    let (a, b, th) = (s * 0.3, s * 0.7, (s / 16.0).max(1.0));
    let e = s * 0.06;
    for (bit, from, to) in [
        (0, (x0 + a, y0 + e), (x0 + b, y0 + e)),
        (2, (x0 + s - e, y0 + a), (x0 + s - e, y0 + b)),
        (4, (x0 + a, y0 + s - e), (x0 + b, y0 + s - e)),
        (6, (x0 + e, y0 + a), (x0 + e, y0 + b)),
    ] {
        if mask & (1 << bit) != 0 {
            pen.line(from, to, th, c);
        }
    }
}

fn shrink(a: (f32, f32), b: (f32, f32), d: f32) -> ((f32, f32), (f32, f32)) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    let (ux, uy) = (dx / len * d, dy / len * d);
    ((a.0 + ux, a.1 + uy), (b.0 - ux, b.1 - uy))
}
