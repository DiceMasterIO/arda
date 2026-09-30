//! `arda-settle render`: the society overlay on the Atlas overview (spec
//! step 9, goal 41).
//!
//! The overview comes from `arda::export_overview_with_quality_and_style`
//! in Atlas style. Over it go quiet land-use tints, a soft tint per realm
//! that deepens into a ribbon along its borders, dashed realm borders,
//! roads by class with casings, settlement, mine, pass and peak symbols,
//! labels set by a greedy placer in anti-aliased serif type with halos
//! (realms in letter-spaced small capitals, rivers in italic along their
//! courses), and page furniture: a neatline, a title cartouche, a legend
//! and a scale bar. Sizes scale with the image: `s = 1` at 2,048 pixels
//! across.
//!
//! Drawing is floating point by nature; the casts between pixel indices
//! and coordinates are bounded by the canvas size.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]

pub mod font;
pub mod furniture;
pub mod guard;
pub mod labels;
pub mod layers;
pub mod mask;
mod text;

use crate::canvas::Canvas;
use crate::error::SettleError;
use crate::num::iu;
use crate::output::{self, NamesFile, RealmsFile, RoadsFile, SettlementsFile};
use std::path::Path;

/// Pixel geometry of the overlay: a window of `pw × ph` pixels at
/// `(ox, oy)` in a full overview of `fw × fh` pixels.
#[derive(Debug, Clone, Copy)]
pub struct View {
    /// Page width, pixels.
    pub pw: f32,
    /// Page height, pixels.
    pub ph: f32,
    /// Full overview width, pixels.
    pub fw: f32,
    /// Full overview height, pixels.
    pub fh: f32,
    /// Page offset in the full overview, pixels.
    pub ox: f32,
    /// Page offset in the full overview, pixels.
    pub oy: f32,
    /// Grid width, cells.
    pub gw: usize,
    /// Grid height, cells.
    pub gh: usize,
    /// Symbol and type scale (1 at 2,048 pixels across).
    pub s: f32,
}

impl View {
    /// Metres to page pixels.
    #[must_use]
    pub fn px(&self, m: [i64; 2]) -> (f32, f32) {
        (
            m[0] as f32 * self.fw / (self.gw as f32 * 100.0) - self.ox,
            m[1] as f32 * self.fh / (self.gh as f32 * 100.0) - self.oy,
        )
    }

    /// The grid cell under a page pixel.
    #[must_use]
    pub fn cell_at(&self, px: usize, py: usize) -> usize {
        let x = ((px as f32 + self.ox + 0.5) * self.gw as f32 / self.fw) as usize;
        let y = ((py as f32 + self.oy + 0.5) * self.gh as f32 / self.fh) as usize;
        y.min(self.gh - 1) * self.gw + x.min(self.gw - 1)
    }

    /// Pixels per kilometre.
    #[must_use]
    pub fn px_per_km(&self) -> f32 {
        self.fw / (self.gw as f32 / 10.0)
    }
}

/// Everything the overlay reads from `society/`.
pub struct Society {
    /// Settlements.
    pub settlements: SettlementsFile,
    /// Roads, crossings and passes.
    pub roads: RoadsFile,
    /// Realms.
    pub realms: RealmsFile,
    /// Rivers, mountains and regions.
    pub names: NamesFile,
    /// Land-use codes.
    pub codes: Vec<u8>,
    /// Realm per cell.
    pub realm_map: Vec<u16>,
    /// Grid size in cells.
    pub size: (usize, usize),
}

impl Society {
    /// Reads `world_dir/society/`.
    ///
    /// # Errors
    /// JSON, raster and I/O errors.
    pub fn read(world_dir: &Path) -> Result<Self, SettleError> {
        let soc = world_dir.join(crate::SOCIETY_DIR);
        let (gw, gh, codes, _) = output::read_landuse(&soc.join("landuse.bin"))?;
        let (rw, rh, realm_map) = output::read_realm_map(&soc.join("realms.bin"))?;
        if (rw, rh) != (gw, gh) {
            return Err(SettleError::Format {
                path: soc.join("realms.bin").display().to_string(),
                reason: "realm map size differs from the land use",
            });
        }
        Ok(Self {
            settlements: output::read_json(&soc.join("settlements.json"))?,
            roads: output::read_json(&soc.join("roads.json"))?,
            realms: output::read_json(&soc.join("realms.json"))?,
            names: output::read_json(&soc.join("names.json"))?,
            codes,
            realm_map,
            size: (gw, gh),
        })
    }
}

/// Renders the overlay for `world` to `out`. With `full` the page is the
/// whole overview; otherwise it is cropped to the land with a margin and
/// room at one side for the legend.
///
/// # Errors
/// Load, export, PNG and I/O errors.
pub fn render(world_dir: &Path, out: &Path, quality: u32, full: bool) -> Result<(), SettleError> {
    let world = arda::World::load(world_dir)?;
    let q = arda::ImageQuality::new(quality).map_err(|e| SettleError::Png(e.to_string()))?;
    let tmp = guard::scratch_dir(&std::env::temp_dir())?;
    let png_path =
        arda::export_overview_with_quality_and_style(&world, &tmp, q, arda::MapStyle::Atlas);
    let base = png_path.map_err(SettleError::from).and_then(|p| decode(&p));
    let _ = std::fs::remove_dir_all(&tmp);
    let mut canvas = base?;
    let society = Society::read(world_dir)?;
    draw(&mut canvas, &society, world.seed(), full)?;
    if let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| SettleError::io(dir, e))?;
    }
    encode(out, &canvas)
}

