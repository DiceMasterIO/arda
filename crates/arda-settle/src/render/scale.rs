//! How big names and settlement symbols are drawn at a given output size
//! (logic/04 §overlay, logic/17 §labels).
//!
//! A world is a hundred kilometres across; on a 2,048 pixel overview a
//! 17 pixel city name and a 7 pixel ring are specks. Symbols and names grow
//! with the output width, reaching 1.6 times the offline page's size at
//! 2,048 pixels and 2.4 times from 3,072 pixels up. Below 1,280 pixels
//! symbols shrink toward 0.6 so thumbnails stay uncluttered; names never
//! drop under the page size.

/// Output width (pixels) at which symbols are at the offline page's size.
const UNIT_WIDTH_PX: f32 = 1_280.0;
/// Smallest symbol scale.
const SYMBOL_MIN: f32 = 0.6;
/// Largest symbol and name scale.
const MAX: f32 = 2.4;

/// Metres per pixel above which hamlets, and the roads built to reach
/// them, are left off (logic/04 §overlay). World-scale outputs (the
/// offline page, overview exports and the overview pyramid of any world
/// longer than 61 km on a 2,048 pixel image, or 123 km on the 4,096 pixel
/// pyramid base, so every MICRO and default-size world) are coarser and show
/// cities, towns and villages only; area exports and relief tiles are
/// finer and show every hamlet. It is the tracks' own switch
/// ([`super::band::TRACK_MAX_M`]), so a hamlet and its track come and go
/// together.
pub const HAMLET_MAX_M: f32 = 30.0;

/// Whether an image at `m_per_px` metres per pixel shows hamlets.
#[must_use]
pub fn shows_hamlets(m_per_px: f32) -> bool {
    m_per_px <= HAMLET_MAX_M
}

/// Scale of settlement symbols on an overview `width_px` pixels across.
#[must_use]
pub fn symbol_scale(width_px: f32) -> f32 {
    (width_px / UNIT_WIDTH_PX).clamp(SYMBOL_MIN, MAX)
}

/// Scale of place names on an image `width_px` pixels across: the symbol
/// scale, but never below the page size 1.
#[must_use]
pub fn label_unit_for_width(width_px: f32) -> f32 {
    symbol_scale(width_px).max(1.0)
}

/// Scale of place names at pyramid zoom `z`, where `base_zoom` is the
/// finest overview level: it is drawn like a 2,048 pixel overview
/// ([`label_unit_for_width`]), the next finer level at 1.3, and every level
/// from two finer on at the page size, so close-zoom tiles are unchanged.
/// Coarser levels grow 0.2 per level up to 2.2.
#[must_use]
pub fn level_unit(z: u8, base_zoom: u8) -> f32 {
    match i32::from(z) - i32::from(base_zoom) {
        d if d >= 2 => 1.0,
        1 => 1.3,
        0 => label_unit_for_width(2_048.0),
        d => (label_unit_for_width(2_048.0) - 0.2 * d as f32).min(2.2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_2048_overview_draws_names_and_symbols_clearly_bigger_than_page_size() {
        assert!((1.5..=1.7).contains(&symbol_scale(2_048.0)));
        assert!((1.5..=1.7).contains(&label_unit_for_width(2_048.0)));
    }

    #[test]
    fn hamlets_show_at_area_scale_and_not_at_world_scale() {
        // The default world (460 km) on a 4,096 px page: about 240 m/px.
        assert!(!shows_hamlets(240.0));
        // An 8K area export (6.25 m/px) and a 2K one (25 m/px).
        assert!(shows_hamlets(6.25) && shows_hamlets(25.0));
        assert!((HAMLET_MAX_M - super::super::band::TRACK_MAX_M).abs() < f32::EPSILON);
    }

    #[test]
    fn scales_grow_with_the_output_and_are_bounded() {
        let mut last = 0.0;
        for w in [256.0, 512.0, 1_024.0, 2_048.0, 4_096.0, 8_192.0, 32_768.0] {
            let s = symbol_scale(w);
            assert!(s >= last && (SYMBOL_MIN..=MAX).contains(&s), "{w}: {s}");
            last = s;
        }
        assert!(label_unit_for_width(256.0) >= 1.0);
    }

    #[test]
    fn pyramid_levels_keep_page_size_when_fine_and_grow_when_coarse() {
        assert!((level_unit(6, 4) - 1.0).abs() < 1e-6);
        assert!((level_unit(9, 4) - 1.0).abs() < 1e-6);
        assert!(level_unit(5, 4) > 1.2 && level_unit(5, 4) < level_unit(4, 4));
        assert!(level_unit(3, 4) > level_unit(4, 4));
        assert!(level_unit(0, 4) <= 2.2 + 1e-6);
    }
}
