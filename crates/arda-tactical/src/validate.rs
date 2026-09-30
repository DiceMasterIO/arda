//! The catalogue validator (goal 60).
//!
//! Every rule reports the asset it concerns and its own [`Rule`] name, so a
//! failing library produces an actionable list rather than the first error.

use crate::catalog::{AssetClass, Catalog, TagVocabulary, Tags, WallRole, FORMAT_VERSION};
use crate::raster::Rgba;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// A validation rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    /// `format_version` is not the one this crate reads, or ppsq is zero.
    FormatVersion,
    /// Two assets share an `id`.
    DuplicateId,
    /// `class` is not one of the known classes.
    UnknownClass,
    /// A controlled tag is missing from the catalogue vocabulary.
    UnknownTag,
    /// `licence` is empty.
    Licence,
    /// `provenance` is empty.
    Provenance,
    /// A rotation is not 0, 90, 180 or 270, or the list is empty.
    Rotation,
    /// A class-specific field is missing (`ground` key, `wall` piece).
    ClassFields,
    /// A wall kit lacks one of the required roles.
    WallKit,
    /// The image file is missing, unreadable or outside the library.
    ImageMissing,
    /// Image size differs from footprint × pixels per square.
    ImageSize,
    /// A cut-out asset (prop, wall, vegetation) is fully opaque.
    AlphaOpaque,
    /// Semi-transparent pixels stray too far from the opaque silhouette.
    AlphaFringe,
    /// A ground or water texture has transparent pixels.
    TextureAlpha,
    /// Opposite edges of a tileable texture do not match.
    TileSeam,
}

impl Rule {
    /// The rule's stable name, used in messages and documentation.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::FormatVersion => "format_version",
            Self::DuplicateId => "duplicate_id",
            Self::UnknownClass => "unknown_class",
            Self::UnknownTag => "unknown_tag",
            Self::Licence => "licence",
            Self::Provenance => "provenance",
            Self::Rotation => "rotation",
            Self::ClassFields => "class_fields",
            Self::WallKit => "wall_kit",
            Self::ImageMissing => "image_missing",
            Self::ImageSize => "image_size",
            Self::AlphaOpaque => "alpha_opaque",
            Self::AlphaFringe => "alpha_fringe",
            Self::TextureAlpha => "texture_alpha",
            Self::TileSeam => "tile_seam",
        }
    }
}

/// One validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// Asset id, `kit:<name>` for kit rules, or `<catalog>`.
    pub asset: String,
    /// The rule that was broken.
    pub rule: Rule,
    /// Human-readable detail.
    pub message: String,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: [{}] {}", self.asset, self.rule.name(), self.message)
    }
}

/// Tolerances for the image rules.
#[derive(Debug, Clone, Copy)]
pub struct Thresholds {
    /// Alpha at or above this counts as opaque.
    pub opaque_alpha: u8,
    /// Semi-transparent pixels may lie this many pixels from an opaque one.
    pub fringe_px: u32,
    /// Allowed fraction of stray semi-transparent pixels.
    pub max_stray_fraction: f32,
    /// Allowed ratio of mean seam difference to mean neighbour difference.
    pub max_seam_ratio: f32,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            opaque_alpha: 240,
            fringe_px: 2,
            max_stray_fraction: 0.005,
            max_seam_ratio: 2.0,
        }
    }
}

/// Loaded images by asset id; `Err` carries the load failure.
pub type Images = BTreeMap<String, Result<Rgba, String>>;

