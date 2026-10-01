//! Debug render (goal 05 step 8): a flat-colour top-down view with
//! hill shading, 5-ft contour lines, water by depth and placement dots.
//! For checking only; the real art is `arda-tactical`'s compositor.

use crate::error::RefineError;
use crate::grid::span;
use crate::layout::AssetRef;
use crate::output::Map;

/// Flat colour for a ground key.
#[must_use]
pub fn ground_colour(key: &str) -> [u8; 3] {
    match key {
        "grass" => [106, 148, 72],
        "meadow" => [142, 168, 84],
        "forest_floor" => [72, 96, 52],
        "leaf_litter" => [128, 112, 64],
        "heath" => [128, 98, 118],
        "scrub" => [100, 118, 62],
        "moss" => [86, 124, 78],
        "scree" => [156, 150, 140],
        "rock" => [124, 121, 115],
        "cliff" => [78, 72, 70],
        "sand" => [222, 204, 152],
        "gravel" => [172, 162, 142],
        "mud" => [112, 92, 66],
        "marsh" => [96, 118, 86],
        "reed_bed" => [152, 160, 92],
        "snow" => [240, 242, 248],
        "ice" => [202, 226, 240],
        "dirt" => [142, 116, 82],
        "salt_crust" => [238, 234, 224],
        "mudflat" => [196, 178, 146],
        "water_shallow" => [96, 156, 192],
        "water_deep" => [44, 94, 152],
        _ => [255, 0, 255],
    }
}

struct Canvas {
    w: usize,
    h: usize,
    px: Vec<[f64; 3]>,
}

impl Canvas {
    fn blend(&mut self, x: i64, y: i64, c: [u8; 3], a: f64) {
        let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
            return;
        };
        if x >= self.w || y >= self.h {
            return;
        }
        let p = &mut self.px[y * self.w + x];
        for k in 0..3 {
            p[k] = p[k] * (1.0 - a) + f64::from(c[k]) * a;
        }
    }

    fn disc(&mut self, cx: f64, cy: f64, r: f64, c: [u8; 3], a: f64) {
        #[allow(clippy::cast_possible_truncation)]
        let (x0, x1, y0, y1) = (
            (cx - r).floor() as i64,
            (cx + r).ceil() as i64,
            (cy - r).floor() as i64,
            (cy + r).ceil() as i64,
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (dx, dy) = (x as f64 + 0.5 - cx, y as f64 + 0.5 - cy);
                let d = (dx * dx + dy * dy).sqrt();
                if d <= r {
                    let edge = (r - d).min(1.0);
                    self.blend(x, y, c, a * edge);
                }
            }
        }
    }
}

fn asset_id(a: &AssetRef) -> &str {
    match a {
        AssetRef::Id(s) => s.as_str(),
        AssetRef::Query { .. } => "",
    }
}

