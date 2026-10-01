//! Recipe-5 land shading (logic/04 §atlas-formed): multi-scale relief light,
//! terrain-driven materials and soft painterly shadows, all integer Q12.
//!
//! Recipe 5 terrain carries real drainage structure at every scale, so the
//! shader lights it at four baselines (39 m, 312 m, 1 km, 3 km). Whole ranges
//! get a lit and a shadowed flank while ridges and gullies stay crisp.
//! Materials come from slope, concavity, aspect and height of the canonical
//! field itself rather than from 100 m saved cells, so colour follows
//! landforms without blocks.

/// Q12 one.
mod v5;

const ONE: i64 = 4_096;
/// Light from the north-west at 42 degrees: horizontal component per axis
/// (cos 42 / sqrt 2) and vertical component (sin 42), Q12.
const LIGHT_AXIS: i64 = 2_152;
const LIGHT_UP: i64 = 2_741;

/// Heights and slopes sampled around one fine point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FormedInputs {
    /// Height at the point, millimetres.
    pub height_mm: i32,
    /// Absolute position of the point in metres (for seamless texture).
    pub position_m: (i64, i64),
    /// The same position in decimetres, for sub-grid relief at close zoom.
    pub position_dm: (i64, i64),
    /// Physical pixel footprint, micrometres (texture fades before aliasing).
    pub footprint_um: i128,
    /// Gradients (dz/dx, dz/dy) as Q12 slopes at 39 m, 312 m, 1 km and 3 km.
    pub gradients: [(i64, i64); 4],
    /// Mean height of the ring at 156 m minus the point, millimetres
    /// (positive in gullies and valleys, negative on crests).
    pub concavity_mm: i64,
    /// Broad sky exposure, Q12 (4096 = open sky).
    pub sky_q12: i64,
    /// Lowest height on the 156 m ring, millimetres (negative at the shore).
    pub ring_min_mm: i64,
    /// Mean annual temperature at this height, centi-degrees Celsius.
    pub temperature_centi: i32,
    /// Saved moisture, 0..=255.
    pub moisture: u8,
    /// Saved wetness, 0..=255.
    pub wetness: u8,
    /// Stored shore-class weights (Q12) by `ShoreClass` discriminant; all
    /// zero when the world has no shore layer.
    pub shore: [i64; 8],
    /// Recipe-6 look; false renders exactly as v0.1 drew recipe 5
    /// ([`v5::shade`]).
    pub v6: bool,
}

/// Vertical exaggeration per lighting scale, Q8.
const EXAGGERATION_Q8: [i64; 4] = [256, 384, 768, 1_280];
/// Lighting weights per scale, Q12 (0.30, 0.25, 0.25, 0.20).
const SCALE_WEIGHT_Q12: [i64; 4] = [1_229, 1_024, 1_024, 819];

fn smooth(e0: i64, e1: i64, x: i64) -> i64 {
    let t = ((x - e0) * ONE / (e1 - e0)).clamp(0, ONE);
    t * t / ONE * (3 * ONE - 2 * t) / ONE
}

/// Deterministic bilinear value noise in [0, ONE] on a `period_m` lattice.
fn value_noise(x_m: i64, y_m: i64, period_m: i64) -> i64 {
    let hash = |ix: i64, iy: i64| {
        let mut v = ix.cast_unsigned().wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ iy.cast_unsigned().wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
            ^ 0x0F0E_57C0;
        v ^= v >> 29;
        v = v.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        v ^= v >> 32;
        i64::try_from(v % 4_097).unwrap_or(0)
    };
    let (ix, iy) = (x_m.div_euclid(period_m), y_m.div_euclid(period_m));
    let tx = x_m.rem_euclid(period_m) * ONE / period_m;
    let ty = y_m.rem_euclid(period_m) * ONE / period_m;
    let (sx, sy) = (smooth(0, ONE, tx), smooth(0, ONE, ty));
    let top = hash(ix, iy) + (hash(ix + 1, iy) - hash(ix, iy)) * sx / ONE;
    let bottom = hash(ix, iy + 1) + (hash(ix + 1, iy + 1) - hash(ix, iy + 1)) * sx / ONE;
    top + (bottom - top) * sy / ONE
}

/// Weight (Q12) of the close-zoom smoothing spline: 1 at or below 4 m/px,
/// 0 at or above 12 m/px (logic/04 §atlas-formed close zoom).
pub(super) fn close_zoom_q12(footprint_um: i128) -> i64 {
    ONE - smooth(
        4_000_000,
        12_000_000,
        i64::try_from(footprint_um).unwrap_or(i64::MAX),
    )
}

