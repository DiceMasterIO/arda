//! High-resolution PNG exports with bounded raster memory and final publication.

use crate::{
    atlas::{atlas_terrain, FINE_READER_BYTES},
    AreaImageScale, ExportError, ImageQuality, MapStyle, World,
};
use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

/// Exports an area PNG at a validated square resolution, from 512 through 32K.
///
/// Streams rows to disk; the stored 100 m terrain is unchanged. The final name
/// remains `area_XX_YY.png`, independent of the selected image resolution.
/// The output directory must already exist.
///
/// # Errors
/// Propagates invalid area, saved geometry, rendering-work, and write errors.
/// An unsuccessful export does not replace a previous completed PNG.
pub fn export_area_with_quality(
    world: &World,
    ax: i32,
    ay: i32,
    out: &Path,
    quality: ImageQuality,
) -> Result<PathBuf, ExportError> {
    export_area_with_quality_and_style(world, ax, ay, out, quality, MapStyle::Classic)
}

/// Exports an area PNG at a validated resolution and presentation.
///
/// # Errors
/// Propagates target/neighbor loading, Atlas halo, saved geometry, PNG and
/// write failures. A failed stream preserves an existing completed destination.
pub fn export_area_with_quality_and_style(
    world: &World,
    ax: i32,
    ay: i32,
    out: &Path,
    quality: ImageQuality,
    style: MapStyle,
) -> Result<PathBuf, ExportError> {
    let area = world.read_area(ax, ay)?;
    let coordinate = |value| {
        u32::try_from(value).map_err(|_| arda_render::RenderError::ChannelGeometry {
            reason: "area origin has a negative global coordinate",
        })
    };
    let origin = arda_core::GlobalCell {
        x: coordinate(ax)? * 512,
        y: coordinate(ay)? * 512,
    };
    let scale = if quality.pixels() == 512 {
        AreaImageScale::Preview
    } else {
        AreaImageScale::Custom(quality)
    };
    let mut fine = match style {
        MapStyle::Classic => None,
        MapStyle::Atlas => world.fine_terrain(FINE_READER_BYTES)?,
    };
    let shore = match style {
        MapStyle::Classic => None,
        MapStyle::Atlas => world.shore()?,
    };
    let terrain = match style {
        MapStyle::Classic => None,
        MapStyle::Atlas => Some(atlas_terrain(
            world,
            ax,
            ay,
            area.cells(),
            area.lakes(),
            fine.as_mut(),
            shore.as_ref(),
        )?),
    };
    let path = out.join(format!("area_{ax:02}_{ay:02}.png"));
    publish_png(&path, |writer| {
        match terrain.as_ref() {
            None => arda_render::render_area_png_to(
                area.cells(),
                area.objects(),
                origin,
                scale,
                writer,
            )?,
            Some(terrain) => arda_render::render_area_png_to_atlas(
                area.cells(),
                area.objects(),
                origin,
                scale,
                terrain,
                writer,
            )?,
        }
        Ok(())
    })?;
    Ok(path)
}

/// Exports an overview PNG with the selected long edge and preserved aspect ratio.
///
/// Streams raster bands and reads uncached areas as needed. The final name is
/// `overview.png`; the output directory must already exist.
///
/// # Errors
/// Propagates invalid dimensions, missing/corrupt saved areas, and PNG/write errors.
/// An unsuccessful export does not replace a previous completed PNG.
pub fn export_overview_with_quality(
    world: &World,
    out: &Path,
    quality: ImageQuality,
) -> Result<PathBuf, ExportError> {
    export_overview_with_quality_and_style(world, out, quality, MapStyle::Classic)
}

