//! Prop placement shared by both generators: footprints, rotations,
//! against-wall slots and a connectivity guard, so no blocking prop ever
//! cuts a room (or the cavern) in two or stands in a doorway.

use crate::grid::flood;
use crate::rng::Rng;
use arda_tactical::layout::{AssetRef, Placement};

/// A prop or rock to place.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Item {
    /// Asset id.
    pub(crate) id: &'static str,
    /// Footprint before rotation.
    pub(crate) w: u32,
    pub(crate) h: u32,
    /// Blocks movement in either the placeholder or the AI library: the
    /// connectivity guard assumes the worst.
    pub(crate) blocks: bool,
}

pub(crate) const fn item(id: &'static str, w: u32, h: u32, blocks: bool) -> Item {
    Item { id, w, h, blocks }
}

/// A rectangle of squares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rect {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) w: u32,
    pub(crate) h: u32,
}

impl Rect {
    pub(crate) fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }

    pub(crate) fn area(&self) -> u32 {
        self.w * self.h
    }

    pub(crate) fn centre(&self) -> (u32, u32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }
}

/// Which wall of a room a prop backs onto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    N,
    E,
    S,
    W,
}

pub(crate) const SIDES: [Side; 4] = [Side::N, Side::E, Side::S, Side::W];

impl Side {
    /// The clockwise rotation that turns an asset's north (its back) to
    /// this wall.
    pub(crate) fn rotation(self) -> u16 {
        match self {
            Self::N => 0,
            Self::E => 90,
            Self::S => 180,
            Self::W => 270,
        }
    }
}

/// Footprint after a rotation.
pub(crate) fn turned(it: &Item, rot: u16) -> (u32, u32) {
    if rot.is_multiple_of(180) {
        (it.w, it.h)
    } else {
        (it.h, it.w)
    }
}

/// Placement state over the whole map.
pub(crate) struct Dresser {
    w: u32,
    h: u32,
    /// Squares covered by any prop.
    pub(crate) occupied: Vec<bool>,
    /// Squares covered by a blocking prop.
    pub(crate) blocked: Vec<bool>,
    /// Squares that must stay free: doorways, stair feet.
    pub(crate) keep: Vec<bool>,
    /// Water squares: walkable (wading or swimming) for the connectivity
    /// guard, but nothing is placed on them.
    pub(crate) wet: Vec<bool>,
    pub(crate) placements: Vec<Placement>,
}

impl Dresser {
    pub(crate) fn new(w: u32, h: u32) -> Self {
        let n = w as usize * h as usize;
        Self {
            w,
            h,
            occupied: vec![false; n],
            blocked: vec![false; n],
            keep: vec![false; n],
            wet: vec![false; n],
            placements: Vec::new(),
        }
    }

    fn idx(&self, x: u32, y: u32) -> usize {
        y as usize * self.w as usize + x as usize
    }

    /// Whether a prop covers `(x, y)` or it must stay free.
    pub(crate) fn is_used(&self, x: u32, y: u32) -> bool {
        x >= self.w || y >= self.h || {
            let i = self.idx(x, y);
            self.occupied[i] || self.keep[i] || self.wet[i]
        }
    }

    pub(crate) fn keep(&mut self, x: u32, y: u32) {
        if x < self.w && y < self.h {
            let i = self.idx(x, y);
            self.keep[i] = true;
        }
    }

