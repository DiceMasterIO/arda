//! Formed shading exactly as v0.1 drew recipe-5 worlds (logic/04
//! §atlas-formed recipes). Recipe-6 worlds use the v0.2 palette, surface
//! mottling, sub-grid detail, stored shore classes, snow sky light and the
//! highlight shoulder in [`super::shade`]; recipe-5 worlds keep rendering
//! identically through this module.

use super::{
    mix, organic_noise, relief_term, rgb, smooth, soft_q12, FormedInputs, EXAGGERATION_Q8, ONE,
    SCALE_WEIGHT_Q12,
};

/// Shaded land colour for one fine point, as recipe-5 worlds render it.
pub(in crate::atlas) fn shade(p: &FormedInputs) -> [u8; 3] {
    let e = i64::from(p.height_mm);
    // Material slope and aspect are prefiltered to the output scale: the
    // 39 m gradient at close range, the 312 m one once a pixel spans a few
    // hundred metres, so gully-scale canopy and rock do not average into
    // speckle (logic/04 §atlas-formed vegetation). Lighting still uses every
    // scale below.
    let broad = smooth(
        40_000_000,
        250_000_000,
        i64::try_from(p.footprint_um).unwrap_or(i64::MAX),
    );
    let (fx0, fy0) = p.gradients[0];
    let (mx0, my0) = p.gradients[1];
    let gx = fx0 + (mx0 - fx0) * broad / ONE;
    let gy = fy0 + (my0 - fy0) * broad / ONE;
    let slope = i64::try_from(
        (i128::from(gx) * i128::from(gx) + i128::from(gy) * i128::from(gy))
            .unsigned_abs()
            .isqrt(),
    )
    .unwrap_or(0);
    let mo = i64::from(p.moisture) * ONE / 255;
    let conc = p.concavity_mm;

    // Vegetation by elevation band, moisture-dependent (warm palette).
    let low = mix(rgb(0x8c_8a4c), rgb(0x66_7338), mo);
    let mid = mix(rgb(0xa8_9058), rgb(0x8a_8446), mo);
    let high = rgb(0x8e_7658);
    let mut c = if e < 600_000 {
        mix(low, mid, smooth(100_000, 600_000, e))
    } else {
        mix(mid, high, smooth(600_000, 1_900_000, e))
    };
    // Green, wet valley floors; drier ochre crests.
    // Steep, rugged ground (steep at 39 m and 312 m) keeps rock colour in
    // its rills: greening every one-cell gully drew a hatch of thin lines
    // across rock faces (logic/04 §atlas-formed materials).
    let (m0x, m0y) = p.gradients[1];
    let mid0 = i64::try_from(
        (i128::from(m0x) * i128::from(m0x) + i128::from(m0y) * i128::from(m0y))
            .unsigned_abs()
            .isqrt(),
    )
    .unwrap_or(0);
    let rugged = smooth(1_024, 2_458, mid0);
    let valley = smooth(5_000, 60_000, conc) / 2 * (ONE - smooth(1_200_000, 2_000_000, e)) / ONE
        * (ONE - rugged)
        / ONE;
    c = mix(c, rgb(0x62_7236), valley);
    // Forest canopy (goal 19): needs moisture and warmth; the tree line
    // falls out of temperature at height (≈ +1…+4 °C mean annual); steep
    // faces thin it. Mottled at 200 m and 60 m like tree clusters.
    // Moisture is saved per 100 m cell with 4–13 km soil patches; blend in
    // a fine-scale term so canopy edges are organic, not blobs.
    let (fx, fy) = p.position_m;
    // Texture only where its ~450 m patches span several pixels.
    let fine_keep = ONE
        - smooth(
            60_000_000,
            200_000_000,
            i64::try_from(p.footprint_um).unwrap_or(i64::MAX),
        );
    let fine_mo = mo + (organic_noise(fx, fy, 900) - ONE / 2) * 4 / 5 * fine_keep / ONE;
    let forest = smooth(90 * ONE / 255, 170 * ONE / 255, fine_mo)
        * smooth(100, 400, i64::from(p.temperature_centi))
        / ONE
        * (ONE - smooth(2_458, 4_096, slope))
        / ONE;
    // Sun-facing (south) slopes are drier: thinner canopy, more ochre;
    // shaded north slopes keep denser forest (goal 19, aspect).
    let south = if slope > 0 {
        (gy * ONE / slope).clamp(0, ONE)
    } else {
        0
    };
    let dry = south * smooth(410, 2_048, slope) / ONE;
    c = mix(c, rgb(0xa0_8a55), dry / 5);
    let forest = forest * (ONE - 2 * dry / 5) / ONE;
    if forest > 0 {
        let (xm, ym) = p.position_m;
        // Tree-cluster mottle fades to its mean once pixels exceed its
        // wavelength, so overviews do not alias it into speckle.
        let raw = (organic_noise(xm, ym, 200) * 2 + organic_noise(xm + 401, ym - 257, 60)) / 3;
        let keep = ONE
            - smooth(
                50_000_000,
                200_000_000,
                i64::try_from(p.footprint_um).unwrap_or(i64::MAX),
            );
        let mottle = ONE / 2 + (raw - ONE / 2) * keep / ONE;
        let canopy = mix(rgb(0x4a_5f2e), rgb(0x5e_7236), mottle);
        c = mix(c, canopy, forest * (2_253 + 1_843 * mottle / ONE) / ONE);
    }
    c = mix(
        c,
        rgb(0x5a_7038),
        i64::from(p.wetness) * ONE / 255 / 3 * (ONE - rugged) / ONE,
    );
    c = mix(c, rgb(0xb0_8e4e), smooth(5_000, 60_000, -conc) / 4);
    // Rock on steep faces, more at altitude.
    // Rock needs steepness at both 39 m and 312 m: small incised lowland
    // gullies stay vegetated; real mountain faces and cliffs show rock.
    let (mx, my) = p.gradients[1];
    let slope_mid = i64::try_from(
        (i128::from(mx) * i128::from(mx) + i128::from(my) * i128::from(my))
            .unsigned_abs()
            .isqrt(),
    )
    .unwrap_or(0);
    let rock = smooth(1_843, 3_686, slope) * smooth(1_024, 2_048, slope_mid) / ONE
        * (1_024 + 3_072 * smooth(300_000, 1_500_000, e) / ONE)
        / ONE;
    c = mix(c, rgb(0x74_6e68), rock);
    // Shore: sand on low, gentle ground touching the sea; bare rock where
    // steep ground meets it (goals 16, 31).
    if p.ring_min_mm < 0 {
        let low = ONE - smooth(1_500, 6_000, e);
        let gentle = ONE - smooth(410, 1_229, slope);
        c = mix(c, rgb(0xdc_c99a), low * gentle / ONE);
        c = mix(
            c,
            rgb(0x94_8a7c),
            smooth(1_229, 2_458, slope) * (ONE - smooth(20_000, 80_000, e)) / ONE,
        );
    }
    // Snow and ice from mean annual temperature at this height (goal 20):
    // perennial snow ramps in from -4 C to -7 C (the Alpine ELA sits near
    // -5 C mean annual). Sun-facing (south) slopes count 1.5 C warmer and
    // gullies up to 2.5 C colder, so snow streaks down couloirs; convex
    // ridges are wind-scoured (up to 1.5 C warmer); cliffs shed snow.
    let aspect_south = if slope > 0 { gy * ONE / slope } else { 0 };
    let t_eff = i64::from(p.temperature_centi) + 150 * aspect_south / ONE
        - 250 * smooth(0, 40_000, conc) / ONE
        + 150 * smooth(0, 40_000, -conc) / ONE;
    let snow = smooth(-400, -700, t_eff) * (ONE - 3 * smooth(2_867, 5_325, slope) / 4) / ONE;
    c = mix(c, rgb(0xde_d8d1), snow);

    // Multi-scale light.
    let mut lit = 0;
    for k in 0..4 {
        let (gx, gy) = p.gradients[k];
        lit += SCALE_WEIGHT_Q12[k] * relief_term(gx, gy, EXAGGERATION_Q8[k]) / ONE;
    }
    let l = soft_q12(lit * 12 / 5);
    let ao = (ONE - 901 * smooth(0, 120_000, conc) / ONE) * (ONE * 3 + p.sky_q12) / (4 * ONE);
    let k = (ONE + 2_662 * l / ONE) * ao / ONE;
    let sw = (-l).max(0) * 1_843 / ONE;
    let shadow = [1_229_i64, 983, 1_065];
    let hl = l.max(0) * 492 / ONE;
    let hl_tint = [ONE, 3_891, 3_277];
    std::array::from_fn(|ch| {
        let base = c[ch] * k / ONE;
        let tinted = base * (ONE - sw) / ONE + shadow[ch] * sw / ONE * c[ch].max(77) / ONE;
        let v =
            tinted + (hl * hl_tint[ch] / ONE) * 255 / ONE + if ch == 2 { 8 * sw / ONE } else { 0 };
        u8::try_from(v.clamp(0, 255)).unwrap_or(255)
    })
}

/// Lake water by depth (mm): a lighter shallow margin grading to deep blue
/// (goal 29).
const LAKE_STOPS: [(i64, u32); 5] = [
    (0, 0x6f_b6cc),
    (6_000, 0x3f_8fb8),
    (20_000, 0x27_6fa4),
    (60_000, 0x1a_5289),
    (150_000, 0x12_3d6e),
];

/// Lake colour for one fine point below the saved water surface, as
/// recipe-5 worlds render it.
pub(in crate::atlas) fn lake(depth_mm: i64) -> [u8; 3] {
    let d = depth_mm.max(0);
    let mut c = rgb(LAKE_STOPS[LAKE_STOPS.len() - 1].1);
    for pair in LAKE_STOPS.windows(2) {
        let (d0, c0) = pair[0];
        let (d1, c1) = pair[1];
        if d < d1 {
            c = mix(rgb(c0), rgb(c1), (d - d0) * ONE / (d1 - d0));
            break;
        }
    }
    std::array::from_fn(|ch| u8::try_from(c[ch].clamp(0, 255)).unwrap_or(255))
}
