//! The society layer of the exports: land-use tone, road network and
//! settlement symbols painted on finished overview rows (task 15).
//!
//! [`SocietyBandOverlay`] implements [`arda_render::BandOverlay`]. It loads
//! `society/` once, prepares the road centrelines and symbol list in the
//! pixel space of the whole image, and then paints any band of rows in
//! vertical strips, so memory is a few strips of canvas and masks, never a
//! page. Every layer draws from geometry in whole-image coordinates (masks
//! are placed in them), so a strip, a band or the whole image give the very
//! same pixels; `tests/band.rs` pins that.
//!
//! What is drawn follows metres per pixel: land-use tone up to
//! [`LAND_MAX_M`]; highways and roads at any scale, tracks up to
//! [`TRACK_MAX_M`], footpaths up to [`FOOTPATH_MAX_M`]; settlement symbols
//! by tier, hamlets (and every road built to one) only up to
//! [`super::scale::HAMLET_MAX_M`], strokes scaled like the offline page (`s = 1` at 2,048 pixels
//! across, kept between 0.5 and 2) and symbols larger with the output width
//! ([`super::scale::symbol_scale`]: 1.6 at 2,048 pixels, up to 2.4). No
//! text: names are the gazetteer's.
//!
//! [`BandLayer::Political`] adds the political layer ([`super::political`]):
//! realm edge bands under the roads and dash-dot borders over them, in
//! ivory cased roads ([`RoadStyle::Ivory`]); [`BandLayer::Society`] keeps
//! the classic red and yellow roads.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]

use super::layers::{self, land_tone, RoadLines, RoadStyle, Sym};
use super::political::PoliticalLayer;
use super::{guard, Society, View};
use crate::canvas::Canvas;
use crate::error::SettleError;
use crate::landuse::code;
use crate::roads::RoadClass;
use arda_render::{BandOverlay, RenderError};
use std::path::Path;

/// Pixels (metres) above which land-use tone is not drawn.
pub const LAND_MAX_M: f32 = 400.0;
/// Pixels (metres) above which tracks are not drawn.
pub const TRACK_MAX_M: f32 = 30.0;
/// Pixels (metres) above which footpaths are not drawn.
pub const FOOTPATH_MAX_M: f32 = 10.0;
/// Columns per strip. Small on purpose: masks and canvases under the
/// allocator's mapping threshold are reused in place instead of being
/// retained as large freed blocks, which kept peak memory proportional to
/// the strip width.
const STRIP: usize = 128;
/// Strips painted at once.
const BATCH: usize = 8;

/// Which layers the band overlay paints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BandLayer {
    /// Land tone, classic roads and settlement symbols.
    #[default]
    Society,
    /// The society layer with ivory roads, plus realm edge bands and
    /// dash-dot borders.
    Political,
}

/// The realm per grid cell, narrow when every id fits a byte.
enum RealmCells {
    Narrow(Vec<u8>),
    Wide(Vec<u16>),
}

impl RealmCells {
    fn new(map: &[u16]) -> Self {
        if map.iter().all(|&r| r <= u16::from(u8::MAX)) {
            Self::Narrow(map.iter().map(|&r| u8::try_from(r).unwrap_or(0)).collect())
        } else {
            Self::Wide(map.to_vec())
        }
    }

    fn at(&self, cell: usize) -> u16 {
        match self {
            Self::Narrow(v) => v.get(cell).map_or(0, |&r| u16::from(r)),
            Self::Wide(v) => v.get(cell).copied().unwrap_or(0),
        }
    }
}

/// Land, roads and settlements over the finished rows of an overview.
pub struct SocietyBandOverlay {
    codes: Vec<u8>,
    view: View,
    lines: RoadLines,
    syms: Vec<Sym>,
    classes: Vec<RoadClass>,
    /// Scale of settlement symbols ([`super::scale::symbol_scale`]).
    sym_s: f32,
    /// Samples per pixel axis for the land tone; 0 draws none.
    land_k: usize,
    /// Road look.
    road_style: RoadStyle,
    /// The political layer and the realm per cell, when asked for.
    political: Option<(PoliticalLayer, RealmCells)>,
}

impl SocietyBandOverlay {
    /// Prepares the society overlay of a `width × height` overview.
    ///
    /// # Errors
    /// [`SettleError::Format`] for inconsistent or off-world society files.
    pub fn new(soc: Society, width: u32, height: u32) -> Result<Self, SettleError> {
        Self::with_layer(soc, width, height, BandLayer::Society)
    }

