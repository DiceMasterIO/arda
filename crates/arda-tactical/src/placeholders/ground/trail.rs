//! A trodden trail or footpath: compacted pale earth, scuffed and dusty,
//! with grit, a few pebbles and the odd grass tuft surviving between feet.

use super::kit::{anywhere, base, dabs, pebbles, strokes, tufts, Palette};
use super::Tex;
use crate::placeholders::brush::Tile;

const TRODDEN: Palette = [[118, 98, 72], [148, 126, 94], [176, 156, 120]];
const GRIT: Palette = [[116, 108, 96], [150, 140, 122], [184, 174, 152]];
const TUFTS: Palette = [[70, 92, 40], [96, 122, 52], [128, 150, 70]];

/// Trail: trodden earth, lighter than dirt and finer grained.
pub fn trail(t: &Tex) -> Tile {
    let mut tile = base(t, &TRODDEN, 3);
    let mut rng = t.rng(0x7A11);
    dabs(
        &mut tile,
        t,
        &mut rng,
        90,
        (5.0, 14.0),
        &TRODDEN,
        (0.2, 0.4),
        &anywhere,
    );
    strokes(
        &mut tile,
        t,
        &mut rng,
        300,
        (3.0, 9.0),
        (0.5, 1.0),
        &TRODDEN,
        (0.25, 0.5),
    );
    pebbles(&mut tile, t, &mut rng, 60, (0.8, 2.2), &GRIT, &anywhere);
    tufts(
        &mut tile,
        t,
        &mut rng,
        6,
        (4, 8),
        (3.0, 7.0),
        0.9,
        &TUFTS,
        (0.6, 1.0),
        &anywhere,
    );
    tile
}