/// Exports an overview PNG with the selected long edge and presentation.
///
/// # Errors
/// Propagates dimension, target/neighbor loading, Atlas halo, render, PNG and
/// write failures. A failed stream preserves an existing completed destination.
pub fn export_overview_with_quality_and_style(
    world: &World,
    out: &Path,
    quality: ImageQuality,
    style: MapStyle,
) -> Result<PathBuf, ExportError> {
    let manifest = world.manifest();
    let (width, height) = quality.overview_dimensions(manifest.areas_wide, manifest.areas_high)?;
    let mut fine = match style {
        MapStyle::Classic => None,
        MapStyle::Atlas => world.fine_terrain(FINE_READER_BYTES)?,
    };
    let shore = match style {
        MapStyle::Classic => None,
        MapStyle::Atlas => world.shore()?,
    };
    let path = out.join("overview.png");
    publish_png(&path, |writer| match style {
        MapStyle::Classic => arda_render::write_overview_png(
            manifest.areas_wide,
            manifest.areas_high,
            width,
            height,
            writer,
            |at| Ok(world.read_area(at.x, at.y)?.into_cells()),
        ),
        MapStyle::Atlas => {
            let load_area = |at: arda_core::AreaCoord| {
                let area = world.read_area(at.x, at.y)?;
                let terrain = atlas_terrain(
                    world,
                    at.x,
                    at.y,
                    area.cells(),
                    area.lakes(),
                    fine.as_mut(),
                    shore.as_ref(),
                )?;
                let mut channels = arda_render::OverviewChannelContext::new(
                    at,
                    manifest.areas_wide,
                    manifest.areas_high,
                )?;
                for y in (at.y - 1).max(0)..=(at.y + 1).min(manifest.areas_high - 1) {
                    for x in (at.x - 1).max(0)..=(at.x + 1).min(manifest.areas_wide - 1) {
                        if x == at.x && y == at.y {
                            channels.add_area(at, area.objects())?;
                            channels.add_braided(braided_cells(
                                at,
                                area.objects(),
                                area.water(),
                            ))?;
                        } else {
                            let here = arda_core::AreaCoord::new(x, y);
                            let neighbor = world.read_area_objects(x, y)?;
                            channels.add_area(here, &neighbor)?;
                            let water = world.read_area_water(x, y)?;
                            channels.add_braided(braided_cells(here, &neighbor, water.as_ref()))?;
                        }
                    }
                }
                let channels = channels.finish()?;
                let (cells, _objects) = area.into_render_parts();
                Ok((cells, terrain, channels))
            };
            let recipe4 = manifest
                .fine_terrain
                .is_some_and(|fine| fine.recipe_version >= 4);
            let formed = manifest
                .fine_terrain
                .is_some_and(|fine| fine.recipe_version >= 5);
            if formed {
                arda_render::write_atlas_overview_png_with_channels_formed(
                    manifest.areas_wide,
                    manifest.areas_high,
                    width,
                    height,
                    writer,
                    load_area,
                )
            } else if recipe4 {
                arda_render::write_atlas_overview_png_with_channels_recipe4(
                    manifest.areas_wide,
                    manifest.areas_high,
                    width,
                    height,
                    writer,
                    load_area,
                )
            } else {
                arda_render::write_atlas_overview_png_with_channels(
                    manifest.areas_wide,
                    manifest.areas_high,
                    width,
                    height,
                    writer,
                    load_area,
                )
            }
        }
    })?;
    Ok(path)
}

// A failed stream must not truncate the last usable export. This is export I/O,
// separate from deterministic simulation state and final PNG bytes.
fn publish_png(
    path: &Path,
    render: impl FnOnce(&mut BufWriter<File>) -> Result<(), ExportError>,
) -> Result<(), ExportError> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let temporary = path.with_file_name(format!(
        ".{}.{}-{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let write_error = |source| ExportError::Write {
        path: path.display().to_string(),
        source,
    };
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(write_error)?;
    let cleanup = PartialExport(temporary);
    let mut writer = BufWriter::new(file);
    render(&mut writer)?;
    writer.flush().map_err(write_error)?;
    drop(writer);
    std::fs::rename(&cleanup.0, path).map_err(write_error)?;
    Ok(())
}

struct PartialExport(PathBuf);

impl Drop for PartialExport {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Global course cells of an area's braided segments with their belt
/// width, from the stored water forms (logic/02 §world-water).
fn braided_cells(
    at: arda_core::AreaCoord,
    objects: &arda_core::AreaObjects,
    water: Option<&arda_core::water::AreaWater>,
) -> Vec<(arda_core::GlobalCell, u32)> {
    let Some(water) = water else {
        return Vec::new();
    };
    let (ox, oy) = (
        u32::try_from(at.x).unwrap_or(0) * 512,
        u32::try_from(at.y).unwrap_or(0) * 512,
    );
    objects
        .rivers
        .iter()
        .zip(&water.segments)
        .filter(|(_, f)| f.pattern == arda_core::water::ChannelPattern::Braided)
        .flat_map(|(r, f)| {
            r.course.iter().map(move |c| {
                (
                    arda_core::GlobalCell {
                        x: ox + u32::from(c.x()),
                        y: oy + u32::from(c.y()),
                    },
                    f.belt_width_dm,
                )
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_stream_preserves_previous_export_and_removes_partial_file() {
        let dir = std::env::temp_dir().join(format!("arda-export-failed-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("overview.png");
        std::fs::write(&path, b"previous complete PNG").unwrap();
        let result = publish_png(&path, |writer| {
            writer.write_all(b"partial").unwrap();
            Err(arda_render::RenderError::Png.into())
        });
        assert!(result.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"previous complete PNG");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn completed_stream_replaces_previous_export_without_leaving_temporary_files() {
        let dir = std::env::temp_dir().join(format!("arda-export-complete-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("overview.png");
        std::fs::write(&path, b"previous").unwrap();
        publish_png(&path, |writer| {
            writer.write_all(b"complete").unwrap();
            Ok(())
        })
        .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"complete");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
