//! Anti-aliased vector text with embedded Noto Serif and Noto Serif Display
//! (SIL OFL 1.1; see `assets/fonts/OFL.txt`).
//!
//! The text cut sets labels, which stay sturdy at small sizes; the display
//! cut sets the large realm names and the title. The faces are Latin subsets with their pair kerning flattened into
//! a legacy `kern` table, which `ab_glyph` reads. Text is laid out with
//! kerning and optional tracking; small capitals are drawn as capitals at
//! a reduced size, the usual cartographic substitute.

use super::mask::Mask;
use crate::error::SettleError;
use ab_glyph::{point, Font, FontRef, GlyphId, PxScale, ScaleFont};

static REGULAR: &[u8] = include_bytes!("../../assets/fonts/NotoSerif-Regular.ttf");
static ITALIC: &[u8] = include_bytes!("../../assets/fonts/NotoSerif-Italic.ttf");
static MEDIUM: &[u8] = include_bytes!("../../assets/fonts/NotoSerif-Medium.ttf");
static BOLD: &[u8] = include_bytes!("../../assets/fonts/NotoSerif-Bold.ttf");
static DISPLAY: &[u8] = include_bytes!("../../assets/fonts/NotoSerifDisplay-Regular.ttf");
static DISPLAY_BOLD: &[u8] = include_bytes!("../../assets/fonts/NotoSerifDisplay-Bold.ttf");

/// Small capitals are this share of the full size.
const SMALL_CAPS: f32 = 0.78;

/// A typeface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// Upright roman.
    Regular,
    /// Italic (water, glosses).
    Italic,
    /// Medium weight (towns).
    Medium,
    /// Bold (cities).
    Bold,
    /// Display roman (realms).
    Display,
    /// Display bold (the title).
    DisplayBold,
}

/// Letter case treatment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Case {
    /// As written.
    AsIs,
    /// All capitals.
    Upper,
    /// Capitals, with lowercase letters as small capitals.
    SmallCaps,
}

/// How a run of text looks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// Typeface.
    pub face: Face,
    /// Pixel height (ascent to descent).
    pub size: f32,
    /// Extra space after each letter, in ems.
    pub tracking: f32,
    /// Case treatment.
    pub case: Case,
}

impl Style {
    /// A plain style.
    #[must_use]
    pub const fn new(face: Face, size: f32) -> Self {
        Self {
            face,
            size,
            tracking: 0.0,
            case: Case::AsIs,
        }
    }

    /// With tracking in ems.
    #[must_use]
    pub const fn tracked(mut self, em: f32) -> Self {
        self.tracking = em;
        self
    }

    /// With a case treatment.
    #[must_use]
    pub const fn cased(mut self, case: Case) -> Self {
        self.case = case;
        self
    }
}

/// One positioned glyph of a laid-out run.
#[derive(Debug, Clone, Copy)]
pub struct Placed {
    /// Glyph.
    pub id: GlyphId,
    /// Its pixel height.
    pub size: f32,
    /// Pen position, pixels from the run's start.
    pub x: f32,
    /// Advance, pixels.
    pub advance: f32,
}

/// The embedded faces.
pub struct Fonts {
    faces: [FontRef<'static>; 6],
}

impl Fonts {
    /// Parses the embedded fonts.
    ///
    /// # Errors
    /// [`SettleError::Png`] if a font fails to parse (a build defect).
    pub fn new() -> Result<Self, SettleError> {
        let parse = |b: &'static [u8]| {
            FontRef::try_from_slice(b).map_err(|e| SettleError::Png(format!("font: {e}")))
        };
        Ok(Self {
            faces: [
                parse(REGULAR)?,
                parse(ITALIC)?,
                parse(MEDIUM)?,
                parse(BOLD)?,
                parse(DISPLAY)?,
                parse(DISPLAY_BOLD)?,
            ],
        })
    }

