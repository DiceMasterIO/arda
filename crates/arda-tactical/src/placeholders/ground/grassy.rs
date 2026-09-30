//! Grassy and wet vegetated ground: grass, meadow, pasture, scrub, heath,
//! moss, marsh and reed beds.

use super::kit::{anywhere, base, dabs, flowers, strokes, tufts, Palette};
use super::Tex;
use crate::placeholders::brush::Tile;
use crate::placeholders::paint::tone;

const GRASS: Palette = [[54, 76, 32], [96, 118, 48], [150, 156, 72]];
const DRY: Palette = [[104, 96, 56], [146, 136, 78], [190, 176, 114]];
const SOIL: Palette = [[86, 68, 46], [118, 96, 66], [150, 128, 92]];

/// Grass: olive patches, grass tufts in three tones, a few tiny flowers.
pub fn grass(t: &Tex) -> Tile {
    let mut tile = base(t, &GRASS, 3);
    let mut rng = t.rng(0x6A55);
    let lush = |x: f32, y: f32| t.noise(9, x, y, 4, 2) > 0.45;
    dabs(
        &mut tile,
        t,
        &mut rng,
        90,
        (6.0, 14.0),
        &GRASS,
        (0.2, 0.4),
        &anywhere,
    );
    dabs(
        &mut tile,
        t,
        &mut rng,
        12,
        (4.0, 8.0),
        &SOIL,
        (0.25, 0.45),
        &|x, y| !lush(x, y),
    );
    tufts(
        &mut tile,
        t,
        &mut rng,
        420,
        (4, 7),
        (4.0, 9.0),
        1.3,
        &GRASS,
        (0.0, 0.4),
        &anywhere,
    );
    tufts(
        &mut tile,
        t,
        &mut rng,
        360,
        (3, 6),
        (3.0, 7.0),
        1.1,
        &GRASS,
        (0.3, 0.75),
        &anywhere,
    );
    tufts(
        &mut tile,
        t,
        &mut rng,
        200,
        (2, 4),
        (2.0, 5.0),
        0.9,
        &GRASS,
        (0.7, 1.0),
        &lush,
    );
    flowers(
        &mut tile,
        t,
        &mut rng,
        10,
        &[[232, 228, 206], [232, 206, 90]],
        1.3,
    );
    tile
}

/// Meadow: taller, sunnier grass thick with wildflowers.
pub fn meadow(t: &Tex) -> Tile {
    const P: Palette = [[70, 96, 38], [122, 142, 58], [180, 182, 94]];
    let mut tile = base(t, &P, 3);
    let mut rng = t.rng(0x3EAD);
    dabs(
        &mut tile,
        t,
        &mut rng,
        80,
        (7.0, 15.0),
        &P,
        (0.2, 0.4),
        &anywhere,
    );
    tufts(
        &mut tile,
        t,
        &mut rng,
        380,
        (5, 8),
        (6.0, 12.0),
        1.3,
        &P,
        (0.0, 0.5),
        &anywhere,
    );
    tufts(
        &mut tile,
        t,
        &mut rng,
        300,
        (4, 7),
        (5.0, 10.0),
        1.1,
        &P,
        (0.45, 1.0),
        &anywhere,
    );
    flowers(
        &mut tile,
        t,
        &mut rng,
        70,
        &[
            [236, 232, 214],
            [240, 206, 72],
            [168, 120, 190],
            [224, 128, 140],
            [236, 232, 214],
        ],
        1.6,
    );
    tile
}

/// Pasture: short, even, grazed grass with clover and small bare spots.
pub fn pasture(t: &Tex) -> Tile {
    const P: Palette = [[70, 98, 42], [106, 130, 56], [148, 162, 82]];
    let mut tile = base(t, &P, 4);
    let mut rng = t.rng(0x9A57);
    dabs(
        &mut tile,
        t,
        &mut rng,
        120,
        (4.0, 10.0),
        &P,
        (0.2, 0.45),
        &anywhere,
    );
    dabs(
        &mut tile,
        t,
        &mut rng,
        6,
        (3.0, 7.0),
        &SOIL,
        (0.35, 0.6),
        &anywhere,
    );
    tufts(
        &mut tile,
        t,
        &mut rng,
        700,
        (3, 5),
        (2.0, 4.5),
        1.0,
        &P,
        (0.0, 1.0),
        &anywhere,
    );
    dabs(
        &mut tile,
        t,
        &mut rng,
        50,
        (1.5, 2.6),
        &[[86, 124, 60], [112, 146, 70], [130, 160, 84]],
        (0.8, 1.0),
        &anywhere,
    );
    flowers(&mut tile, t, &mut rng, 16, &[[238, 236, 222]], 1.1);
    tile
}

/// Scrub: dry grass over bare soil with low dark bushes.
pub fn scrub(t: &Tex) -> Tile {
    let mut tile = base(t, &DRY, 3);
    let mut rng = t.rng(0x5C2B);
    dabs(
        &mut tile,
        t,
        &mut rng,
        40,
        (6.0, 13.0),
        &SOIL,
        (0.3, 0.55),
        &anywhere,
    );
    tufts(
        &mut tile,
        t,
        &mut rng,
        200,
        (4, 7),
        (4.0, 9.0),
        1.1,
        &DRY,
        (0.0, 1.0),
        &anywhere,
    );
    const BUSH: Palette = [[44, 58, 32], [68, 84, 44], [102, 116, 60]];
    let bushy = |x: f32, y: f32| t.noise(11, x, y, 4, 2) > 0.52;
    for _ in 0..t.n(26) {
        let c = super::kit::spot_where(t, &mut rng, &bushy);
        for (r, tn) in [(7.0, 0.0), (5.0, 0.5), (2.5, 1.0)] {
            let col = super::kit::ramp(&BUSH, tn);
            let o = (c.0 - r * 0.2 * t.k, c.1 - r * 0.2 * t.k);
            tile.dab(o, r * t.k * rng.range(0.8, 1.2), col, 0.95, rng.next_u64());
        }
    }
    tufts(
        &mut tile,
        t,
        &mut rng,
        60,
        (3, 5),
        (3.0, 6.0),
        0.9,
        &DRY,
        (0.6, 1.0),
        &anywhere,
    );
    tile
}