/// Value noise with its lattice axes broken up: the mean of two copies
/// sampled in domains rotated by 37° and -23° (Q12 cos/sin), so patches do
/// not tile into axis-aligned squares.
fn organic_noise(x_m: i64, y_m: i64, period_m: i64) -> i64 {
    let rot = |c: i64, s: i64| ((x_m * c - y_m * s) / ONE, (x_m * s + y_m * c) / ONE);
    let (ax, ay) = rot(3_271, 2_465); // 37°
    let (bx, by) = rot(3_770, -1_600); // -23°
    (value_noise(ax + 7_919, ay + 3_571, period_m) + value_noise(bx - 1_117, by + 911, period_m))
        / 2
}

fn mix(a: [i64; 3], b: [i64; 3], w: i64) -> [i64; 3] {
    let w = w.clamp(0, ONE);
    std::array::from_fn(|c| a[c] + (b[c] - a[c]) * w / ONE)
}

const fn rgb(hex: u32) -> [i64; 3] {
    [
        ((hex >> 16) & 255) as i64,
        ((hex >> 8) & 255) as i64,
        (hex & 255) as i64,
    ]
}

/// Lambert term minus flat ground for one exaggerated gradient, Q12.
fn relief_term(gx: i64, gy: i64, exaggeration_q8: i64) -> i64 {
    let gx = i128::from(gx * exaggeration_q8 / 256);
    let gy = i128::from(gy * exaggeration_q8 / 256);
    let one = i128::from(ONE);
    let len_sq = gx * gx + gy * gy + one * one;
    let len = i128::try_from(len_sq.unsigned_abs().isqrt()).unwrap_or(i128::MAX);
    let dot = (i128::from(LIGHT_AXIS) * (gx + gy) + i128::from(LIGHT_UP) * one) / len;
    i64::try_from(dot).unwrap_or(0) - LIGHT_UP
}

/// Padé tanh on a Q12 argument, clamped to [-1, 1].
fn soft_q12(x: i64) -> i64 {
    let x = i128::from(x.clamp(-3 * ONE, 3 * ONE));
    let q2 = i128::from(ONE) * i128::from(ONE);
    let v = x * (27 * q2 + x * x) / (27 * q2 + 9 * x * x);
    i64::try_from(v).unwrap_or(0).clamp(-ONE, ONE)
}

