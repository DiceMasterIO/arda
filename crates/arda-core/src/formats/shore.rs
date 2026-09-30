//! `terrain/shore.bin` — shore classes and island census of a recipe-5
//! world at the 100 m cell grid (logic/02 §fine-formation shore classes).
//!
//! Layout, little-endian: magic `ARDASHOR`, major/minor `u16`, spacing µm
//! `u32`, width `u32`, height `u32`, run, island and landform counts
//! `u32`, then runs (`y u32, x u32, len u16, class u8`, row-major, no
//! overlap), then islands (`x_um i64, y_um i64, cells u32, top_m i16,
//! cause u8`), then landforms (`x_um i64, y_um i64, area_km2 u32,
//! relief_m i16, kind u8, cause u8`), then a BLAKE3 digest of everything
//! before it. Absent cells are
//! [`ShoreClass::None`]. The layer is optional: legacy and pre-shore
//! worlds simply have no file.

use crate::error::FormatError;

/// Fixed relative path inside a world.
pub const SHORE_PATH: &str = "terrain/shore.bin";
const MAGIC: [u8; 8] = *b"ARDASHOR";
const MAJOR: u16 = 1;
const MINOR: u16 = 0;
const HEADER: usize = 8 + 2 + 2 + 4 * 6;
const RUN_BYTES: usize = 11;
const ISLAND_BYTES: usize = 23;
const LANDFORM_BYTES: usize = 24;

/// The kind of shore a 100 m cell belongs to (goals 15 and 16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord)]
#[repr(u8)]
pub enum ShoreClass {
    /// Not a shore cell.
    #[default]
    None = 0,
    /// Sandy beach: sediment-fed, low, usually in bays or at river mouths.
    SandBeach = 1,
    /// Shingle (gravel) beach: high-energy coast below eroding hard rock.
    ShingleBeach = 2,
    /// Cliff: high ground meeting the sea.
    Cliff = 3,
    /// Rocky shore: moderate relief without a beach.
    RockyShore = 4,
    /// Salt marsh: low, sheltered, muddy land margin.
    Marsh = 5,
    /// Tidal flat: very shallow, sheltered water margin.
    TidalFlat = 6,
    /// Estuary: enclosed water at a river mouth.
    Estuary = 7,
}

impl ShoreClass {
    /// Decodes a stored discriminant.
    #[must_use]
    pub const fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::None,
            1 => Self::SandBeach,
            2 => Self::ShingleBeach,
            3 => Self::Cliff,
            4 => Self::RockyShore,
            5 => Self::Marsh,
            6 => Self::TidalFlat,
            7 => Self::Estuary,
            _ => return None,
        })
    }

    /// Whether this is a beach of any kind.
    #[must_use]
    pub const fn is_beach(self) -> bool {
        matches!(self, Self::SandBeach | Self::ShingleBeach)
    }
}

/// Why an island exists (goal 17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum IslandCause {
    /// A drowned ridge or detached continental block.
    Continental = 0,
    /// A sand barrier island built by waves on a low coast.
    Barrier = 1,
    /// A volcanic arc edifice above a subduction margin.
    Volcanic = 2,
    /// Land between distributaries of a delta.
    Delta = 3,
}

impl IslandCause {
    /// Decodes a stored discriminant.
    #[must_use]
    pub const fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Continental,
            1 => Self::Barrier,
            2 => Self::Volcanic,
            3 => Self::Delta,
            _ => return None,
        })
    }
}

/// A closed basin, a plateau or a glacial trough of the landform audit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LandformKind {
    /// A large closed depression (endorheic sink, lake or inland sea).
    Basin = 0,
    /// A high surface of low relief.
    Plateau = 1,
    /// A glacially overdeepened trough (lake or fjord basin).
    GlacialTrough = 2,
}

impl LandformKind {
    /// Decodes a stored discriminant.
    #[must_use]
    pub const fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Basin,
            1 => Self::Plateau,
            2 => Self::GlacialTrough,
            _ => return None,
        })
    }
}

