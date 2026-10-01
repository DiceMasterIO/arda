//! Parallel PNG encoding of tactical renders (goal 50).
//!
//! A 4096² px battle map is 64 MiB of RGBA. The image is cut into strips of
//! [`STRIP_ROWS`] rows; each strip is Paeth-filtered and deflated on its own
//! thread as raw deflate ending in a sync flush, so the strips concatenate
//! into one valid zlib stream (the last strip finishes it). The Adler-32 of
//! the whole stream is combined from the strips'. Strip boundaries depend
//! only on the image size, never on the thread count, so equal pixels give
//! equal bytes.

use crate::error::{ServerError, ServerResult};
use flate2::{Compress, Compression, FlushCompress, Status};
use rayon::prelude::*;

/// Rows per independently compressed strip.
pub const STRIP_ROWS: usize = 64;
/// Adler-32 modulus.
const BASE: u32 = 65_521;

fn image_err(what: impl std::fmt::Display) -> ServerError {
    ServerError::Image(format!("png: {what}"))
}

/// Encodes straight-alpha RGBA8 pixels as a PNG: colour type 2 (RGB) when
/// every pixel is opaque, as painted maps are, else 6 (RGBA); depth 8.
///
/// # Errors
/// [`ServerError::Image`] on a size mismatch or a compressor failure.
pub fn encode_png(width: u32, height: u32, rgba: &[u8]) -> ServerResult<Vec<u8>> {
    let (w, h) = (width as usize, height as usize);
    if w == 0 || h == 0 || rgba.len() != w * 4 * h {
        return Err(image_err("pixel buffer does not match the size"));
    }
    let opaque = rgba
        .par_chunks(1 << 16)
        .all(|c| c.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    let bpp = if opaque { 3 } else { 4 };
    let strips = h.div_ceil(STRIP_ROWS);
    let parts: Vec<ServerResult<(Vec<u8>, u32, usize)>> = (0..strips)
        .into_par_iter()
        .map(|s| {
            let (y0, y1) = (s * STRIP_ROWS, ((s + 1) * STRIP_ROWS).min(h));
            let filtered = filter_rows(rgba, w, bpp, y0, y1);
            let data = deflate(&filtered, s + 1 == strips)?;
            Ok((data, adler32(&filtered), filtered.len()))
        })
        .collect();
    let mut zlib = vec![0x78, 0x01];
    let mut adler = 1u32;
    for p in parts {
        let (data, a, len) = p?;
        zlib.extend_from_slice(&data);
        adler = adler32_combine(adler, a, len);
    }
    zlib.extend_from_slice(&adler.to_be_bytes());
    let mut out = Vec::with_capacity(zlib.len() + 64);
    out.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, if opaque { 2 } else { 6 }, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr)?;
    // IDAT chunks of at most 1 MiB, as encoders conventionally write them.
    for part in zlib.chunks(1 << 20) {
        chunk(&mut out, b"IDAT", part)?;
    }
    chunk(&mut out, b"IEND", &[])?;
    Ok(out)
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) -> ServerResult<()> {
    let len = u32::try_from(data.len()).map_err(|_| image_err("chunk too large"))?;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc = crc32fast::Hasher::new();
    crc.update(kind);
    crc.update(data);
    out.extend_from_slice(&crc.finalize().to_be_bytes());
    Ok(())
}

/// Row `y` of `rgba` packed to `bpp` bytes a pixel (3 drops alpha).
fn packed(rgba: &[u8], w: usize, bpp: usize, y: usize) -> Vec<u8> {
    let row = &rgba[y * w * 4..(y + 1) * w * 4];
    if bpp == 4 {
        return row.to_vec();
    }
    row.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect()
}

/// Rows `y0..y1`, each prefixed by filter type 4 (Paeth) and filtered
/// against the row above (none for row 0).
fn filter_rows(rgba: &[u8], w: usize, bpp: usize, y0: usize, y1: usize) -> Vec<u8> {
    let n = w * bpp;
    let mut out = Vec::with_capacity((y1 - y0) * (n + 1));
    let mut up = (y0 > 0).then(|| packed(rgba, w, bpp, y0 - 1));
    for y in y0..y1 {
        let cur = packed(rgba, w, bpp, y);
        out.push(4);
        for i in 0..n {
            let (a, b, c) = match (&up, i >= bpp) {
                (Some(u), true) => (cur[i - bpp], u[i], u[i - bpp]),
                (Some(u), false) => (0, u[i], 0),
                // Paeth with no row above reduces to Sub.
                (None, true) => (cur[i - bpp], 0, 0),
                (None, false) => (0, 0, 0),
            };
            out.push(cur[i].wrapping_sub(paeth(a, b, c)));
        }
        up = Some(cur);
    }
    out
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let (ia, ib, ic) = (i16::from(a), i16::from(b), i16::from(c));
    let p = ia + ib - ic;
    let (pa, pb, pc) = ((p - ia).abs(), (p - ib).abs(), (p - ic).abs());
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// Raw deflate of `data` at the fastest level, ending with a sync flush
/// (or the final block when `last`).
fn deflate(data: &[u8], last: bool) -> ServerResult<Vec<u8>> {
    let mut c = Compress::new(Compression::fast(), false);
    let mut out = Vec::with_capacity(data.len() / 2 + 1024);
    let flush = if last {
        FlushCompress::Finish
    } else {
        FlushCompress::Sync
    };
    loop {
        if out.len() == out.capacity() {
            out.reserve(out.capacity().max(4096));
        }
        let before = c.total_out();
        let consumed = usize::try_from(c.total_in()).map_err(image_err)?;
        let rest = data.get(consumed..).unwrap_or(&[]);
        let status = c.compress_vec(rest, &mut out, flush).map_err(image_err)?;
        let all_in = usize::try_from(c.total_in()).map_err(image_err)? == data.len();
        // A sync flush is complete once all input is consumed, the
        // compressor stopped short of filling the output and the stream
        // ends on the flush's empty stored block (`00 00 FF FF`). The
        // compressor may return with room to spare before it has written
        // the marker, so room alone is not enough: a strip without its
        // marker corrupts every strip after it.
        let flushed = all_in && out.len() < out.capacity() && out.ends_with(&SYNC_MARKER);
        match status {
            Status::StreamEnd => break,
            _ if !last && flushed => break,
            _ if all_in && out.len() < out.capacity() && c.total_out() == before => {
                // No progress with room to spare: the flush cannot finish.
                return Err(image_err("deflate sync flush did not complete"));
            }
            _ => {}
        }
    }
    Ok(out)
}

/// The tail of a completed sync flush: an empty stored block's lengths.
const SYNC_MARKER: [u8; 4] = [0x00, 0x00, 0xFF, 0xFF];

/// Adler-32 of `data`.
fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    // 5552 bytes keep the sums below 2^32 between reductions.
    for block in data.chunks(5552) {
        for &x in block {
            a += u32::from(x);
            b += a;
        }
        a %= BASE;
        b %= BASE;
    }
    (b << 16) | a
}

