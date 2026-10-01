//! Synthetic "raw AI" images: the defects real generator output has, made
//! in code so the tests need no binary fixtures.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    dead_code
)]

use arda_tactical::noise::hash2;
use arda_tactical::Rgba;
use std::path::Path;

fn grain(seed: u64, x: u32, y: u32, amp: u32) -> i32 {
    (hash2(seed, i64::from(x), i64::from(y)) % u64::from(2 * amp + 1)) as i32 - amp as i32
}

fn clamp(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

/// An off-white backdrop with faint noise (what "on a white background"
/// prompts give).
fn backdrop(w: u32, h: u32) -> Rgba {
    let mut img = Rgba::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let n = grain(1, x, y, 2);
            img.set(x, y, [clamp(246 + n), clamp(245 + n), clamp(241 + n), 255]);
        }
    }
    img
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A barrel-ish disc on white with a soft drop shadow falling SE and a
/// pale glow halo around it.
pub fn barrel_on_white() -> Rgba {
    let (w, h) = (620, 540);
    let mut img = backdrop(w, h);
    let (cx, cy, r) = (290.0f32, 250.0f32, 150.0f32);
    // Soft shadow: the backdrop darkened, offset SE, feathered edge.
    let (sx, sy) = (cx + 55.0, cy + 48.0);
    for y in 0..h {
        for x in 0..w {
            let d = ((x as f32 - sx).powi(2) + (y as f32 - sy).powi(2)).sqrt();
            let k = 1.0 - 0.5 * (1.0 - smooth((d - r * 0.85) / 40.0));
            let p = img.get(x, y);
            img.set(
                x,
                y,
                [
                    clamp((f32::from(p[0]) * k) as i32),
                    clamp((f32::from(p[1]) * k) as i32),
                    clamp((f32::from(p[2]) * k) as i32),
                    255,
                ],
            );
        }
    }
    for y in 0..h {
        for x in 0..w {
            let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
            let p = img.get(x, y);
            if d < r - 4.0 {
                // Staves and hoops, lit from the top-left.
                let lit = 1.0 - 0.35 * ((x as f32 - cx) + (y as f32 - cy)) / (2.0 * r);
                let stave = if ((x as f32 - cx).abs() as u32 / 18).is_multiple_of(2) {
                    12
                } else {
                    -8
                };
                let hoop = ((d - r * 0.55).abs() < 6.0) as i32 * -50;
                let base = [138.0, 88.0, 46.0].map(|c: f32| c * lit);
                let n = grain(2, x, y, 6) + stave + hoop;
                img.set(
                    x,
                    y,
                    [
                        clamp(base[0] as i32 + n),
                        clamp(base[1] as i32 + n),
                        clamp(base[2] as i32 + n),
                        255,
                    ],
                );
            } else if d < r {
                img.set(x, y, [40, 28, 18, 255]); // ink outline
            } else if d < r + 8.0 {
                // Glow halo: a pale yellow wash.
                let t = 0.45 * (1.0 - (d - r) / 8.0);
                let g = [255.0, 248.0, 200.0];
                let px: [u8; 3] =
                    std::array::from_fn(|c| clamp((f32::from(p[c]) * (1.0 - t) + g[c] * t) as i32));
                img.set(x, y, [px[0], px[1], px[2], 255]);
            }
        }
    }
    img
}

/// A bush already cut out (alpha), with a soft grey shadow baked into the
/// alpha, falling SE.
pub fn bush_with_alpha_shadow() -> Rgba {
    let (w, h) = (400, 400);
    let mut img = Rgba::new(w, h);
    let (cx, cy, r) = (180.0f32, 180.0f32, 110.0f32);
    for y in 0..h {
        for x in 0..w {
            let ds = ((x as f32 - cx - 45.0).powi(2) + (y as f32 - cy - 40.0).powi(2)).sqrt();
            let a = 0.55 * (1.0 - smooth((ds - r * 0.8) / 35.0));
            if a > 0.01 {
                img.set(x, y, [30, 30, 32, (a * 255.0) as u8]);
            }
            let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
            let wobble = grain(5, x / 9, y / 9, 8) as f32;
            if d < r + wobble {
                let n = grain(6, x, y, 14);
                img.set(
                    x,
                    y,
                    [clamp(58 + n), clamp(112 + n), clamp(44 + n / 2), 255],
                );
            }
        }
    }
    img
}

/// A non-tileable, wrongly sized grass photo: a strong diagonal gradient
/// with tufts.
pub fn grass_raw(seed: u64) -> Rgba {
    let (w, h) = (700, 500);
    let mut img = Rgba::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let ramp = (x * 50 / w + y * 30 / h) as i32;
            let n = grain(seed, x, y, 18) + grain(seed + 1, x / 3, y / 5, 10);
            img.set(
                x,
                y,
                [
                    clamp(62 + ramp + n / 2),
                    clamp(104 + ramp + n),
                    clamp(40 + n / 3),
                    255,
                ],
            );
        }
    }
    img
}

