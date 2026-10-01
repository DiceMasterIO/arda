//! Cached overview renders and the slippy tile pyramid cut from one of them.
//!
//! Renders reuse `arda::export_overview_with_quality_and_style` byte for byte;
//! one render runs at a time so peak memory stays that of a single export.

use crate::cache::ByteLru;
use crate::dto::TilePyramidDto;
use crate::error::{lock, ServerError, ServerResult};
use crate::query::WorldQuery;
use crate::tiles::{self, Rgb, TILE_PX};
use arda::{ImageQuality, MapStyle, OverviewLook};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Overview limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverviewLimits {
    /// Largest `/overview.png` long edge a client may request, pixels.
    pub max_quality_px: u32,
    /// Square pyramid base, pixels (`256 · 2^max_zoom`).
    pub tile_base_px: u32,
    /// Byte budget for cached overview PNGs and tiles, each.
    pub cache_bytes: usize,
}

impl Default for OverviewLimits {
    fn default() -> Self {
        Self {
            max_quality_px: 8192,
            tile_base_px: 4096,
            cache_bytes: 256 << 20,
        }
    }
}

/// Default `/overview.png` long edge.
pub const DEFAULT_QUALITY_PX: u32 = 2048;

/// Presentation of an overview render.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Style {
    /// Natural atlas relief (default).
    Atlas,
    /// Categorical cartography.
    Classic,
    /// Atlas relief seen slightly obliquely (goal 24, opt-in).
    AtlasOblique,
}

impl Style {
    /// Parses `atlas`, `classic` or `atlas-oblique`.
    ///
    /// # Errors
    /// [`ServerError::BadRequest`] for other names.
    pub fn parse(name: &str) -> ServerResult<Self> {
        match name {
            "atlas" => Ok(Self::Atlas),
            "classic" => Ok(Self::Classic),
            "atlas-oblique" => Ok(Self::AtlasOblique),
            other => Err(ServerError::BadRequest(format!(
                "style must be atlas, classic or atlas-oblique, not {other:?}"
            ))),
        }
    }

    const fn map_style(self) -> MapStyle {
        match self {
            Self::Atlas | Self::AtlasOblique => MapStyle::Atlas,
            Self::Classic => MapStyle::Classic,
        }
    }

    const fn look(self) -> OverviewLook {
        OverviewLook {
            oblique: matches!(self, Self::AtlasOblique),
        }
    }
}

/// Encoding of an overview tile (goal 68: WebP; PNG kept for compatibility).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TileFormat {
    /// RGBA PNG.
    Png,
    /// Lossless RGBA WebP.
    Webp,
}

impl TileFormat {
    /// Splits `{y}.png` or `{y}.webp` into the row text and the format.
    #[must_use]
    pub fn split(file: &str) -> Option<(&str, Self)> {
        file.strip_suffix(".png")
            .map(|y| (y, Self::Png))
            .or_else(|| file.strip_suffix(".webp").map(|y| (y, Self::Webp)))
    }

    /// The response `Content-Type`.
    #[must_use]
    pub const fn content_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Webp => "image/webp",
        }
    }
}

/// Cache identity of an encoded overview tile: `(z, x, y, format, oblique)`.
type TileKey = (u32, u32, u32, TileFormat, bool);

/// Overview renders, the decoded pyramid base and cut tiles, all bounded.
pub struct Overview {
    limits: OverviewLimits,
    pyramid: TilePyramidDto,
    pngs: Mutex<ByteLru<(u32, Style), Vec<u8>>>,
    tiles: Mutex<ByteLru<TileKey, Vec<u8>>>,
    /// Decoded pyramid bases: default, then oblique (goal 24).
    base: Mutex<[Option<Arc<Rgb>>; 2]>,
    render: Mutex<()>,
}

impl Overview {
    /// Validates limits against the world's area grid.
    ///
    /// # Errors
    /// Invalid pyramid base or quality limits.
    pub fn new(query: &WorldQuery, limits: OverviewLimits) -> ServerResult<Self> {
        let max_zoom = tiles::max_zoom(limits.tile_base_px)?;
        let m = query.world().manifest();
        let (image_width_px, image_height_px) = ImageQuality::new(limits.tile_base_px)
            .and_then(|q| q.overview_dimensions(m.areas_wide, m.areas_high))
            .map_err(|e| ServerError::BadRequest(format!("tile base: {e}")))?;
        ImageQuality::new(limits.max_quality_px)
            .map_err(|e| ServerError::BadRequest(format!("max overview quality: {e}")))?;
        Ok(Self {
            limits,
            pyramid: TilePyramidDto {
                tile_px: TILE_PX,
                max_zoom,
                base_px: limits.tile_base_px,
                image_width_px,
                image_height_px,
                relief_max_zoom: max_zoom,
            },
            pngs: Mutex::new(ByteLru::new(limits.cache_bytes)),
            tiles: Mutex::new(ByteLru::new(limits.cache_bytes)),
            base: Mutex::new([None, None]),
            render: Mutex::new(()),
        })
    }

    /// Pyramid geometry.
    #[must_use]
    pub const fn pyramid(&self) -> TilePyramidDto {
        self.pyramid
    }

    /// Parses and bounds a `quality` query value (pixels or `NK`).
    ///
    /// # Errors
    /// [`ServerError::BadRequest`] outside 512 px..=`max_quality_px`.
    pub fn quality(&self, text: Option<&str>) -> ServerResult<u32> {
        let px = match text {
            None => DEFAULT_QUALITY_PX,
            Some(t) => t
                .parse::<ImageQuality>()
                .map_err(|e| ServerError::BadRequest(format!("quality {t:?}: {e}")))?
                .pixels(),
        };
        if px > self.limits.max_quality_px {
            return Err(ServerError::BadRequest(format!(
                "quality {px} exceeds this server's limit of {}",
                self.limits.max_quality_px
            )));
        }
        Ok(px)
    }