/// Renders a map at `scale` pixels per square to PNG bytes.
///
/// # Errors
/// PNG encoding failure.
pub fn render_png(map: &Map, scale: u32) -> Result<Vec<u8>, RefineError> {
    let l = &map.layout;
    let (w, h, s) = (l.width as usize, l.height as usize, scale.max(1) as usize);
    let mut cv = Canvas {
        w: w * s,
        h: h * s,
        px: vec![[0.0; 3]; w * s * h * s],
    };
    // Hill shade reads a 5 x 5 box-blurred elevation so the 5-ft terraces
    // do not band the shading.
    let raw = |x: i64, y: i64| {
        let cx = usize::try_from(x.clamp(0, span(w) - 1)).unwrap_or(0);
        let cy = usize::try_from(y.clamp(0, span(h) - 1)).unwrap_or(0);
        f64::from(l.squares[cy * w + cx].elevation_ft)
    };
    let mut smooth = vec![0.0; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut s = 0.0;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    s += raw(span(x) + dx, span(y) + dy);
                }
            }
            smooth[y * w + x] = s / 25.0;
        }
    }
    let elev = |x: usize, y: usize| smooth[y.min(h - 1) * w + x.min(w - 1)];
    for y in 0..h {
        for x in 0..w {
            let sq = &l.squares[y * w + x];
            let mut c = ground_colour(&sq.ground).map(f64::from);
            if sq.water_depth_ft > 0 {
                let t = (f64::from(sq.water_depth_ft) / 14.0).min(1.0);
                c = [
                    c[0] * (1.0 - 0.45 * t),
                    c[1] * (1.0 - 0.35 * t),
                    c[2] * (1.0 - 0.15 * t),
                ];
            } else {
                // Hill shade from the north-west.
                let gx = (elev(x + 1, y) - elev(x.saturating_sub(1), y)) / 10.0;
                let gy = (elev(x, y + 1) - elev(x, y.saturating_sub(1))) / 10.0;
                let shade = (1.0 - 0.22 * (gx + gy)).clamp(0.6, 1.3);
                c = c.map(|v| (v * shade).min(255.0));
            }
            for py in 0..s {
                for px in 0..s {
                    cv.px[(y * s + py) * w * s + x * s + px] = c;
                }
            }
        }
    }
    // Contour lines on every 5-ft step, drawn on the lower square's edge.
    for y in 0..h {
        for x in 0..w {
            let e = l.squares[y * w + x].elevation_ft;
            let dry = l.squares[y * w + x].water_depth_ft == 0;
            let step = |o: usize| {
                let n = &l.squares[o];
                dry && n.water_depth_ft == 0 && n.elevation_ft != e
            };
            let major = |o: usize| {
                let n = l.squares[o].elevation_ft;
                (n.max(e) / 25) != (n.min(e) / 25)
            };
            if x + 1 < w && step(y * w + x + 1) {
                let a = if major(y * w + x + 1) { 0.45 } else { 0.12 };
                for py in 0..s {
                    cv.blend(span((x + 1) * s) - 1, span(y * s + py), [40, 30, 20], a);
                }
            }
            if y + 1 < h && step((y + 1) * w + x) {
                let a = if major((y + 1) * w + x) { 0.45 } else { 0.12 };
                for px in 0..s {
                    cv.blend(span(x * s + px), span((y + 1) * s) - 1, [40, 30, 20], a);
                }
            }
        }
    }
    let sf = s as f64;
    let mut order: Vec<usize> = (0..l.placements.len()).collect();
    // Low things first, canopies last.
    let layer = |id: &str| {
        if id.contains("tree_") {
            3
        } else if id.contains("boulder") || id.contains("rock_large") {
            2
        } else {
            1
        }
    };
    order.sort_by_key(|&i| layer(asset_id(&l.placements[i].asset)));
    for i in order {
        let p = &l.placements[i];
        let id = asset_id(&p.asset);
        let (cx, cy) = (f64::from(p.x) * sf, f64::from(p.y) * sf);
        match id {
            "veg.tree_pine" | "veg.tree_spruce" => {
                cv.disc(cx, cy, 1.25 * sf, [34, 70, 48], 0.85);
                cv.disc(cx, cy, 0.5 * sf, [24, 52, 36], 0.9);
            }
            "veg.tree_dead" => cv.disc(cx, cy, 0.9 * sf, [110, 100, 90], 0.8),
            "veg.tree_birch" => cv.disc(cx, cy, 0.95 * sf, [108, 150, 74], 0.8),
            "veg.tree_willow" => cv.disc(cx, cy, 1.4 * sf, [110, 146, 80], 0.75),
            x if x.starts_with("veg.tree_") => {
                cv.disc(cx, cy, 1.5 * sf, [52, 92, 44], 0.82);
                cv.disc(cx - 0.3 * sf, cy - 0.3 * sf, 0.7 * sf, [78, 120, 58], 0.5);
            }
            "veg.bush" | "veg.bush_flowering" | "veg.fern" => {
                let c = if id == "veg.bush_flowering" {
                    [150, 110, 140]
                } else {
                    [70, 110, 50]
                };
                cv.disc(cx, cy, 0.55 * sf, c, 0.8);
            }
            "veg.boulder" => {
                cv.disc(cx, cy, 0.8 * sf, [60, 58, 55], 0.9);
                cv.disc(cx, cy, 0.62 * sf, [150, 146, 138], 1.0);
            }
            "veg.rock_large" => cv.disc(cx, cy, 0.5 * sf, [110, 106, 100], 0.95),
            "veg.rock_small" | "veg.stones" | "veg.scree_patch" => {
                cv.disc(cx, cy, 0.25 * sf, [100, 96, 92], 0.9);
            }
            "veg.fallen_log" | "veg.stump" => {
                let (dx, dy) = if p.rotation.is_multiple_of(180) {
                    (1.0, 0.0)
                } else {
                    (0.0, 1.0)
                };
                let len = if id == "veg.stump" { 0.0 } else { 1.1 };
                for k in 0..=10 {
                    let t = f64::from(k) / 10.0 * 2.0 - 1.0;
                    cv.disc(
                        cx + dx * t * len * sf,
                        cy + dy * t * len * sf,
                        0.28 * sf,
                        [96, 70, 44],
                        0.9,
                    );
                }
            }
            "veg.reeds" | "veg.cattail" => cv.disc(cx, cy, 0.32 * sf, [176, 170, 96], 0.9),
            "veg.lily_pads" => cv.disc(cx, cy, 0.4 * sf, [90, 150, 80], 0.85),
            "veg.flower_patch" => cv.disc(cx, cy, 0.3 * sf, [220, 190, 90], 0.8),
            "veg.heather" => cv.disc(cx, cy, 0.35 * sf, [150, 90, 150], 0.8),
            _ => cv.disc(cx, cy, 0.25 * sf, [150, 170, 90], 0.7),
        }
    }
    let mut bytes = Vec::new();
    {
        let wu = u32::try_from(cv.w).unwrap_or(u32::MAX);
        let hu = u32::try_from(cv.h).unwrap_or(u32::MAX);
        let mut enc = png::Encoder::new(&mut bytes, wu, hu);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header()?;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let data: Vec<u8> = cv
            .px
            .iter()
            .flat_map(|p| p.map(|v| v.round().clamp(0.0, 255.0) as u8))
            .collect();
        writer.write_image_data(&data)?;
    }
    Ok(bytes)
}