/// Heath: peaty ground covered in low heather and wiry grass.
pub fn heath(t: &Tex) -> Tile {
    const P: Palette = [[66, 58, 42], [98, 88, 58], [132, 120, 80]];
    const HEATHER: Palette = [[92, 56, 84], [138, 86, 124], [182, 128, 164]];
    const WIRY: Palette = [[70, 84, 44], [104, 112, 62], [140, 142, 86]];
    let mut tile = base(t, &P, 3);
    let mut rng = t.rng(0x4EA7);
    tufts(
        &mut tile,
        t,
        &mut rng,
        160,
        (4, 6),
        (3.0, 7.0),
        1.0,
        &WIRY,
        (0.0, 1.0),
        &anywhere,
    );
    let bloom = |x: f32, y: f32| t.noise(13, x, y, 5, 2) > 0.46;
    for _ in 0..t.n(170) {
        let c = super::kit::spot_where(t, &mut rng, &bloom);
        for i in 0..5 {
            let (dx, dy) = rng.dir();
            let r = rng.range(1.2, 2.4) * t.k;
            let p = (c.0 + dx * 3.0 * t.k, c.1 + dy * 3.0 * t.k);
            let col =
                super::kit::shade_of(&HEATHER, &mut rng, 0.1 * i as f32, 0.5 + 0.1 * i as f32);
            tile.dab(p, r, col, 0.95, rng.next_u64());
        }
    }
    tile
}

/// Moss: lumpy cushions in rich greens with bright tips.
pub fn moss(t: &Tex) -> Tile {
    const P: Palette = [[44, 70, 26], [82, 116, 38], [140, 170, 66]];
    let mut tile = base(t, &P, 4);
    let mut rng = t.rng(0x3055);
    for pass in 0..3 {
        let lo = pass as f32 * 0.3;
        for _ in 0..t.n(260) {
            let c = super::kit::spot(t, &mut rng);
            let r = rng.range(3.0, 7.0) * t.k * (1.0 - 0.25 * pass as f32);
            let col = super::kit::shade_of(&P, &mut rng, lo, lo + 0.4);
            tile.dab(c, r, col, 0.9, rng.next_u64());
            let hi = (c.0 - r * 0.3, c.1 - r * 0.3);
            tile.dab(hi, r * 0.45, tone(col, 1.16), 0.6, rng.next_u64());
        }
    }
    dabs(
        &mut tile,
        t,
        &mut rng,
        80,
        (0.8, 1.4),
        &[[170, 190, 90], [190, 200, 110], [150, 176, 80]],
        (0.9, 1.0),
        &anywhere,
    );
    tile
}

/// Marsh: sodden green-brown ground, dark pools and tussocks of rushes.
pub fn marsh(t: &Tex) -> Tile {
    const P: Palette = [[62, 70, 40], [88, 96, 56], [118, 124, 78]];
    const RUSH: Palette = [[62, 84, 40], [100, 118, 56], [146, 150, 84]];
    const MUD: Palette = [[56, 52, 38], [74, 68, 48], [92, 86, 60]];
    let mut tile = base(t, &P, 3);
    let mut rng = t.rng(0x3A25);
    // Sodden patches: darker, glossier mud between the tussocks.
    let wet = |x: f32, y: f32| t.noise(21, x, y, 5, 3) > 0.56;
    dabs(
        &mut tile,
        t,
        &mut rng,
        90,
        (3.0, 8.0),
        &MUD,
        (0.35, 0.6),
        &wet,
    );
    for _ in 0..t.n(50) {
        let c = super::kit::spot_where(t, &mut rng, &wet);
        let l = rng.range(2.0, 5.0) * t.k;
        tile.stroke(
            c,
            (c.0 + l, c.1 - l * 0.3),
            0.6 * t.k,
            0.3 * t.k,
            [132, 144, 128],
            0.45,
        );
    }
    tufts(
        &mut tile,
        t,
        &mut rng,
        320,
        (5, 9),
        (5.0, 11.0),
        1.1,
        &RUSH,
        (0.0, 1.0),
        &anywhere,
    );
    tile
}

/// Reed bed: a dense stand of reeds over dark water.
pub fn reed_bed(t: &Tex) -> Tile {
    const P: Palette = [[34, 48, 42], [52, 68, 54], [76, 92, 70]];
    const REED: Palette = [[74, 92, 42], [124, 134, 64], [180, 172, 104]];
    let mut tile = base(t, &P, 3);
    let mut rng = t.rng(0x2EED);
    tufts(
        &mut tile,
        t,
        &mut rng,
        170,
        (7, 12),
        (9.0, 18.0),
        1.2,
        &REED,
        (0.0, 0.6),
        &anywhere,
    );
    tufts(
        &mut tile,
        t,
        &mut rng,
        120,
        (5, 9),
        (8.0, 15.0),
        1.0,
        &REED,
        (0.4, 1.0),
        &anywhere,
    );
    strokes(
        &mut tile,
        t,
        &mut rng,
        30,
        (2.0, 4.0),
        (1.3, 1.9),
        &[[92, 60, 34], [112, 74, 42], [130, 90, 54]],
        (0.9, 1.0),
    );
    tile
}