/// Runs every rule and returns all issues, sorted by asset then rule.
#[must_use]
pub fn validate(catalog: &Catalog, images: &Images, t: &Thresholds) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut push = |asset: &str, rule: Rule, message: String| {
        issues.push(Issue {
            asset: asset.to_string(),
            rule,
            message,
        });
    };
    if catalog.format_version != FORMAT_VERSION {
        let v = catalog.format_version;
        push(
            "<catalog>",
            Rule::FormatVersion,
            format!("version {v}, expected {FORMAT_VERSION}"),
        );
    }
    if catalog.pixels_per_square == 0 {
        push(
            "<catalog>",
            Rule::FormatVersion,
            "pixels_per_square must be positive".into(),
        );
    }
    let mut seen = BTreeSet::new();
    let mut kits: BTreeMap<&str, BTreeSet<WallRole>> = BTreeMap::new();
    for a in &catalog.assets {
        if !seen.insert(a.id.as_str()) {
            push(
                &a.id,
                Rule::DuplicateId,
                "id already used by an earlier asset".into(),
            );
        }
        if a.class == AssetClass::Unknown {
            push(
                &a.id,
                Rule::UnknownClass,
                "class must be ground, wall, prop, vegetation or water".into(),
            );
        }
        for msg in unknown_tags(&a.tags, &catalog.vocabulary) {
            push(&a.id, Rule::UnknownTag, msg);
        }
        if a.licence.trim().is_empty() {
            push(&a.id, Rule::Licence, "licence is required".into());
        }
        if a.provenance.trim().is_empty() {
            push(&a.id, Rule::Provenance, "provenance is required".into());
        }
        let mut rots = BTreeSet::new();
        if a.rotations.is_empty()
            || a.rotations
                .iter()
                .any(|r| r % 90 != 0 || *r >= 360 || !rots.insert(*r))
        {
            push(
                &a.id,
                Rule::Rotation,
                format!(
                    "rotations {:?} must be distinct values of 0, 90, 180, 270",
                    a.rotations
                ),
            );
        }
        if a.class.is_texture() && a.ground.is_none() {
            push(
                &a.id,
                Rule::ClassFields,
                "ground and water textures need a `ground` key".into(),
            );
        }
        match (&a.wall, a.class) {
            (Some(w), AssetClass::Wall) => {
                kits.entry(w.kit.as_str()).or_default().insert(w.role);
            }
            (None, AssetClass::Wall) => push(
                &a.id,
                Rule::ClassFields,
                "wall assets need a `wall` piece".into(),
            ),
            (Some(_), _) => push(
                &a.id,
                Rule::ClassFields,
                "only wall assets may carry a `wall` piece".into(),
            ),
            (None, _) => {}
        }
        match images.get(&a.id) {
            Some(Ok(img)) => image_rules(a, img, catalog.pixels_per_square, t, &mut push),
            Some(Err(e)) => push(&a.id, Rule::ImageMissing, e.clone()),
            None => push(
                &a.id,
                Rule::ImageMissing,
                format!("{} was not loaded", a.image),
            ),
        }
    }
    for (kit, roles) in &kits {
        let missing: Vec<_> = WallRole::REQUIRED
            .iter()
            .filter(|r| !roles.contains(r))
            .collect();
        if !missing.is_empty() {
            push(
                &format!("kit:{kit}"),
                Rule::WallKit,
                format!("missing roles {missing:?}"),
            );
        }
    }
    issues.sort_by(|a, b| (&a.asset, a.rule).cmp(&(&b.asset, b.rule)));
    issues
}

fn unknown_tags(tags: &Tags, vocab: &TagVocabulary) -> Vec<String> {
    let lists = [
        ("biome", &tags.biome, &vocab.biome),
        ("culture", &tags.culture, &vocab.culture),
        ("wealth", &tags.wealth, &vocab.wealth),
        ("function", &tags.function, &vocab.function),
    ];
    let mut out = Vec::new();
    for (kind, used, known) in lists {
        for tag in used.iter().filter(|t| !known.contains(t)) {
            out.push(format!(
                "{kind} tag `{tag}` is not in the catalogue vocabulary"
            ));
        }
    }
    out
}

