//! Canonical modeled rectangle and prepared tile index derived from the request.
#![deny(missing_docs)]

use super::routing::Extent;
use super::types::{MarineBoundary, PreparedExtent};
use arda_core::{AreaCoord, GenerateConfig, AREA_CELLS};

/// One deterministic prepared-tile index entry; no directory scan is involved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// Stable area-grid coordinate.
    pub area: AreaCoord,
    /// Actual modeled cells and outer-domain sides.
    pub valid: PreparedExtent,
    /// Whether this tile belongs to the existing final export rectangle.
    pub exported: bool,
}

/// The requested continent plus the complete existing exported area cells.
/// Requested fringe is modeled even where no final area file is produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedDomain {
    width: u32,
    height: u32,
    exported_wide: u32,
    exported_high: u32,
}

/// Invalid deserialized request or unsupported union geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid prepared domain request")]
pub struct DomainError;

impl PreparedDomain {
    /// Preserves the existing final area count and 512-cell physical extent.
    /// The modeled width is max(requested km *10, exported areas *512), and
    /// likewise for height. Any remaining partial tile is private fringe.
    ///
    /// # Errors
    /// Revalidates the request, including values constructed by deserialization,
    /// and rejects any domain beyond the shared routing bound.
    pub fn for_config(config: GenerateConfig) -> Result<Self, DomainError> {
        let size = config.size_km();
        GenerateConfig::new(size, config.latitude_band(), config.mean_density_per_km2())
            .map_err(|_| DomainError)?;
        let exported_wide = u32::try_from(config.areas_wide()).map_err(|_| DomainError)?;
        let exported_high = u32::try_from(config.areas_high()).map_err(|_| DomainError)?;
        let width = (size.width * 10).max(exported_wide * u32::from(AREA_CELLS));
        let height = (size.height * 10).max(exported_high * u32::from(AREA_CELLS));
        Extent::new(width, height).ok_or(DomainError)?;
        Ok(Self {
            width,
            height,
            exported_wide,
            exported_high,
        })
    }
    /// Actual modeled width in 100-metre cells.
    #[must_use]
    pub fn width(self) -> u32 {
        self.width
    }
    /// Actual modeled height in 100-metre cells.
    #[must_use]
    pub fn height(self) -> u32 {
        self.height
    }
    /// Number of prepared tile columns, including a partial fringe if present.
    #[must_use]
    pub fn columns(self) -> u32 {
        self.width.div_ceil(u32::from(AREA_CELLS))
    }
    /// Number of prepared tile rows, including a partial fringe if present.
    #[must_use]
    pub fn rows(self) -> u32 {
        self.height.div_ceil(u32::from(AREA_CELLS))
    }
    /// Bounded index length; at most79²=6,241 entries for supported requests.
    #[must_use]
    pub fn count(self) -> u32 {
        self.columns() * self.rows()
    }
    /// Exact row-major entry at an admitted ordinal.
    #[must_use]
    pub fn entry(self, ordinal: u32) -> Option<Entry> {
        if ordinal >= self.count() {
            return None;
        }
        let x = ordinal % self.columns();
        let y = ordinal / self.columns();
        let width = (self.width - x * u32::from(AREA_CELLS)).min(u32::from(AREA_CELLS));
        let height = (self.height - y * u32::from(AREA_CELLS)).min(u32::from(AREA_CELLS));
        Some(Entry {
            area: AreaCoord::new(i32::try_from(x).ok()?, i32::try_from(y).ok()?),
            valid: PreparedExtent {
                width: u16::try_from(width).ok()?,
                height: u16::try_from(height).ok()?,
                boundary: MarineBoundary {
                    north: y == 0,
                    east: x + 1 == self.columns(),
                    south: y + 1 == self.rows(),
                    west: x == 0,
                },
            },
            exported: x < self.exported_wide && y < self.exported_high,
        })
    }
    /// Resolves an actual prepared area; rejects negative/outside coordinates.
    #[must_use]
    pub fn ordinal(self, area: AreaCoord) -> Option<u32> {
        let x = u32::try_from(area.x).ok()?;
        let y = u32::try_from(area.y).ok()?;
        (x < self.columns() && y < self.rows()).then(|| y * self.columns() + x)
    }
    /// Immutable row-major iteration without a retained tile/path vector.
    pub fn entries(self) -> impl Iterator<Item = Entry> {
        (0..self.count()).filter_map(move |i| self.entry(i))
    }
    /// Canonical private-index geometry header. Readers compare it to the
    /// request-derived domain; none of its count fields controls allocation alone.
    #[must_use]
    pub fn encode(self) -> [u8; 32] {
        let mut row = [0; 32];
        row[..8].copy_from_slice(b"ARDAPDI1");
        row[8..12].copy_from_slice(&self.width.to_le_bytes());
        row[12..16].copy_from_slice(&self.height.to_le_bytes());
        row[16..20].copy_from_slice(&self.exported_wide.to_le_bytes());
        row[20..24].copy_from_slice(&self.exported_high.to_le_bytes());
        row[24..28].copy_from_slice(&self.count().to_le_bytes());
        row
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::{LatitudeBand, SizeKm};
    #[test]
    fn micro_contains_full_exported_cells_without_changing_final_area_count() {
        let d = PreparedDomain::for_config(GenerateConfig::MICRO).unwrap();
        assert_eq!((d.width(), d.height(), d.count()), (1024, 2048, 8));
        let entries: Vec<_> = d.entries().collect();
        assert_eq!(entries.len(), 8);
        assert!(entries
            .iter()
            .all(|e| e.exported && e.valid.width == 512 && e.valid.height == 512));
        assert!(entries[7].valid.boundary.east && entries[7].valid.boundary.south);
    }
    #[test]
    fn requested_fringe_is_real_terrain_and_exported_edges_are_not_domain_exits() {
        let d = PreparedDomain::for_config(GenerateConfig::default()).unwrap();
        assert_eq!((d.width(), d.height(), d.count()), (5000, 10000, 200));
        assert_eq!(d.entries().filter(|e| e.exported).count(), 171);
        let last_export = d.entry(d.ordinal(AreaCoord::new(8, 18)).unwrap()).unwrap();
        assert!(last_export.exported);
        assert!(!last_export.valid.boundary.east && !last_export.valid.boundary.south);
        let fringe = d.entry(d.count() - 1).unwrap();
        assert!(!fringe.exported);
        assert_eq!((fringe.valid.width, fringe.valid.height), (392, 272));
        assert!(fringe.valid.boundary.east && fringe.valid.boundary.south);
    }
    #[test]
    fn every_supported_axis_has_exact_nonoverlapping_coverage_and_bounded_index() {
        for km in 64..=4000 {
            let c =
                GenerateConfig::new(SizeKm::new(km, 64), LatitudeBand::new(35, 55), 15).unwrap();
            let d = PreparedDomain::for_config(c).unwrap();
            assert!(d.width() <= 40000 && d.count() <= 6241);
            let mut cells = 0u64;
            for (i, e) in d.entries().enumerate() {
                assert_eq!(d.ordinal(e.area), Some(u32::try_from(i).unwrap()));
                assert_eq!(
                    e.valid.boundary.east,
                    e.area.x + 1 == i32::try_from(d.columns()).unwrap()
                );
                if e.exported {
                    assert_eq!((e.valid.width, e.valid.height), (512, 512));
                }
                cells += u64::from(e.valid.width) * u64::from(e.valid.height);
            }
            assert_eq!(cells, u64::from(d.width()) * u64::from(d.height()));
        }
        let max = PreparedDomain::for_config(
            GenerateConfig::new(SizeKm::new(4000, 4000), LatitudeBand::new(-80, 80), 200).unwrap(),
        )
        .unwrap();
        assert_eq!(
            (
                max.count(),
                max.entry(6240).unwrap().valid.width,
                max.entry(6240).unwrap().valid.height
            ),
            (6241, 64, 64)
        );
        assert_eq!(max.ordinal(AreaCoord::new(-1, 0)), None);
        assert_eq!(max.ordinal(AreaCoord::new(i32::MAX, i32::MAX)), None);
        assert_eq!(max.entry(6241), None);
    }
}