/// The page window: the land's bounding box with a margin, widened on the
/// side with more sea to hold the legend.
fn page(fw: usize, fh: usize, soc: &Society) -> (usize, usize, usize, usize) {
    let (gw, gh) = soc.size;
    let (mut x0, mut y0, mut x1, mut y1) = (gw, gh, 0, 0);
    for (i, &r) in soc.realm_map.iter().enumerate() {
        if r != 0 {
            let (x, y) = (i % gw, i / gw);
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
        }
    }
    if x1 <= x0 || y1 <= y0 {
        return (0, 0, fw, fh);
    }
    let sx = |x: usize| (x * fw / gw) as f32;
    let sy = |y: usize| (y * fh / gh) as f32;
    let s = (fw as f32 / 2048.0).clamp(0.4, 8.0);
    let (m, need) = (60.0 * s, 470.0 * s);
    let (bx0, bx1, by0, by1) = (sx(x0), sx(x1), sy(y0), sy(y1));
    let (mut px0, mut px1) = ((bx0 - m).max(0.0), (bx1 + m).min(fw as f32));
    if bx0 >= fw as f32 - bx1 {
        px0 = (bx0 - need).max(0.0);
    } else {
        px1 = (bx1 + need).min(fw as f32);
    }
    let (py0, py1) = ((by0 - m).max(0.0), (by1 + m).min(fh as f32));
    let (ox, oy) = (px0 as usize, py0 as usize);
    (
        ox,
        oy,
        (px1 as usize).saturating_sub(ox).max(1),
        (py1 as usize).saturating_sub(oy).max(1),
    )
}

/// Draws every layer of the overlay onto `c` (the full overview), cropping
/// it to the page first unless `full`.
///
/// # Errors
/// [`SettleError::Format`] for inconsistent or off-world inputs
/// ([`guard::check`]); [`SettleError::Png`] if the embedded fonts fail to
/// parse.
pub fn draw(c: &mut Canvas, soc: &Society, seed: u64, full: bool) -> Result<(), SettleError> {
    guard::check(soc)?;
    let (gw, gh) = soc.size;
    let (fw, fh) = (c.width, c.height);
    let (ox, oy, w, h) = if full {
        (0, 0, fw, fh)
    } else {
        page(fw, fh, soc)
    };
    if (ox, oy, w, h) != (0, 0, fw, fh) {
        let mut rgb = Vec::with_capacity(w * h * 3);
        for y in oy..oy + h {
            rgb.extend_from_slice(&c.rgb[(y * fw + ox) * 3..(y * fw + ox + w) * 3]);
        }
        *c = Canvas {
            width: w,
            height: h,
            rgb,
        };
    }
    let v = View {
        pw: w as f32,
        ph: h as f32,
        fw: fw as f32,
        fh: fh as f32,
        ox: ox as f32,
        oy: oy as f32,
        gw,
        gh,
        s: (fw as f32 / 2048.0).clamp(0.4, 8.0),
    };
    layers::land_use(c, &v, &soc.codes);
    let rp = layers::realm_pixels(c, &v, &soc.realm_map);
    let anchors = layers::realm_tints(c, &v, &rp);
    layers::realm_borders(c, &v, &soc.realms);
    layers::roads(c, &v, &soc.roads);
    let fonts = font::Fonts::new()?;
    let mut pl = labels::Placer::new(&fonts, v.pw, v.ph, 2.3 * v.s);
    let land: Vec<bool> = rp.iter().map(|&r| r != 0).collect();
    let at = furniture::place(c, &v, &mut pl, &land);
    let seats: Vec<u64> = soc.realms.realms.iter().map(|r| r.seat).collect();
    layers::settlements(c, &v, &soc.settlements, &seats, &mut pl);
    let peaks = text::peaks(&v, &soc.names);
    let peak_px: Vec<(f32, f32)> = peaks.iter().map(|p| p.at).collect();
    layers::features(c, &v, &soc.codes, &soc.roads, &peak_px, &mut pl);
    let inks = text::labels(&v, soc, &anchors, &peaks, &mut pl);
    labels::paint(c, &inks, layers::PAPER, 0.82);
    let title = text::title(soc, seed, &v);
    furniture::draw(c, &v, &fonts, at, &title);
    Ok(())
}

fn decode(path: &Path) -> Result<Canvas, SettleError> {
    let file = std::fs::File::open(path).map_err(|e| SettleError::io(path, e))?;
    let mut dec = png::Decoder::new(std::io::BufReader::new(file));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec
        .read_info()
        .map_err(|e| SettleError::Png(e.to_string()))?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| SettleError::Png(e.to_string()))?;
    let (w, h) = (iu(i64::from(info.width)), iu(i64::from(info.height)));
    let channels = info.color_type.samples();
    let mut rgb = Vec::with_capacity(w * h * 3);
    for px in buf[..w * h * channels].chunks_exact(channels) {
        match channels {
            1 | 2 => rgb.extend_from_slice(&[px[0], px[0], px[0]]),
            _ => rgb.extend_from_slice(&px[..3]),
        }
    }
    Ok(Canvas {
        width: w,
        height: h,
        rgb,
    })
}

fn encode(path: &Path, c: &Canvas) -> Result<(), SettleError> {
    let file = std::fs::File::create(path).map_err(|e| SettleError::io(path, e))?;
    let (w, h) = (crate::num::u32_of(c.width), crate::num::u32_of(c.height));
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc
        .write_header()
        .map_err(|e| SettleError::Png(e.to_string()))?;
    writer
        .write_image_data(&c.rgb)
        .map_err(|e| SettleError::Png(e.to_string()))
}