fn image_rules(
    a: &crate::catalog::Asset,
    img: &Rgba,
    ppsq: u32,
    t: &Thresholds,
    push: &mut impl FnMut(&str, Rule, String),
) {
    // Catalogue JSON is external input: an absurd footprint is an issue, not an overflow.
    let size = a
        .footprint
        .w
        .checked_mul(ppsq)
        .zip(a.footprint.h.checked_mul(ppsq));
    if size.is_none_or(|(w, h)| (img.width, img.height) != (w, h) || w == 0 || h == 0) {
        let need = size.map_or_else(
            || "more pixels than fit in u32".to_string(),
            |(w, h)| format!("{w}x{h}"),
        );
        let msg = format!(
            "image is {}x{}, footprint needs {need}",
            img.width, img.height
        );
        push(&a.id, Rule::ImageSize, msg);
        return;
    }
    let alphas = || img.data.as_chunks::<4>().0.iter().map(|p| p[3]);
    if a.class.is_texture() {
        if alphas().any(|v| v < 255) {
            push(
                &a.id,
                Rule::TextureAlpha,
                "textures must be fully opaque".into(),
            );
        }
        if a.tileable {
            let ratio = seam_ratio(img);
            if ratio > t.max_seam_ratio {
                let msg = format!(
                    "seam/interior difference ratio {ratio:.2} exceeds {:.2}",
                    t.max_seam_ratio
                );
                push(&a.id, Rule::TileSeam, msg);
            }
        }
        return;
    }
    if alphas().all(|v| v == 255) {
        push(
            &a.id,
            Rule::AlphaOpaque,
            "cut-out assets need a transparent background".into(),
        );
        return;
    }
    let stray = stray_fraction(img, t);
    if stray > t.max_stray_fraction {
        let msg = format!(
            "{:.2}% of pixels are semi-transparent more than {} px from the silhouette (max {:.2}%)",
            stray * 100.0,
            t.fringe_px,
            t.max_stray_fraction * 100.0
        );
        push(&a.id, Rule::AlphaFringe, msg);
    }
}

fn rgb_diff(a: [u8; 4], b: [u8; 4]) -> u32 {
    (0..3).map(|c| u32::from(a[c].abs_diff(b[c]))).sum()
}

/// Mean RGB difference across the wrap seams divided by the mean difference
/// between interior neighbours. A seamless texture scores about 1.
#[must_use]
pub fn seam_ratio(img: &Rgba) -> f32 {
    let (w, h) = (img.width, img.height);
    if w < 2 || h < 2 {
        return 0.0;
    }
    let (mut seam, mut seam_n, mut inner, mut inner_n) = (0u64, 0u64, 0u64, 0u64);
    for y in 0..h {
        for x in 0..w {
            let p = img.get(x, y);
            let right = u64::from(rgb_diff(p, img.get((x + 1) % w, y)));
            let down = u64::from(rgb_diff(p, img.get(x, (y + 1) % h)));
            if x + 1 == w {
                (seam, seam_n) = (seam + right, seam_n + 1);
            } else {
                (inner, inner_n) = (inner + right, inner_n + 1);
            }
            if y + 1 == h {
                (seam, seam_n) = (seam + down, seam_n + 1);
            } else {
                (inner, inner_n) = (inner + down, inner_n + 1);
            }
        }
    }
    let seam_mean = seam as f32 / seam_n as f32;
    let inner_mean = (inner as f32 / inner_n as f32).max(2.0);
    seam_mean / inner_mean
}

/// Fraction of all pixels that are partly transparent yet further than
/// `fringe_px` (Chebyshev) from any opaque pixel: haze, halos, glow.
fn stray_fraction(img: &Rgba, t: &Thresholds) -> f32 {
    let (w, h) = (img.width as usize, img.height as usize);
    let opaque: Vec<bool> = img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| p[3] >= t.opaque_alpha)
        .collect();
    let r = t.fringe_px as usize;
    // Separable max filter: rows, then columns.
    let mut rows = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            let (x0, x1) = (x.saturating_sub(r), (x + r).min(w - 1));
            rows[y * w + x] = (x0..=x1).any(|xx| opaque[y * w + xx]);
        }
    }
    let mut stray = 0usize;
    for y in 0..h {
        for x in 0..w {
            let a = img.data[(y * w + x) * 4 + 3];
            if a == 0 || a >= t.opaque_alpha {
                continue;
            }
            let (y0, y1) = (y.saturating_sub(r), (y + r).min(h - 1));
            if !(y0..=y1).any(|yy| rows[yy * w + x]) {
                stray += 1;
            }
        }
    }
    stray as f32 / (w * h) as f32
}