/// The geological reason for a landform (goal 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LandformCause {
    /// Crustal extension: a rift or graben.
    Rift = 0,
    /// Flexure beside a collision orogen: foreland or intermontane basin.
    Foreland = 1,
    /// A subduction margin: fore-arc, intra-arc or back-arc.
    Arc = 2,
    /// Crustal thickening in a collision orogen.
    Orogenic = 3,
    /// The uplifted flank of a rift.
    RiftShoulder = 4,
    /// Glacial overdeepening below an ice-age glacier.
    Glacial = 5,
    /// No cause found (reported, never silently kept for basins).
    Unexplained = 6,
}

impl LandformCause {
    /// Decodes a stored discriminant.
    #[must_use]
    pub const fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Rift,
            1 => Self::Foreland,
            2 => Self::Arc,
            3 => Self::Orogenic,
            4 => Self::RiftShoulder,
            5 => Self::Glacial,
            6 => Self::Unexplained,
            _ => return None,
        })
    }
}

/// One audited landform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Landform {
    /// Centroid, micrometres from the terrain origin.
    pub x_um: i64,
    /// Centroid, micrometres from the terrain origin.
    pub y_um: i64,
    /// Area, km².
    pub area_km2: u32,
    /// Basin depth below spill or plateau mean height, metres.
    pub relief_m: i16,
    /// What it is.
    pub kind: LandformKind,
    /// Why it exists.
    pub cause: LandformCause,
}

/// One island of the census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Island {
    /// Centroid, micrometres from the terrain origin.
    pub x_um: i64,
    /// Centroid, micrometres from the terrain origin.
    pub y_um: i64,
    /// Land cells (100 m).
    pub cells: u32,
    /// Highest point, metres.
    pub top_m: i16,
    /// Cause.
    pub cause: IslandCause,
}

/// Shore classes on a regular grid plus the island census.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShoreLayer {
    /// Grid spacing, micrometres (100 m for the area cell grid).
    pub spacing_um: u32,
    /// Cells per row.
    pub width: u32,
    /// Rows.
    pub height: u32,
    /// Row-major classes, `width * height` long.
    pub classes: Vec<ShoreClass>,
    /// Islands, in centroid row-major order.
    pub islands: Vec<Island>,
    /// Audited basins, plateaus and glacial troughs.
    pub landforms: Vec<Landform>,
}

impl ShoreLayer {
    /// Class at grid cell `(x, y)`; `None` outside the grid.
    #[must_use]
    pub fn class_at(&self, x: u32, y: u32) -> ShoreClass {
        if x >= self.width || y >= self.height {
            return ShoreClass::None;
        }
        let i = y as usize * self.width as usize + x as usize;
        self.classes.get(i).copied().unwrap_or_default()
    }

    /// Class of the cell nearest to a position in micrometres.
    #[must_use]
    pub fn class_near_um(&self, x_um: i64, y_um: i64) -> ShoreClass {
        let s = i64::from(self.spacing_um.max(1));
        let (x, y) = ((x_um + s / 2).div_euclid(s), (y_um + s / 2).div_euclid(s));
        match (u32::try_from(x), u32::try_from(y)) {
            (Ok(x), Ok(y)) => self.class_at(x, y),
            _ => ShoreClass::None,
        }
    }