/// Shaded land colour for one fine point.
pub(super) fn shade(p: &FormedInputs) -> [u8; 3] {
    if !p.v6 {
        return v5::shade(p);
    }
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
    let low = mix(rgb(0x8c_8a4c), rgb(0x76_7a40), mo);
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
    c = mix(c, rgb(0x6c_7a3c), valley);
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
    let fine_mo = mo + (organic_noise(fx, fy, 900) - ONE / 2) * 2 / 5 * fine_keep / ONE;
    // Canopy follows landform: it thins on convex crests and spurs, so
    // vegetation reads as golden ridges over green hollows rather than
    // random patches (goals 24, 25).
    let crest = smooth(5_000, 50_000, -conc);
    let forest = smooth(90 * ONE / 255, 170 * ONE / 255, fine_mo)
        * smooth(100, 400, i64::from(p.temperature_centi))
        / ONE
        * (ONE - smooth(2_458, 4_096, slope))
        / ONE
        * (ONE - crest * 11 / 20)
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
        let canopy = mix(rgb(0x52_6030), rgb(0x68_7238), mottle);
        c = mix(c, canopy, forest * (2_253 + 1_843 * mottle / ONE) / ONE);
    }
    c = mix(
        c,
        rgb(0x62_703c),
        i64::from(p.wetness) * ONE / 255 / 3 * (ONE - rugged) / ONE,
    );
    c = mix(c, rgb(0xb0_8e4e), smooth(5_000, 60_000, -conc) / 3);
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
    // Surface mottling (logic/04 §atlas-formed materials): rock, scree and
    // turf vary in tone at 250 m and 90 m, ±9% on rock and ±5% on turf.
    // It fades out before its wavelength drops below a few pixels.
    {
        let fp = i64::try_from(p.footprint_um).unwrap_or(i64::MAX);
        let keep = ONE - smooth(40_000_000, 120_000_000, fp);
        if keep > 0 {
            let (mx, my) = p.position_m;
            let n = (organic_noise(mx + 311, my - 97, 250) * 2
                + organic_noise(mx - 57, my + 211, 90))
                / 3
                - ONE / 2;
            let amp = 205 + 164 * rock / ONE; // 5% .. 9% of ONE
            let f = ONE + n * 2 * amp / ONE * keep / ONE;
            c = std::array::from_fn(|ch| c[ch] * f / ONE);
        }
        // Finer 40 m and 20 m tone once pixels resolve it (goal 33).
        let t = super::detail::tone(p.position_dm, p.footprint_um);
        let f = ONE + t * (164 + 123 * rock / ONE) / ONE;
        c = std::array::from_fn(|ch| c[ch] * f / ONE);
    }
    // Shore (goals 16, 31). With a stored shore layer each class paints its
    // own material near the waterline; without one, sand on low, gentle
    // ground touching the sea and bare rock where steep ground meets it.
    let stored: i64 = p.shore[1..].iter().sum();
    if stored > 0 {
        c = shore_land(c, &p.shore, e, slope);
    } else if p.ring_min_mm < 0 {
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

    // Multi-scale light, plus sub-grid runnels once pixels are small enough
    // to draw them (logic/04 §atlas-formed detail).
    let strength = 614 + 2_458 * smooth(410, 2_867, slope) / ONE + 4_915 * rock / ONE;
    let (bx, by, trough) =
        super::detail::relief(p.position_dm, p.footprint_um, p.gradients[1], strength);
    let mut lit = 0;
    for k in 0..4 {
        let (gx, gy) = p.gradients[k];
        let (gx, gy) = if k == 0 { (gx + bx, gy + by) } else { (gx, gy) };
        lit += SCALE_WEIGHT_Q12[k] * relief_term(gx, gy, EXAGGERATION_Q8[k]) / ONE;
    }
    let l = soft_q12(lit * 12 / 5);
    let ao = (ONE - 901 * smooth(0, 120_000, conc) / ONE) * (ONE * 3 + p.sky_q12) / (4 * ONE)
        * (ONE - trough.clamp(0, ONE) / 4)
        / ONE;
    let k = (ONE + 2_867 * l / ONE) * ao / ONE;
    // Snow and ice take sky light: their range is pulled toward flat light,
    // so lit snow keeps gradation and shaded snow reads blue-grey rather
    // than charcoal (goals 20, 24).
    let k = k + (ONE - k) * snow * 2 / 5 / ONE;
    let sw = (-l).max(0) * 1_843 / ONE;
    let shadow: [i64; 3] = std::array::from_fn(|ch| {
        [1_229, 983, 1_065][ch] + ([1_720, 1_880, 2_380][ch] - [1_229, 983, 1_065][ch]) * snow / ONE
    });
    let hl = l.max(0) * 492 / ONE * (ONE - snow / 2) / ONE;
    let hl_tint = [ONE, 3_891, 3_277];
    std::array::from_fn(|ch| {
        let base = c[ch] * k / ONE;
        let tinted = base * (ONE - sw) / ONE + shadow[ch] * sw / ONE * c[ch].max(77) / ONE;
        let v =
            tinted + (hl * hl_tint[ch] / ONE) * 255 / ONE + if ch == 2 { 8 * sw / ONE } else { 0 };
        u8::try_from(shoulder(v).clamp(0, 255)).unwrap_or(255)
    })
}

/// Soft highlight shoulder: values above 190 approach 255 asymptotically,
/// so bright snow, sand and sunlit crests keep gradation instead of
/// clipping to flat white.
fn shoulder(v: i64) -> i64 {
    if v <= 190 {
        v
    } else {
        190 + 65 * (v - 190) / (v - 190 + 65)
    }
}

/// Sea colour stops by depth (mm): a narrow bright coastal band, teal
/// shelf, blue slope and navy abyss (goal 30).
const SEA_STOPS: [(i64, u32); 8] = [
    (0, 0x7c_c6c4),
    (3_000, 0x44_a0b0),
    (15_000, 0x2a_7f9f),
    (60_000, 0x1d_6388),
    (200_000, 0x14_4b72),
    (1_500_000, 0x0d_3a60),
    (4_000_000, 0x0a_3055),
    (6_000_000, 0x08_2849),
];

/// Seafloor colour for one fine point: depth ramp with gentle broad relief
/// so shelf edges and submarine valleys read (logic/04 §atlas-formed sea).
pub(super) fn sea(depth_mm: i64, broad: (i64, i64)) -> [u8; 3] {
    let d = depth_mm.max(0);
    let mut c = rgb(SEA_STOPS[SEA_STOPS.len() - 1].1);
    for pair in SEA_STOPS.windows(2) {
        let (d0, c0) = pair[0];
        let (d1, c1) = pair[1];
        if d < d1 {
            c = mix(rgb(c0), rgb(c1), (d - d0) * ONE / (d1 - d0));
            break;
        }
    }
    let l = soft_q12(relief_term(broad.0, broad.1, 2_560) * 2);
    let k = ONE + 614 * l / ONE;
    std::array::from_fn(|ch| u8::try_from((c[ch] * k / ONE).clamp(0, 255)).unwrap_or(255))
}