    /// The overview PNG at `quality_px`, rendering on a cache miss.
    ///
    /// # Errors
    /// Render, write or read failures.
    pub fn png(
        &self,
        query: &WorldQuery,
        quality_px: u32,
        style: Style,
    ) -> ServerResult<Arc<Vec<u8>>> {
        let key = (quality_px, style);
        if let Some(hit) = lock(&self.pngs)?.get(&key) {
            return Ok(hit);
        }
        let _one_at_a_time = lock(&self.render)?;
        if let Some(hit) = lock(&self.pngs)?.get(&key) {
            return Ok(hit);
        }
        let bytes = Arc::new(render(query, quality_px, style)?);
        let size = bytes.len();
        Ok(lock(&self.pngs)?.insert(key, bytes, size))
    }

    fn base(&self, query: &WorldQuery, oblique: bool) -> ServerResult<Arc<Rgb>> {
        let i = usize::from(oblique);
        if let Some(base) = lock(&self.base)?[i].as_ref() {
            return Ok(Arc::clone(base));
        }
        let style = if oblique {
            Style::AtlasOblique
        } else {
            Style::Atlas
        };
        let png = self.png(query, self.limits.tile_base_px, style)?;
        let rgb = Arc::new(tiles::decode_rgb(&png)?);
        let mut slots = lock(&self.base)?;
        Ok(Arc::clone(slots[i].get_or_insert(rgb)))
    }

    /// Slippy tile `(z, x, y)` as a 256 px RGBA PNG or lossless WebP. Both
    /// encode the same pixels. `oblique` cuts the tile from the oblique
    /// pyramid (goal 24, opt-in) instead of the default one.
    ///
    /// # Errors
    /// [`ServerError::NotFound`] outside the pyramid; render or encode failures.
    pub fn tile(
        &self,
        query: &WorldQuery,
        (z, x, y): (u32, u32, u32),
        (format, oblique): (TileFormat, bool),
    ) -> ServerResult<Arc<Vec<u8>>> {
        let key = (z, x, y, format, oblique);
        if z > self.pyramid.max_zoom || x >= (1 << z) || y >= (1 << z) {
            return Err(ServerError::NotFound(format!(
                "tile {z}/{x}/{y} is outside the pyramid (zoom 0..={})",
                self.pyramid.max_zoom
            )));
        }
        if let Some(hit) = lock(&self.tiles)?.get(&key) {
            return Ok(hit);
        }
        let base = self.base(query, oblique)?;
        let rgba = tiles::cut(&base, self.pyramid.max_zoom, z, x, y)?;
        let bytes = Arc::new(match format {
            TileFormat::Png => tiles::encode_rgba(TILE_PX, TILE_PX, &rgba)?,
            TileFormat::Webp => crate::tactical::images::encode_webp(&rgba, TILE_PX)?,
        });
        let size = bytes.len();
        Ok(lock(&self.tiles)?.insert(key, bytes, size))
    }
}

/// Removes a scratch directory when dropped.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Creates a fresh, empty scratch directory `<parent>/<stem>-<n>`.
///
/// The name is predictable and the temp directory shared, so the directory
/// must be new: `create_dir` refuses anything already there (another
/// user's directory or a planted symlink), and the next name is tried.
fn scratch_in(parent: &Path, stem: &str, next: &AtomicU64) -> ServerResult<Scratch> {
    for _ in 0..64 {
        let path = parent.join(format!("{stem}-{}", next.fetch_add(1, Ordering::Relaxed)));
        match std::fs::create_dir(&path) {
            Ok(()) => return Ok(Scratch(path)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(ServerError::Internal(format!("scratch dir: {e}"))),
        }
    }
    Err(ServerError::Internal(
        "scratch dir: no free name after 64 tries".into(),
    ))
}

fn render(query: &WorldQuery, quality_px: u32, style: Style) -> ServerResult<Vec<u8>> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let quality = ImageQuality::new(quality_px)
        .map_err(|e| ServerError::BadRequest(format!("quality: {e}")))?;
    let stem = format!("arda-server-{}", std::process::id());
    let dir = scratch_in(&std::env::temp_dir(), &stem, &NEXT)?;
    let path = arda::export_overview_with_look(
        query.world(),
        &dir.0,
        quality,
        style.map_style(),
        style.look(),
    )?;
    std::fs::read(&path).map_err(|e| ServerError::Internal(format!("reading render: {e}")))
}

impl std::fmt::Debug for Overview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Overview")
            .field("limits", &self.limits)
            .field("pyramid", &self.pyramid)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scratch_dirs_never_reuse_a_planted_path() {
        let root = std::env::temp_dir().join(format!("arda-scratch-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let victim = root.join("victim");
        std::fs::create_dir_all(&victim).unwrap();
        std::fs::write(victim.join("keep.txt"), b"precious").unwrap();
        let parent = root.join("tmp");
        std::fs::create_dir_all(&parent).unwrap();
        // Another local user plants the next predictable name.
        #[cfg(unix)]
        std::os::unix::fs::symlink(&victim, parent.join("s-0")).unwrap();
        #[cfg(not(unix))]
        std::fs::create_dir(parent.join("s-0")).unwrap();
        let next = AtomicU64::new(0);
        let dir = scratch_in(&parent, "s", &next).unwrap();
        assert_eq!(dir.0, parent.join("s-1"));
        std::fs::write(dir.0.join("render.png"), b"x").unwrap();
        drop(dir);
        assert!(!parent.join("s-1").exists(), "scratch is removed");
        assert!(victim.join("keep.txt").exists());
        assert!(!victim.join("render.png").exists());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