/// The Adler-32 of `A ++ B` from those of `A` and `B` (zlib's
/// `adler32_combine`).
fn adler32_combine(a1: u32, a2: u32, len2: usize) -> u32 {
    let rem = u32::try_from(len2 % BASE as usize).unwrap_or(0);
    let mut sum1 = a1 & 0xffff;
    let mut sum2 = u32::try_from(u64::from(rem) * u64::from(sum1) % u64::from(BASE)).unwrap_or(0);
    sum1 += (a2 & 0xffff) + BASE - 1;
    sum2 += (a1 >> 16) + (a2 >> 16) + BASE - rem;
    if sum1 >= BASE {
        sum1 -= BASE;
    }
    if sum1 >= BASE {
        sum1 -= BASE;
    }
    if sum2 >= BASE << 1 {
        sum2 -= BASE << 1;
    }
    if sum2 >= BASE {
        sum2 -= BASE;
    }
    sum1 | (sum2 << 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
        let mut r = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .unwrap();
        let mut buf = vec![0; r.output_buffer_size()];
        let info = r.next_frame(&mut buf).unwrap();
        buf.truncate(info.buffer_size());
        (info.width, info.height, buf)
    }

    fn picture(w: u32, h: u32) -> Vec<u8> {
        (0..w * h)
            .flat_map(|i| {
                let (x, y) = (i % w, i / w);
                let v = |k: u32| u8::try_from((x * k + y * (k + 3) + (x ^ y)) % 251).unwrap();
                [v(3), v(5), v(7), u8::try_from(254 - (x + y) % 40).unwrap()]
            })
            .collect()
    }

    #[test]
    fn strips_round_trip_through_a_standard_decoder() {
        for (w, h) in [(1, 1), (7, 3), (130, 64), (257, 200)] {
            let rgba = picture(w, h);
            let png = encode_png(w, h, &rgba).unwrap();
            assert_eq!(decode(&png), (w, h, rgba.clone()), "{w}x{h}");
            let opaque: Vec<u8> = rgba
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|p| [p[0], p[1], p[2], 255])
                .collect();
            let rgb: Vec<u8> = rgba
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|p| [p[0], p[1], p[2]])
                .collect();
            let png = encode_png(w, h, &opaque).unwrap();
            assert_eq!(decode(&png), (w, h, rgb), "opaque {w}x{h} as RGB");
            assert_eq!(png, encode_png(w, h, &opaque).unwrap(), "deterministic");
        }
    }

    /// Noisy strips compress to more than the compressor's internal
    /// buffer, so a strip's sync flush arrives over several calls (a map
    /// of 2048 × 2432 px once lost a flush marker mid-stream).
    #[test]
    fn noisy_strips_keep_their_flush_markers() {
        let (w, h) = (2048_u32, 704_u32);
        let mut s = 0x9E37_79B9_7F4A_7C15_u64;
        let rgba: Vec<u8> = (0..w * h)
            .flat_map(|_| {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                let b = s.to_le_bytes();
                [b[0], b[1], b[2], 255]
            })
            .collect();
        let png = encode_png(w, h, &rgba).unwrap();
        let rgb: Vec<u8> = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect();
        assert_eq!(decode(&png), (w, h, rgb));
    }

    #[test]
    fn combined_adler_equals_the_whole() {
        let data: Vec<u8> = (0..70_000u32)
            .map(|i| (i * 31 % 256).to_le_bytes()[0])
            .collect();
        for cut in [0, 1, 5552, 40_000, 70_000] {
            let (a, b) = data.split_at(cut);
            assert_eq!(
                adler32_combine(adler32(a), adler32(b), b.len()),
                adler32(&data)
            );
        }
    }

    #[test]
    fn wrong_sizes_are_refused() {
        assert!(encode_png(2, 2, &[0; 15]).is_err());
        assert!(encode_png(0, 2, &[]).is_err());
    }
}