    /// Encodes the layer.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut runs: Vec<(u32, u32, u16, u8)> = Vec::new();
        let w = self.width as usize;
        for (y, row) in self.classes.chunks(w.max(1)).enumerate() {
            let mut x = 0;
            while x < row.len() {
                let c = row[x];
                let start = x;
                while x < row.len() && row[x] == c && x - start < usize::from(u16::MAX) {
                    x += 1;
                }
                if c != ShoreClass::None {
                    runs.push((
                        u32::try_from(y).unwrap_or(u32::MAX),
                        u32::try_from(start).unwrap_or(u32::MAX),
                        u16::try_from(x - start).unwrap_or(u16::MAX),
                        c as u8,
                    ));
                }
            }
        }
        let mut out = Vec::with_capacity(
            HEADER
                + runs.len() * RUN_BYTES
                + self.islands.len() * ISLAND_BYTES
                + self.landforms.len() * LANDFORM_BYTES
                + 32,
        );
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&MAJOR.to_le_bytes());
        out.extend_from_slice(&MINOR.to_le_bytes());
        for v in [
            self.spacing_um,
            self.width,
            self.height,
            u32::try_from(runs.len()).unwrap_or(u32::MAX),
            u32::try_from(self.islands.len()).unwrap_or(u32::MAX),
            u32::try_from(self.landforms.len()).unwrap_or(u32::MAX),
        ] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for (y, x, len, c) in runs {
            out.extend_from_slice(&y.to_le_bytes());
            out.extend_from_slice(&x.to_le_bytes());
            out.extend_from_slice(&len.to_le_bytes());
            out.push(c);
        }
        for island in &self.islands {
            out.extend_from_slice(&island.x_um.to_le_bytes());
            out.extend_from_slice(&island.y_um.to_le_bytes());
            out.extend_from_slice(&island.cells.to_le_bytes());
            out.extend_from_slice(&island.top_m.to_le_bytes());
            out.push(island.cause as u8);
        }
        for f in &self.landforms {
            out.extend_from_slice(&f.x_um.to_le_bytes());
            out.extend_from_slice(&f.y_um.to_le_bytes());
            out.extend_from_slice(&f.area_km2.to_le_bytes());
            out.extend_from_slice(&f.relief_m.to_le_bytes());
            out.push(f.kind as u8);
            out.push(f.cause as u8);
        }
        let digest = blake3::hash(&out);
        out.extend_from_slice(digest.as_bytes());
        out
    }

    /// Decodes and verifies a layer read from `path`.
    ///
    /// # Errors
    /// Bad magic or version, truncation, checksum mismatch, unknown
    /// discriminants, overlapping or out-of-grid runs.
    pub fn decode(bytes: &[u8], path: &str) -> Result<Self, FormatError> {
        let eof = |expected: usize| FormatError::UnexpectedEof {
            path: path.to_owned(),
            read: bytes.len(),
            expected,
        };
        if bytes.len() < HEADER + 32 {
            return Err(eof(HEADER + 32));
        }
        if bytes[..8] != MAGIC {
            return Err(FormatError::BadMagic {
                path: path.to_owned(),
                layer: "shore",
            });
        }
        let u16_at = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
        let u32_at = |at: usize| {
            u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
        };
        if u16_at(8) != MAJOR {
            return Err(FormatError::UnknownDiscriminant {
                path: path.to_owned(),
                field: "shore major version",
                value: u16_at(8),
            });
        }
        let (spacing_um, width, height) = (u32_at(12), u32_at(16), u32_at(20));
        let (run_count, island_count, form_count) = (
            u32_at(24) as usize,
            u32_at(28) as usize,
            u32_at(32) as usize,
        );
        let body = HEADER
            + run_count * RUN_BYTES
            + island_count * ISLAND_BYTES
            + form_count * LANDFORM_BYTES;
        if bytes.len() != body + 32 {
            return Err(eof(body + 32));
        }
        if blake3::hash(&bytes[..body]).as_bytes() != &bytes[body..] {
            return Err(FormatError::Shore {
                path: path.to_owned(),
                reason: "checksum mismatch",
            });
        }
        let cells = (width as usize)
            .checked_mul(height as usize)
            .filter(|&c| c <= 1 << 32)
            .ok_or_else(|| FormatError::DimensionsOverflow {
                path: path.to_owned(),
                width,
                height,
            })?;
        let mut classes = vec![ShoreClass::None; cells];
        let bad = |reason| FormatError::Shore {
            path: path.to_owned(),
            reason,
        };
        let mut at = HEADER;
        for _ in 0..run_count {
            let (y, x) = (u32_at(at), u32_at(at + 4));
            let len = u32::from(u16_at(at + 8));
            let c =
                ShoreClass::from_u8(bytes[at + 10]).ok_or(FormatError::UnknownDiscriminant {
                    path: path.to_owned(),
                    field: "shore class",
                    value: u16::from(bytes[at + 10]),
                })?;
            at += RUN_BYTES;
            if y >= height || x.checked_add(len).is_none_or(|end| end > width) {
                return Err(bad("run outside the grid"));
            }
            let start = y as usize * width as usize + x as usize;
            for v in &mut classes[start..start + len as usize] {
                if *v != ShoreClass::None {
                    return Err(bad("overlapping runs"));
                }
                *v = c;
            }
        }
        let i64_at = |a: usize| {
            let mut b = [0_u8; 8];
            b.copy_from_slice(&bytes[a..a + 8]);
            i64::from_le_bytes(b)
        };
        let unknown = |field, value: u8| FormatError::UnknownDiscriminant {
            path: path.to_owned(),
            field,
            value: u16::from(value),
        };
        let mut islands = Vec::with_capacity(island_count);
        for _ in 0..island_count {
            let cause =
                IslandCause::from_u8(bytes[at + 22]).ok_or(FormatError::UnknownDiscriminant {
                    path: path.to_owned(),
                    field: "island cause",
                    value: u16::from(bytes[at + 22]),
                })?;
            islands.push(Island {
                x_um: i64_at(at),
                y_um: i64_at(at + 8),
                cells: u32_at(at + 16),
                top_m: i16::from_le_bytes([bytes[at + 20], bytes[at + 21]]),
                cause,
            });
            at += ISLAND_BYTES;
        }
        let mut landforms = Vec::with_capacity(form_count);
        for _ in 0..form_count {
            landforms.push(Landform {
                x_um: i64_at(at),
                y_um: i64_at(at + 8),
                area_km2: u32_at(at + 16),
                relief_m: i16::from_le_bytes([bytes[at + 20], bytes[at + 21]]),
                kind: LandformKind::from_u8(bytes[at + 22])
                    .ok_or_else(|| unknown("landform kind", bytes[at + 22]))?,
                cause: LandformCause::from_u8(bytes[at + 23])
                    .ok_or_else(|| unknown("landform cause", bytes[at + 23]))?,
            });
            at += LANDFORM_BYTES;
        }
        Ok(Self {
            spacing_um,
            width,
            height,
            classes,
            islands,
            landforms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ShoreLayer {
        let (w, h) = (7_u32, 3_u32);
        let mut classes = vec![ShoreClass::None; 21];
        classes[2] = ShoreClass::Cliff;
        classes[3] = ShoreClass::Cliff;
        classes[9] = ShoreClass::SandBeach;
        classes[20] = ShoreClass::Estuary;
        ShoreLayer {
            spacing_um: 100_000_000,
            width: w,
            height: h,
            classes,
            islands: vec![Island {
                x_um: 150_000_000,
                y_um: -3,
                cells: 12,
                top_m: 431,
                cause: IslandCause::Volcanic,
            }],
            landforms: vec![Landform {
                x_um: 9,
                y_um: 8,
                area_km2: 700,
                relief_m: -120,
                kind: LandformKind::Basin,
                cause: LandformCause::Rift,
            }],
        }
    }

    #[test]
    fn round_trips_and_detects_corruption() {
        let layer = sample();
        let bytes = layer.encode();
        assert_eq!(ShoreLayer::decode(&bytes, "t").unwrap(), layer);
        let mut bad = bytes.clone();
        bad[HEADER + 3] ^= 1;
        assert!(ShoreLayer::decode(&bad, "t").is_err());
        assert!(ShoreLayer::decode(&bytes[..bytes.len() - 1], "t").is_err());
        assert_eq!(layer.class_at(3, 0), ShoreClass::Cliff);
        assert_eq!(
            layer.class_near_um(190_000_000, 110_000_000),
            ShoreClass::SandBeach
        );
        assert_eq!(layer.class_at(99, 0), ShoreClass::None);
    }

    #[test]
    fn encoding_is_deterministic() {
        assert_eq!(sample().encode(), sample().encode());
    }
}
