//! The only byte codec in the workspace (`01-architecture.md`).
//!
//! Layouts are hand-specified little-endian; each layer documents its own
//! row format. No other crate encodes or decodes world bytes.

pub mod area_objects_v4;
pub mod blocks;
pub mod cells;
pub mod hydrology;
pub mod manifest;
pub mod objects;
pub mod overview;

/// The world-format major. Bumped only by a breaking layout change; loaders
/// refuse any other major with the regenerate remedy (`logic/05`).
/// 4: shared hydrology plus widened area records and discharge.
pub const FORMAT_VERSION: u32 = 4;

/// Writes a little-endian `u16` into `out`.
pub(crate) fn put_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Writes a little-endian `u32` into `out`.
pub(crate) fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Writes a little-endian `i32` into `out`.
pub(crate) fn put_i32(out: &mut Vec<u8>, v: i32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Writes a little-endian `i16` into `out`.
pub(crate) fn put_i16(out: &mut Vec<u8>, v: i16) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Reads a little-endian `u16` at `at`, advancing the cursor.
pub(crate) fn take_u16(src: &[u8], at: &mut usize) -> u16 {
    let v = u16::from_le_bytes([src[*at], src[*at + 1]]);
    *at += 2;
    v
}

/// Reads a little-endian `u32` at `at`, advancing the cursor.
pub(crate) fn take_u32(src: &[u8], at: &mut usize) -> u32 {
    let v = u32::from_le_bytes([src[*at], src[*at + 1], src[*at + 2], src[*at + 3]]);
    *at += 4;
    v
}

/// Reads a little-endian `i32` at `at`, advancing the cursor.
pub(crate) fn take_i32(src: &[u8], at: &mut usize) -> i32 {
    let v = i32::from_le_bytes([src[*at], src[*at + 1], src[*at + 2], src[*at + 3]]);
    *at += 4;
    v
}

/// Reads a little-endian `i16` at `at`, advancing the cursor.
pub(crate) fn take_i16(src: &[u8], at: &mut usize) -> i16 {
    let v = i16::from_le_bytes([src[*at], src[*at + 1]]);
    *at += 2;
    v
}

/// Reads one byte at `at`, advancing the cursor.
pub(crate) fn take_u8(src: &[u8], at: &mut usize) -> u8 {
    let v = src[*at];
    *at += 1;
    v
}

/// A self-deleting temporary directory for format round-trip tests.
///
/// `code-prefs.md` §Q6 bans mocks; temp dirs are the sanctioned substitute,
/// and hand-rolling this avoids a dev-dependency for ten lines.
#[cfg(test)]
pub(crate) struct TempDir(std::path::PathBuf);

#[cfg(test)]
impl TempDir {
    pub(crate) fn new() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("arda-test-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub(crate) fn path(&self) -> &std::path::Path {
        &self.0
    }
}

#[cfg(test)]
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Writes a little-endian u64.
pub(crate) fn put_u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}
/// Reads a little-endian u64 after the caller checks its record length.
pub(crate) fn take_u64(src: &[u8], at: &mut usize) -> u64 {
    let v = u64::from_le_bytes([
        src[*at],
        src[*at + 1],
        src[*at + 2],
        src[*at + 3],
        src[*at + 4],
        src[*at + 5],
        src[*at + 6],
        src[*at + 7],
    ]);
    *at += 8;
    v
}