/// Stored shore classes on land (logic/04 §atlas-formed shore): `w` are
/// Q12 weights by class, `e` the height (mm) and `slope` the local slope
/// (Q12). Beaches are bright sand or grey shingle on the low strip above
/// the water; cliffs show bare rock on their steep faces; rocky shores a
/// darker rock band; salt marsh a muted olive.
fn shore_land(mut c: [i64; 3], w: &[i64; 8], e: i64, slope: i64) -> [i64; 3] {
    // Class shares among the classified neighbours: the 100 m class grid
    // picks the material, the fine height and slope place it, so the
    // painted band follows the smooth waterline contour.
    let total: i64 = w[1..].iter().sum::<i64>().max(1);
    let low = ONE - smooth(2_000, 7_000, e);
    let band = ONE - smooth(2_000, 12_000, e);
    let steep = smooth(1_229, 2_867, slope);
    let layers: [(usize, u32, i64); 5] = [
        (1, 0xe2_d0a0, low),                           // sand beach
        (2, 0xb4_ab98, low),                           // shingle beach
        (3, 0x8e_8476, steep.max(band / 2)),           // cliff face
        (4, 0x86_8073, band * 2 / 3),                  // rocky shore
        (5, 0x86_8f5c, ONE - smooth(1_500, 5_000, e)), // salt marsh
    ];
    for (class, colour, reach) in layers {
        let k = w[class] * ONE / total * reach / ONE;
        if k > 0 {
            c = mix(c, rgb(colour), k);
        }
    }
    c
}

/// Stored shore classes on water: tidal flats read as wet sand-grey
/// shallows, estuaries as sediment-laden green-teal water.
pub(super) fn shore_water(c: [u8; 3], depth_mm: i64, w: &[i64; 8]) -> [u8; 3] {
    let base = c.map(i64::from);
    let total: i64 = w[1..].iter().sum::<i64>().max(1);
    let shallow = ONE - smooth(500, 4_000, depth_mm);
    let flat = mix(base, rgb(0xa6_a283), w[6] * ONE / total * shallow / ONE);
    let out = mix(flat, rgb(0x4f_8c86), w[7] * ONE / total * 2 / 3);
    out.map(|v| u8::try_from(v.clamp(0, 255)).unwrap_or(255))
}

/// Saline lake water by depth (mm): lighter turquoise than fresh lakes,
/// over pale evaporite shallows (goal 12, recipe 7).
const SALINE_STOPS: [(i64, u32); 5] = [
    (0, 0xa8_e2d6),
    (1_500, 0x78_cfc6),
    (5_000, 0x4f_b3b4),
    (15_000, 0x35_93a2),
    (40_000, 0x26_7890),
];

/// Saline lake colour for one fine point below the saved water surface.
pub(super) fn saline_lake(depth_mm: i64) -> [u8; 3] {
    let d = depth_mm.max(0);
    let mut c = rgb(SALINE_STOPS[SALINE_STOPS.len() - 1].1);
    for pair in SALINE_STOPS.windows(2) {
        let (d0, c0) = pair[0];
        let (d1, c1) = pair[1];
        if d < d1 {
            c = mix(rgb(c0), rgb(c1), (d - d0) * ONE / (d1 - d0));
            break;
        }
    }
    std::array::from_fn(|ch| u8::try_from(c[ch].clamp(0, 255)).unwrap_or(255))
}

/// A dry salt pan over land colour `c`: white evaporite crust (Q12 weight
/// `crust`) and pale clay mudflats (`mudflat`) keep a little of the land's
/// light and shade (goal 12, recipe 7).
pub(super) fn pan(c: [u8; 3], crust: i64, mudflat: i64) -> [u8; 3] {
    let base = c.map(i64::from);
    let out = mix(base, rgb(0xd9_cdb0), mudflat * 13 / 16);
    let out = mix(out, rgb(0xf0_ece2), crust * 7 / 8);
    out.map(|v| u8::try_from(v.clamp(0, 255)).unwrap_or(255))
}

/// Lake water by depth (mm): a lighter shallow margin grading to deep blue
/// (goal 29).
const LAKE_STOPS: [(i64, u32); 6] = [
    (0, 0x74_b8c8),
    (1_500, 0x4a_9cc0),
    (5_000, 0x2d_78ab),
    (15_000, 0x1c_5690),
    (40_000, 0x14_4174),
    (120_000, 0x0f_3462),
];

/// Lake colour for one fine point below the saved water surface; `v6`
/// false keeps the recipe-5 stops ([`v5::lake`]).
pub(super) fn lake(depth_mm: i64, v6: bool) -> [u8; 3] {
    if !v6 {
        return v5::lake(depth_mm);
    }
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

#[cfg(test)]
mod tests;
