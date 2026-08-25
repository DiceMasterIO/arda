//! `blocks/<ax>_<ay>.tiles.zst` — one zstd frame per area (`mockup/02`).
//!
//! Blocks are stored in ascending `(cell_y, cell_x)` order so the bytes do
//! not depend on the order areas finished — `architecture-interview.md` §Q4
//! forbids thread count from changing a world.

use super::{put_u16, put_u32, take_u16, take_u32, take_u8};
use crate::coords::{CellCoord, SquareCoord, BLOCK_SQUARES};
use crate::error::FormatError;
use crate::tiles::TileId;
use std::collections::BTreeMap;

/// Archive magic.
pub const BLOCKS_MAGIC: &[u8; 8] = b"ARDABLK\0";

/// Fixed compression level — a varying level would change the bytes.
pub const ZSTD_LEVEL: i32 = 3;

/// Squares in one block.
const SQUARE_COUNT: usize = BLOCK_SQUARES as usize * BLOCK_SQUARES as usize;

/// One 64x64 tactical block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    tiles: Vec<TileId>,
    relaxed: bool,
}

impl Block {
    /// Builds a block with every square set to `fill`.
    #[must_use]
    pub fn filled(fill: TileId) -> Self {
        Self {
            tiles: vec![fill; SQUARE_COUNT],
            relaxed: false,
        }
    }

    /// Reads one square.
    ///
    /// Invariant: `SquareCoord` is bounds-checked and `tiles` is always
    /// `SQUARE_COUNT` long.
    #[must_use]
    pub fn square(&self, at: SquareCoord) -> TileId {
        self.tiles[at.index()]
    }

    /// Overwrites one square.
    pub fn set(&mut self, at: SquareCoord, tile: TileId) {
        self.tiles[at.index()] = tile;
    }

    /// Marks this block as a relaxed fallback fill (`logic/03` §Q12).
    pub fn mark_relaxed(&mut self) {
        self.relaxed = true;
    }

    /// Whether the WFC fell back to a relaxed fill here.
    #[must_use]
    pub const fn is_relaxed(&self) -> bool {
        self.relaxed
    }
}

/// Every block generated for one area tile.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BlockArchive {
    blocks: BTreeMap<(u16, u16), Block>,
}

impl BlockArchive {
    /// Stores a block for one cell.
    pub fn insert(&mut self, cell: CellCoord, block: Block) {
        self.blocks.insert((cell.y(), cell.x()), block);
    }

    /// Reads the block for one cell, when generated.
    #[must_use]
    pub fn get(&self, cell: CellCoord) -> Option<&Block> {
        self.blocks.get(&(cell.y(), cell.x()))
    }