    /// Prepares the overlay of a `width × height` overview with `layer`.
    ///
    /// # Errors
    /// [`SettleError::Format`] for inconsistent or off-world society files.
    pub fn with_layer(
        soc: Society,
        width: u32,
        height: u32,
        layer: BandLayer,
    ) -> Result<Self, SettleError> {
        guard::check(&soc)?;
        let (gw, gh) = soc.size;
        let (fw, fh) = (width as f32, height as f32);
        let view = View {
            pw: fw,
            ph: fh,
            fw,
            fh,
            ox: 0.0,
            oy: 0.0,
            gw,
            gh,
            s: (fw / 2048.0).clamp(0.5, 2.0),
        };
        let m_per_px = (gw as f32 * 100.0 / fw).max(gh as f32 * 100.0 / fh);
        let mut classes = vec![RoadClass::Road, RoadClass::Highway];
        if m_per_px <= TRACK_MAX_M {
            classes.insert(0, RoadClass::Track);
        }
        if m_per_px <= FOOTPATH_MAX_M {
            classes.insert(0, RoadClass::Footpath);
        }
        let seats: Vec<u64> = soc.realms.realms.iter().map(|r| r.seat).collect();
        let hamlets = super::scale::shows_hamlets(m_per_px);
        let skip = if hamlets {
            std::collections::BTreeSet::new()
        } else {
            layers::hamlet_ids(&soc.settlements)
        };
        let (road_style, political) = match layer {
            BandLayer::Society => (RoadStyle::Classic, None),
            BandLayer::Political => {
                let palette = super::palette::Palette::of(&soc.realms);
                (
                    RoadStyle::Ivory,
                    Some((
                        PoliticalLayer::new(&view, &soc.realms, palette),
                        RealmCells::new(&soc.realm_map),
                    )),
                )
            }
        };
        Ok(Self {
            lines: layers::road_lines(&view, &soc.roads, &skip),
            syms: layers::symbol_list(&view, &soc.settlements, &seats, hamlets),
            codes: soc.codes,
            view,
            classes,
            sym_s: super::scale::symbol_scale(fw),
            land_k: if m_per_px > LAND_MAX_M {
                0
            } else {
                (m_per_px / 100.0).ceil().clamp(1.0, 4.0) as usize
            },
            road_style,
            political,
        })
    }

    /// The scale settlement symbols are drawn at: they grow with the
    /// output width ([`super::scale::symbol_scale`]) while road strokes keep
    /// the page scale (1 at 2,048 pixels).
    #[must_use]
    pub const fn symbol_scale(&self) -> f32 {
        self.sym_s
    }

    /// Reads `world_dir/society/` and prepares the overlay.
    ///
    /// # Errors
    /// Society read errors and [`SettleError::Format`].
    pub fn open(world_dir: &Path, width: u32, height: u32) -> Result<Self, SettleError> {
        Self::new(Society::read(world_dir)?, width, height)
    }

    /// Reads `world_dir/society/` and prepares the overlay with `layer`.
    ///
    /// # Errors
    /// Society read errors and [`SettleError::Format`].
    pub fn open_with(
        world_dir: &Path,
        (width, height): (u32, u32),
        layer: BandLayer,
    ) -> Result<Self, SettleError> {
        Self::with_layer(Society::read(world_dir)?, width, height, layer)
    }

    /// The realm colours of the political layer, when it is painted.
    #[must_use]
    pub fn palette(&self) -> Option<&super::palette::Palette> {
        self.political.as_ref().map(|(p, _)| p.palette())
    }

    /// Paints the window of the image whose top-left pixel is `origin` onto
    /// `c`: land tone, realm bands, roads, border lines, then symbols.
    pub fn paint_window(&self, c: &mut Canvas, origin: (usize, usize)) {
        self.paint_with(c, origin, &self.lines, &self.syms);
    }

    fn paint_with(&self, c: &mut Canvas, origin: (usize, usize), lines: &RoadLines, syms: &[Sym]) {
        let v = View {
            pw: c.width as f32,
            ph: c.height as f32,
            ox: origin.0 as f32,
            oy: origin.1 as f32,
            ..self.view
        };
        if self.land_k > 0 {
            land_tone_sampled(c, &v, &self.codes, self.land_k);
        }
        let o = (origin.0 as i64, origin.1 as i64);
        if let Some((layer, cells)) = &self.political {
            layer.paint_band(c, &v, &|cell| cells.at(cell));
        }
        layers::paint_roads(c, lines, (v.s, self.road_style), o, &self.classes);
        if let Some((layer, _)) = &self.political {
            layer.paint_lines(c, o);
        }
        layers::paint_symbols(c, syms, self.sym_s, o);
    }
}