    fn font(&self, f: Face) -> &FontRef<'static> {
        &self.faces[match f {
            Face::Regular => 0,
            Face::Italic => 1,
            Face::Medium => 2,
            Face::Bold => 3,
            Face::Display => 4,
            Face::DisplayBold => 5,
        }]
    }

    /// Ascent above the baseline, pixels.
    #[must_use]
    pub fn ascent(&self, s: &Style) -> f32 {
        self.font(s.face).as_scaled(PxScale::from(s.size)).ascent()
    }

    /// Descent below the baseline, pixels (positive).
    #[must_use]
    pub fn descent(&self, s: &Style) -> f32 {
        -self.font(s.face).as_scaled(PxScale::from(s.size)).descent()
    }

    /// Height of a capital letter, pixels (drawn text's visual top).
    #[must_use]
    pub fn cap_height(&self, s: &Style) -> f32 {
        self.ascent(s) * 0.72
    }

    /// Lays out `text`; returns the glyphs and the total advance.
    #[must_use]
    pub fn layout(&self, s: &Style, text: &str) -> (Vec<Placed>, f32) {
        let font = self.font(s.face);
        let mut out = Vec::with_capacity(text.len());
        let mut x = 0.0_f32;
        let mut prev: Option<(GlyphId, f32)> = None;
        for c in text.chars() {
            let (ch, size) = match s.case {
                Case::AsIs => (c, s.size),
                Case::Upper => (c.to_ascii_uppercase(), s.size),
                Case::SmallCaps if c.is_lowercase() => {
                    (c.to_ascii_uppercase(), s.size * SMALL_CAPS)
                }
                Case::SmallCaps => (c, s.size),
            };
            let scaled = font.as_scaled(PxScale::from(size));
            let id = scaled.glyph_id(ch);
            if let Some((p, psize)) = prev {
                if (psize - size).abs() < f32::EPSILON {
                    x += scaled.kern(p, id);
                }
            }
            let advance = scaled.h_advance(id);
            out.push(Placed {
                id,
                size,
                x,
                advance,
            });
            x += advance + if c == ' ' { 0.0 } else { s.tracking * s.size };
            prev = Some((id, size));
        }
        let width = out.last().map_or(0.0, |p| p.x + p.advance).max(0.0);
        (out, width)
    }

    /// Advance width of `text`, pixels.
    #[must_use]
    pub fn width(&self, s: &Style, text: &str) -> f32 {
        self.layout(s, text).1
    }

    /// Draws `text` with its baseline starting at `(x, baseline)`.
    pub fn draw(&self, m: &mut Mask, s: &Style, text: &str, x: f32, baseline: f32) {
        let (glyphs, _) = self.layout(s, text);
        for g in glyphs {
            self.glyph(m, s.face, &g, x + g.x, baseline);
        }
    }

    /// Draws one glyph with its pen at `(x, baseline)`.
    pub fn glyph(&self, m: &mut Mask, face: Face, g: &Placed, x: f32, baseline: f32) {
        let glyph =
            g.id.with_scale_and_position(PxScale::from(g.size), point(x, baseline));
        if let Some(o) = self.font(face).outline_glyph(glyph) {
            let b = o.px_bounds();
            o.draw(|gx, gy, c| {
                m.add(
                    b.min.x as i64 + i64::from(gx),
                    b.min.y as i64 + i64::from(gy),
                    c,
                );
            });
        }
    }

    /// Draws one glyph centred on `(cx, cy)` along the baseline direction
    /// `angle` (radians), the pen's baseline passing through the centre
    /// line at `rise` pixels below the glyph's middle.
    #[allow(clippy::too_many_arguments)]
    pub fn glyph_rotated(
        &self,
        m: &mut Mask,
        face: Face,
        g: &Placed,
        cx: f32,
        cy: f32,
        angle: f32,
        rise: f32,
    ) {
        // Draw upright into a scratch mask around the origin, then sample
        // it rotated into `m`.
        let r = (g.size * 1.2).ceil() as i64;
        let side = usize::try_from(2 * r + 1).unwrap_or(1);
        let mut tmp = Mask::new(-r, -r, side, side);
        self.glyph(&mut tmp, face, g, -g.advance / 2.0, rise);
        let (sin, cos) = angle.sin_cos();
        for dy in -r..=r {
            for dx in -r..=r {
                let (px, py) = (cx.floor() as i64 + dx, cy.floor() as i64 + dy);
                let (fx, fy) = (px as f32 + 0.5 - cx, py as f32 + 0.5 - cy);
                // Inverse rotation back into the upright glyph.
                let ux = fx * cos + fy * sin;
                let uy = -fx * sin + fy * cos;
                let c = tmp.sample(ux, uy);
                if c > 0.0 {
                    m.add(px, py, c);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_kerned_tracked_and_drawn() {
        let f = Fonts::new().unwrap();
        let s = Style::new(Face::Regular, 40.0);
        let w = f.width(&s, "AV");
        let solid = f.width(&s, "A") + f.width(&s, "V");
        assert!(w < solid, "kerning pulls AV together ({w} vs {solid})");
        assert!(f.width(&s.tracked(0.2), "AV") > w + 7.0);
        let small = f.width(&s.cased(Case::SmallCaps), "ab");
        let caps = f.width(&s.cased(Case::Upper), "ab");
        assert!(small < caps);
        let mut m = Mask::new(0, 0, 200, 60);
        f.draw(&mut m, &s, "Oakford", 2.0, 45.0);
        let ink: f32 = m.a.iter().sum();
        assert!(ink > 100.0);
        assert!(m.a.iter().any(|&c| c > 0.0 && c < 1.0), "anti-aliased");
    }
}