    /// Number of stored blocks.
    #[must_use]
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    /// Whether the archive holds no blocks.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}

/// Encodes and compresses an area's block archive.
///
/// # Errors
/// Returns [`FormatError::Io`] when zstd fails to compress.
pub fn encode_blocks(archive: &BlockArchive) -> Result<Vec<u8>, FormatError> {
    let mut raw =
        Vec::with_capacity(BLOCKS_MAGIC.len() + 4 + archive.len() * (5 + SQUARE_COUNT * 2));
    raw.extend_from_slice(BLOCKS_MAGIC);
    put_u32(&mut raw, u32::try_from(archive.len()).unwrap_or(u32::MAX));

    // BTreeMap iterates in ascending key order — the determinism guarantee.
    for (&(cy, cx), block) in &archive.blocks {
        put_u16(&mut raw, cx);
        put_u16(&mut raw, cy);
        raw.push(u8::from(block.relaxed));
        for tile in &block.tiles {
            put_u16(&mut raw, tile.raw());
        }
    }

    zstd::encode_all(raw.as_slice(), ZSTD_LEVEL).map_err(|e| FormatError::Io {
        path: "block archive".to_owned(),
        source: e,
    })
}

/// Decompresses and decodes an area's block archive.
///
/// `ponytail:` whole-archive decompression; `logic/05` step 3 wants per-cell
/// lazy decompression. Upgrade path when an area's archive stops fitting in
/// memory: write one zstd frame per block plus an offset index in the header,
/// and seek to the frame.
///
/// # Errors
/// - [`FormatError::Io`] when the zstd frame will not decompress.
/// - [`FormatError::BadMagic`] when the payload is not a block archive.
/// - [`FormatError::UnexpectedEof`] when a block runs past the end.
pub fn decode_blocks(path: &str, bytes: &[u8]) -> Result<BlockArchive, FormatError> {
    let raw = zstd::decode_all(bytes).map_err(|e| FormatError::Io {
        path: path.to_owned(),
        source: e,
    })?;

    if raw.len() < BLOCKS_MAGIC.len() || &raw[..BLOCKS_MAGIC.len()] != BLOCKS_MAGIC {
        return Err(FormatError::BadMagic {
            path: path.to_owned(),
            layer: "blocks",
        });
    }

    let mut at = BLOCKS_MAGIC.len();
    let eof = |expected: usize| FormatError::UnexpectedEof {
        path: path.to_owned(),
        read: raw.len(),
        expected,
    };
    if at + 4 > raw.len() {
        return Err(eof(at + 4));
    }
    let count = take_u32(&raw, &mut at) as usize;

    let mut archive = BlockArchive::default();
    let per_block = 5 + SQUARE_COUNT * 2;
    for _ in 0..count {
        if at + per_block > raw.len() {
            return Err(eof(at + per_block));
        }
        let cx = take_u16(&raw, &mut at);
        let cy = take_u16(&raw, &mut at);
        let relaxed = take_u8(&raw, &mut at) == 1;
        let mut tiles = Vec::with_capacity(SQUARE_COUNT);
        for _ in 0..SQUARE_COUNT {
            tiles.push(TileId::new(take_u16(&raw, &mut at)));
        }
        let cell = CellCoord::new(cx, cy).ok_or(FormatError::UnknownDiscriminant {
            path: path.to_owned(),
            field: "block cell coordinate",
            value: cx.max(cy),
        })?;
        archive.insert(cell, Block { tiles, relaxed });
    }
    Ok(archive)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tiles::{TileGroup, SKELETON_TILES};

    fn cc(x: u16, y: u16) -> CellCoord {
        CellCoord::new(x, y).unwrap()
    }

    fn sq(x: u8, y: u8) -> SquareCoord {
        SquareCoord::new(x, y).unwrap()
    }

    fn sample() -> BlockArchive {
        let mut archive = BlockArchive::default();

        let mut a = Block::filled(TileId::new(1));
        a.set(sq(0, 0), TileId::new(9));
        a.set(sq(63, 63), TileId::new(23));
        archive.insert(cc(4, 7), a);

        let mut b = Block::filled(TileId::new(5));
        b.mark_relaxed();
        archive.insert(cc(1, 2), b);

        archive
    }

    #[test]
    fn archive_round_trips_through_zstd() {
        let bytes = encode_blocks(&sample()).unwrap();
        let back = decode_blocks("blocks/00_00.tiles.zst", &bytes).unwrap();
        assert_eq!(back, sample());
    }

    #[test]
    fn relaxed_mark_survives_the_round_trip() {
        let bytes = encode_blocks(&sample()).unwrap();
        let back = decode_blocks("blocks/00_00.tiles.zst", &bytes).unwrap();
        assert!(back.get(cc(1, 2)).unwrap().is_relaxed());
        assert!(!back.get(cc(4, 7)).unwrap().is_relaxed());
    }

    #[test]
    fn encoding_is_independent_of_insertion_order() {
        // architecture-interview.md §Q4: thread count must not change bytes.
        let mut forward = BlockArchive::default();
        forward.insert(cc(1, 2), Block::filled(TileId::new(5)));
        forward.insert(cc(4, 7), Block::filled(TileId::new(1)));

        let mut reverse = BlockArchive::default();
        reverse.insert(cc(4, 7), Block::filled(TileId::new(1)));
        reverse.insert(cc(1, 2), Block::filled(TileId::new(5)));

        assert_eq!(
            encode_blocks(&forward).unwrap(),
            encode_blocks(&reverse).unwrap()
        );
    }

    #[test]
    fn empty_archive_round_trips() {
        let bytes = encode_blocks(&BlockArchive::default()).unwrap();
        let back = decode_blocks("blocks/00_00.tiles.zst", &bytes).unwrap();
        assert_eq!(back.len(), 0);
        assert!(back.is_empty());
    }

    #[test]
    fn corrupt_frame_is_refused_naming_the_file() {
        let mut bytes = encode_blocks(&sample()).unwrap();
        let n = bytes.len();
        bytes[n / 2] ^= 0xFF;
        bytes[n / 2 + 1] ^= 0xFF;
        let err = decode_blocks("blocks/03_11.tiles.zst", &bytes).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("blocks/03_11.tiles.zst"), "message was: {msg}");
    }

    #[test]
    fn vocabulary_has_twenty_four_unique_tiles() {
        // implementation.md step 3: "~24-tile WFC subset".
        assert_eq!(SKELETON_TILES.len(), 24);
        let mut ids: Vec<u16> = SKELETON_TILES.iter().map(|t| t.id.raw()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 24, "tile ids must be unique");
        assert_eq!(ids[0], 0, "id 0 is reserved for deep water");
    }

    #[test]
    fn adjacency_is_symmetric() {
        // logic/03 §Q12: adjacency is a relation, not a direction.
        for a in SKELETON_TILES {
            for b in SKELETON_TILES {
                assert_eq!(
                    crate::tiles::may_adjoin(a.id, b.id),
                    crate::tiles::may_adjoin(b.id, a.id),
                    "{} vs {}",
                    a.name,
                    b.name
                );
            }
        }
    }

    #[test]
    fn deep_water_never_touches_dry_ground() {
        let deep = SKELETON_TILES
            .iter()
            .find(|t| t.group == TileGroup::Water && t.name == "deep_water")
            .unwrap();
        let grass = SKELETON_TILES.iter().find(|t| t.name == "grass").unwrap();
        assert!(!crate::tiles::may_adjoin(deep.id, grass.id));
    }
}
