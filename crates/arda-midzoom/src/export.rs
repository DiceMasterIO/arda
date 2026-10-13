//! PNG exports with the society layer (task 15): the `arda` exporters with
//! land use, roads and settlements painted on, chosen by [`SocietyMode`].
//!
//! Overviews take the band overlay of `arda-settle` (land tone, roads by
//! class, tier symbols); areas take the worked land of relief tiles
//! ([`crate::land::export`]). Both stream; neither holds a page.

use crate::land::export::AreaLandOverlay;
use arda::{ExportError, ImageQuality, MapStyle, OverviewLook, World};
use arda_render::{BandOverlay, RenderError};
use std::path::{Path, PathBuf};

/// Whether an export carries the society layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SocietyMode {
    /// No society layer: the plain terrain export.
    Off,
    /// The society layer when the world has `society/` (and the look can
    /// carry it); otherwise the plain export.
    #[default]
    Auto,
    /// The society layer, or an error when the world has none.
    On,
}

/// Whether `world` has settled society files (`arda settle` ran).
#[must_use]
pub fn has_society(world: &World) -> bool {
    world
        .dir()
        .join(arda_settle::SOCIETY_DIR)
        .join("settlements.json")
        .is_file()
}

fn overlay_error(reason: impl ToString) -> ExportError {
    ExportError::Render(RenderError::Overlay {
        reason: reason.to_string(),
    })
}

/// Whether the export carries the layer, or why it must not.
fn wanted(world: &World, mode: SocietyMode, can_carry: bool) -> Result<bool, ExportError> {
    match mode {
        SocietyMode::Off => Ok(false),
        SocietyMode::Auto => Ok(can_carry && has_society(world)),
        SocietyMode::On if !has_society(world) => Err(overlay_error(
            "the world has no society/ directory; run `arda settle` first",
        )),
        SocietyMode::On if !can_carry => Err(overlay_error(
            "the oblique look cannot carry the society layer",
        )),
        SocietyMode::On => Ok(true),
    }
}

/// [`arda::export_overview_with_look`] with the society layer per `mode`.
///
/// # Errors
/// As [`arda::export_overview_with_overlay`]; `On` also refuses a world
/// without society and the oblique look.
pub fn export_overview(
    world: &World,
    out: &Path,
    quality: ImageQuality,
    (style, look): (MapStyle, OverviewLook),
    mode: SocietyMode,
) -> Result<PathBuf, ExportError> {
    let overlay = if wanted(world, mode, !look.oblique)? {
        let m = world.manifest();
        let (w, h) = quality.overview_dimensions(m.areas_wide, m.areas_high)?;
        Some(
            arda_settle::render::band::SocietyBandOverlay::open(world.dir(), w, h)
                .map_err(overlay_error)?,
        )
    } else {
        None
    };
    arda::export_overview_with_overlay(
        world,
        out,
        quality,
        (style, look),
        overlay.as_ref().map(|o| o as &dyn BandOverlay),
    )
}

/// [`arda::export_area_with_quality_and_style`] with the society layer per
/// `mode`.
///
/// # Errors
/// As [`arda::export_area_with_overlay`]; `On` also refuses a world without
/// society.
pub fn export_area(
    world: &World,
    (ax, ay): (i32, i32),
    out: &Path,
    quality: ImageQuality,
    style: MapStyle,
    mode: SocietyMode,
) -> Result<PathBuf, ExportError> {
    let overlay = if wanted(world, mode, true)? {
        AreaLandOverlay::open(world.dir(), (ax, ay), quality.pixels()).map_err(overlay_error)?
    } else {
        None
    };
    arda::export_area_with_overlay(
        world,
        ax,
        ay,
        out,
        quality,
        (style, overlay.as_ref().map(|o| o as &dyn BandOverlay)),
    )
}