    /// Places `it` with its footprint's top-left at `(x0, y0)` turned by
    /// `rot`, when every covered square is in `region`, free and not kept,
    /// and (for a blocking prop) the region's open squares stay connected.
    /// `offset` nudges the drawn position (a sconce hugs its wall).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn place(
        &mut self,
        it: &Item,
        x0: u32,
        y0: u32,
        rot: u16,
        region: &dyn Fn(u32, u32) -> bool,
        offset: (f32, f32),
    ) -> bool {
        let (fw, fh) = turned(it, rot);
        if x0 + fw > self.w || y0 + fh > self.h {
            return false;
        }
        let cells: Vec<(u32, u32)> = (y0..y0 + fh)
            .flat_map(|y| (x0..x0 + fw).map(move |x| (x, y)))
            .collect();
        if cells.iter().any(|&(x, y)| {
            let i = self.idx(x, y);
            !region(x, y) || self.occupied[i] || self.keep[i] || self.wet[i]
        }) {
            return false;
        }
        if it.blocks {
            for &(x, y) in &cells {
                let i = self.idx(x, y);
                self.blocked[i] = true;
            }
            if !self.connected(region) {
                for &(x, y) in &cells {
                    let i = self.idx(x, y);
                    self.blocked[i] = false;
                }
                return false;
            }
        }
        for &(x, y) in &cells {
            let i = self.idx(x, y);
            self.occupied[i] = true;
        }
        #[allow(clippy::cast_precision_loss)] // map sides are at most 160
        let (cx, cy) = (
            x0 as f32 + fw as f32 / 2.0 + offset.0,
            y0 as f32 + fh as f32 / 2.0 + offset.1,
        );
        self.placements.push(Placement {
            asset: AssetRef::Id(it.id.to_string()),
            x: cx,
            y: cy,
            rotation: rot,
            mirror: false,
        });
        true
    }

    /// Whether the unblocked squares of `region` form one piece.
    fn connected(&self, region: &dyn Fn(u32, u32) -> bool) -> bool {
        let open = |x: u32, y: u32| region(x, y) && !self.blocked[self.idx(x, y)];
        let mut first = None;
        let mut total = 0usize;
        for y in 0..self.h {
            for x in 0..self.w {
                if open(x, y) {
                    total += 1;
                    first.get_or_insert((x, y));
                }
            }
        }
        let Some(start) = first else { return true };
        flood(self.w, self.h, start, open)
            .iter()
            .filter(|s| **s)
            .count()
            == total
    }

    /// Tries `tries` random slots against the walls of `room`.
    pub(crate) fn on_wall(
        &mut self,
        rng: &mut Rng,
        it: &Item,
        room: Rect,
        region: &dyn Fn(u32, u32) -> bool,
        tries: u32,
    ) -> bool {
        for _ in 0..tries {
            let side = SIDES[rng.index(4)];
            if self.at_wall(rng, it, room, side, region) {
                return true;
            }
        }
        false
    }

    /// One random slot against wall `side` of `room`.
    pub(crate) fn at_wall(
        &mut self,
        rng: &mut Rng,
        it: &Item,
        room: Rect,
        side: Side,
        region: &dyn Fn(u32, u32) -> bool,
    ) -> bool {
        let rot = side.rotation();
        let (fw, fh) = turned(it, rot);
        if fw > room.w || fh > room.h {
            return false;
        }
        let (x0, y0) = match side {
            Side::N => (room.x + rng.range(0, room.w - fw), room.y),
            Side::S => (room.x + rng.range(0, room.w - fw), room.y + room.h - fh),
            Side::E => (room.x + room.w - fw, room.y + rng.range(0, room.h - fh)),
            Side::W => (room.x, room.y + rng.range(0, room.h - fh)),
        };
        self.place(it, x0, y0, rot, region, (0.0, 0.0))
    }

    /// Tries `tries` random free-standing positions inside `room` shrunk by
    /// `margin`, in a random quarter turn.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn free(
        &mut self,
        rng: &mut Rng,
        it: &Item,
        room: Rect,
        margin: u32,
        region: &dyn Fn(u32, u32) -> bool,
        tries: u32,
    ) -> bool {
        for _ in 0..tries {
            let rot = [0u16, 90, 180, 270][rng.index(4)];
            let (fw, fh) = turned(it, rot);
            if room.w < fw + 2 * margin || room.h < fh + 2 * margin {
                continue;
            }
            let x0 = room.x + margin + rng.range(0, room.w - fw - 2 * margin);
            let y0 = room.y + margin + rng.range(0, room.h - fh - 2 * margin);
            if self.place(it, x0, y0, rot, region, (0.0, 0.0)) {
                return true;
            }
        }
        false
    }
}