/// A cobble layout (structured): stones in a running bond. `shift` rolls
/// the layout, `tint` changes the stone colour.
pub fn cobbles_raw(shift: (u32, u32), tint: i32) -> Rgba {
    let s = 600;
    let mut img = Rgba::new(s, s);
    for y in 0..s {
        for x in 0..s {
            let (xx, yy) = ((x + shift.0) % s, (y + shift.1) % s);
            let row = yy / 50;
            let xo = (xx + row % 2 * 37) % 75;
            let grout = yy % 50 < 7 || xo < 7;
            let n = grain(9 + tint as u64, x, y, 10);
            let v = if grout { 70 + n / 2 } else { 150 + tint + n };
            img.set(x, y, [clamp(v), clamp(v - 8), clamp(v - 22), 255]);
        }
    }
    img
}

/// A stone corner drawn the wrong way round: arms west and north instead
/// of east and south, on a white canvas framed as one square.
pub fn corner_wrong_way() -> Rgba {
    let s = 400u32;
    let mut img = backdrop(s, s);
    let c = s / 2;
    let t = 48u32;
    for y in 0..s {
        for x in 0..s {
            let block = x.abs_diff(c) < 70 && y.abs_diff(c) < 70;
            let west = x <= c && y.abs_diff(c) < t;
            let north = y <= c && x.abs_diff(c) < t;
            if block || west || north {
                let n = grain(11, x, y, 12);
                img.set(x, y, [clamp(128 + n), clamp(124 + n), clamp(112 + n), 255]);
            }
        }
    }
    img
}

/// A small crate on a grey studio backdrop.
pub fn crate_on_grey() -> Rgba {
    let mut img = Rgba::filled(300, 300, [128, 128, 130, 255]);
    for y in 70..230 {
        for x in 70..230 {
            let plank = if (y / 20) % 2 == 0 { 10 } else { -10 };
            let edge = if !(78..=221).contains(&x) || !(78..=221).contains(&y) {
                -60
            } else {
                0
            };
            let n = grain(13, x, y, 6) + plank + edge;
            img.set(x, y, [clamp(170 + n), clamp(126 + n), clamp(74 + n), 255]);
        }
    }
    img
}

/// Writes an RGBA PNG with optional text chunks.
pub fn write_png(img: &Rgba, path: &Path, text: &[(&str, &str)]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let file = std::fs::File::create(path).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), img.width, img.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    for (k, v) in text {
        enc.add_text_chunk((*k).to_string(), (*v).to_string())
            .unwrap();
    }
    let mut w = enc.write_header().unwrap();
    w.write_image_data(&img.data).unwrap();
}

/// Writes an RGB PNG (no alpha channel at all, like most generator output).
pub fn write_rgb_png(img: &Rgba, path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let file = std::fs::File::create(path).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), img.width, img.height);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let rgb: Vec<u8> = img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    let mut w = enc.write_header().unwrap();
    w.write_image_data(&rgb).unwrap();
}

/// The ComfyUI `prompt` graph a real export carries.
pub const COMFY_GRAPH: &str = r#"{
  "3": {"class_type": "KSampler", "inputs": {"seed": 4242, "steps": 4}},
  "4": {"class_type": "UNETLoader", "inputs": {"unet_name": "flux1-schnell.safetensors"}},
  "6": {"class_type": "CLIPTextEncode", "inputs": {"text": "top-down wooden crate, game asset, flat even lighting, no shadow"}}
}"#;

/// Writes the whole synthetic raw set into `dir`.
pub fn write_raw_set(dir: &Path) {
    write_rgb_png(
        &barrel_on_white(),
        &dir.join("props/prop.barrel__comfy_00007_.png"),
    );
    write_png(
        &bush_with_alpha_shadow(),
        &dir.join("vegetation/veg.bush.png"),
        &[],
    );
    write_rgb_png(&grass_raw(21), &dir.join("ground/ground.grass.a.png"));
    write_rgb_png(&grass_raw(22), &dir.join("ground/ground.grass.b.png"));
    write_rgb_png(
        &cobbles_raw((0, 0), 0),
        &dir.join("ground/ground.cobbles.a.png"),
    );
    write_rgb_png(
        &cobbles_raw((41, 23), 18),
        &dir.join("ground/ground.cobbles.b.png"),
    );
    write_rgb_png(
        &corner_wrong_way(),
        &dir.join("walls/wall.stone.corner__take1.png"),
    );
    write_png(
        &crate_on_grey(),
        &dir.join("ComfyUI_00002_.png"),
        &[("prompt", COMFY_GRAPH)],
    );
    write_rgb_png(&crate_on_grey(), &dir.join("props/prop.anvill.png"));
    write_rgb_png(&crate_on_grey(), &dir.join("ComfyUI_00001_.png"));
}

/// A manifest for the raw set.
pub const MANIFEST: &str = r##"
[library]
name = "synthetic-ai"
version = "0.0.1"
licence = "CC0-1.0"
tool = "ComfyUI"
author = "arda tests"

[grade]
palette = ["#6f7f4a", "#8c7656", "#5c6650", "#a39a80"]
strength = 0.3

[[asset]]
file = "ComfyUI_00002_.png"
id = "prop.crate"

[[asset]]
id = "prop.barrel"
prompt = "top-down oak barrel with iron hoops, game asset"
seed = 77
"##;