/// Land-use tone averaged over `k × k` samples per pixel (`k = 1` is
/// [`layers::land_use`]); coarse pixels blend the tones of the cells they
/// cover instead of aliasing one cell.
fn land_tone_sampled(c: &mut Canvas, v: &View, codes: &[u8], k: usize) {
    // Representative codes of the four tones.
    const TONES: [u8; 4] = [code::ARABLE, code::FALLOW, code::MEADOW, code::BUILT];
    let tone_of = |cell: u8| match cell {
        code::ARABLE | code::ORCHARD => Some(0),
        code::FALLOW => Some(1),
        code::MEADOW => Some(2),
        code::BUILT => Some(3),
        _ => None,
    };
    let n = (k * k) as f32;
    for py in 0..c.height {
        for px in 0..c.width {
            let mut count = [0_u32; 4];
            for j in 0..k {
                for i in 0..k {
                    let x = (px as f32 + v.ox + (i as f32 + 0.5) / k as f32) * v.gw as f32 / v.fw;
                    let y = (py as f32 + v.oy + (j as f32 + 0.5) / k as f32) * v.gh as f32 / v.fh;
                    let cell = (y as usize).min(v.gh - 1) * v.gw + (x as usize).min(v.gw - 1);
                    if let Some(t) = tone_of(codes[cell]) {
                        count[t] += 1;
                    }
                }
            }
            let (x, y) = (px as i64, py as i64);
            for (t, &cnt) in count.iter().enumerate() {
                if cnt == 0 {
                    continue;
                }
                if let Some((multiply, colour, alpha)) = land_tone(TONES[t]) {
                    let a = alpha * cnt as f32 / n;
                    if multiply {
                        c.multiply(x, y, colour, a);
                    } else {
                        c.blend(x, y, colour, a);
                    }
                }
            }
        }
    }
}

impl BandOverlay for SocietyBandOverlay {
    fn paint(&self, y0: u32, rows: &mut [u8], width: u32) -> Result<(), RenderError> {
        let w = width as usize;
        if w == 0 || !rows.len().is_multiple_of(w * 3) {
            return Err(RenderError::Overlay {
                reason: "rows are not a whole number of image rows".into(),
            });
        }
        let h = rows.len() / (w * 3);
        // Only what meets these rows goes on to the strips.
        let (top, bottom) = (y0 as f32, (y0 as usize + h) as f32);
        let pad = 8.0 * self.view.s + 2.0;
        let lines = RoadLines {
            by_class: std::array::from_fn(|k| {
                self.lines.by_class[k]
                    .iter()
                    .filter(|l| l.bbox[1] <= bottom + pad && l.bbox[3] >= top - pad)
                    .cloned()
                    .collect()
            }),
        };
        let syms: Vec<Sym> = self
            .syms
            .iter()
            .filter(|x| x.at.1 + 14.0 * self.sym_s >= top && x.at.1 - 14.0 * self.sym_s <= bottom)
            .copied()
            .collect();
        let (lines, syms) = (&lines, &syms);
        let starts: Vec<usize> = (0..w).step_by(STRIP).collect();
        for batch in starts.chunks(BATCH) {
            let done: Vec<(usize, Canvas)> = std::thread::scope(|scope| {
                let jobs: Vec<_> = batch
                    .iter()
                    .map(|&x0| {
                        let sw = STRIP.min(w - x0);
                        let src: &[u8] = rows;
                        scope.spawn(move || {
                            let mut rgb = Vec::with_capacity(sw * h * 3);
                            for r in 0..h {
                                rgb.extend_from_slice(
                                    &src[(r * w + x0) * 3..(r * w + x0 + sw) * 3],
                                );
                            }
                            let mut c = Canvas {
                                width: sw,
                                height: h,
                                rgb,
                            };
                            self.paint_with(&mut c, (x0, y0 as usize), lines, syms);
                            (x0, c)
                        })
                    })
                    .collect();
                jobs.into_iter().filter_map(|j| j.join().ok()).collect()
            });
            if done.len() != batch.len() {
                return Err(RenderError::Overlay {
                    reason: "a painting thread failed".into(),
                });
            }
            for (x0, c) in done {
                for r in 0..h {
                    rows[(r * w + x0) * 3..(r * w + x0 + c.width) * 3]
                        .copy_from_slice(&c.rgb[r * c.width * 3..(r + 1) * c.width * 3]);
                }
            }
        }
        Ok(())
    }
}
