//! PNG and JSON export. Reads stored worlds only — never depends on `arda-gen`.

// `code-prefs.md` §Q1 bans unwrap/expect *outside* `#[cfg(test)]`.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod carto;
pub mod json;
pub mod symbolic;

pub use carto::render_area_png;
pub use json::{area_json, block_json, SCHEMA_VERSION};
pub use symbolic::{render_block_png, SQUARE_PX};

use thiserror::Error;

/// An export failure (`logic/04` refusals).
#[derive(Debug, Error)]
pub enum RenderError {
    /// The PNG encoder failed.
    #[error("png encoding failed")]
    Png,
    /// A square holds a tile id this build has no entry for.
    #[error("tile id {id} has no entry in the vocabulary")]
    UnmappedTile {
        /// The unmapped id.
        id: u16,
    },
    /// The world is missing its manifest.
    #[error("refusing to export a partial world")]
    PartialWorld,
}

/// Encodes an RGB buffer as a PNG.
///
/// Compression and filter are pinned so repeated exports are byte-identical
/// (`logic/04`).
pub(crate) fn encode_png(width: u32, height: u32, rgb: &[u8]) -> Result<Vec<u8>, RenderError> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Default);
        encoder.set_filter(png::FilterType::NoFilter);
        let mut writer = encoder.write_header().map_err(|_| RenderError::Png)?;
        writer.write_image_data(rgb).map_err(|_| RenderError::Png)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::{
        AreaCells, Block, Cell, GenerateConfig, HeightMm, Manifest, SquareCoord, TerrainKind,
        TileId, ValidationStats, FORMAT_VERSION,
    };

    fn manifest() -> Manifest {
        Manifest {
            format_version: FORMAT_VERSION,
            arda_version: "0.1.0".to_owned(),
            seed: 42,
            config: GenerateConfig::MICRO,
            areas_wide: 2,
            areas_high: 4,
            stats: ValidationStats {
                land_fraction_permille: 600,
                area_count: 8,
                settlement_count: 0,
                named_river_count: 0,
            },
        }
    }

    fn block() -> Block {
        let mut b = Block::filled(TileId::new(9));
        b.set(SquareCoord::new(0, 0).unwrap(), TileId::new(1));
        b.set(SquareCoord::new(63, 63).unwrap(), TileId::new(21));
        b
    }

    fn cells() -> AreaCells {
        AreaCells::flat(Cell {
            height: HeightMm::new(180_000),
            terrain: TerrainKind::Land,
            ..Cell::default()
        })
    }

    #[test]
    fn block_png_has_a_png_signature() {
        let bytes = render_block_png(&block()).unwrap();
        assert_eq!(
            &bytes[..8],
            &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
        );
    }

    #[test]
    fn block_png_is_byte_identical_on_repeat() {
        // logic/04: re-export must produce the same bytes.
        assert_eq!(
            render_block_png(&block()).unwrap(),
            render_block_png(&block()).unwrap()
        );
    }

    #[test]
    fn area_png_is_byte_identical_on_repeat() {
        assert_eq!(
            render_area_png(&cells()).unwrap(),
            render_area_png(&cells()).unwrap()
        );
    }

    #[test]
    fn an_unmapped_tile_is_refused_naming_the_id() {
        // logic/04: unmapped-tile is a typed refusal, not a silent default.
        let mut b = Block::filled(TileId::new(9));
        b.set(SquareCoord::new(1, 1).unwrap(), TileId::new(60_000));
        let err = render_block_png(&b).unwrap_err();
        match err {
            RenderError::UnmappedTile { id } => assert_eq!(id, 60_000),
            other => panic!("expected UnmappedTile, got {other:?}"),
        }
    }

    #[test]
    fn area_json_carries_the_schema_version_and_snake_case_keys() {
        let json = area_json(
            &manifest(),
            1,
            1,
            &cells(),
            &arda_core::AreaObjects::empty(),
        );
        assert!(json.contains("\"schema_version\""));
        assert!(json.contains("\"area_x\""));
        assert!(json.contains("\"height_mm\""));
        assert!(
            !json.contains("areaX"),
            "schema must be snake_case (logic/04)"
        );
    }

    #[test]
    fn block_json_carries_the_tile_legend() {
        let json = block_json(&manifest(), &block());
        assert!(json.contains("\"legend\""));
        assert!(json.contains("\"grass\""));
        assert!(json.contains("\"relaxed\""));
    }

    #[test]
    fn json_is_byte_identical_on_repeat() {
        let a = area_json(
            &manifest(),
            1,
            1,
            &cells(),
            &arda_core::AreaObjects::empty(),
        );
        let b = area_json(
            &manifest(),
            1,
            1,
            &cells(),
            &arda_core::AreaObjects::empty(),
        );
        assert_eq!(a, b);
    }
}
