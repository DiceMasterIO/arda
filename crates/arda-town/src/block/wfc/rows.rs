//! Ordered furniture by rule, before the pass-B WFC fills around it.
//!
//! Some furniture reads wrong when a solver scatters it: pews in a nave,
//! beds in a dormitory, racks in a warehouse, bookshelves in a reading
//! room and stall tables in a market hall stand in aligned rows. A zone
//! item with an [`Order`] is laid out here first (logic/10 §town-wfc):
//! its squares are fixed in the WFC domains, the piece is taken out of the
//! rest of the room, and the anchors and limits that asked for it are
//! adjusted, so the WFC only furnishes around the rows.

use super::furnish::{Frame, OPEN, PART, SHELL};
use super::piece::{Need, Side, Vocab};
use super::programme::Programme;
use arda_wfc::{Anchor, Dir, Limit, TileSet};

/// How an ordered piece is laid out in its room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// Rows across the room facing its back, either side of a clear
    /// central aisle, with legroom between rows (pews).
    Pews,
    /// Along both long walls, backs to the wall, `gap` squares apart,
    /// short of the corners so the end walls stay free for a hearth or a
    /// door (dormitory beds, library bookshelves).
    Walls {
        /// Free squares between neighbours along the wall.
        gap: i64,
    },
    /// Free-standing rows along the room with a walk between rows and
    /// `gap` squares between pieces in a row (warehouse racks, stall
    /// tables).
    Rows {
        /// Free squares between neighbours in a row.
        gap: i64,
    },
}

/// Ordered placements of a furnished room.
#[derive(Debug, Clone, Default)]
pub struct Fixed {
    /// `(cell, tile)` pairs fixed before the WFC.
    pub cells: Vec<(usize, usize)>,
    /// `(room, piece)` groups laid out; the piece stays out of the rest of
    /// the room.
    pub groups: Vec<(usize, usize)>,
    /// Squares kept walkable: the fronts of fixed pieces and the passages
    /// at the ends of free-standing rows.
    pub open: Vec<usize>,
}

/// One candidate: anchor square and orientation.
type Spot = (i64, i64, u8);

/// A tried layout: its score (pieces placed, rows across first), its `(cell, tile)` squares and the
/// squares taken after it.
type Trial = (usize, Squares, Vec<u8>);

/// `(cell, tile)` squares of fixed pieces.
type Squares = Vec<(usize, usize)>;

fn dims(v: &Vocab, p: usize, o: u8) -> (i64, i64) {
    let piece = &v.pieces[p];
    if o.is_multiple_of(2) {
        (piece.w, piece.h)
    } else {
        (piece.h, piece.w)
    }
}

fn tile(v: &Vocab, p: usize, o: u8, px: i64, py: i64) -> Option<usize> {
    v.tiles
        .iter()
        .position(|t| t.piece == p && t.o == o && t.px == px && t.py == py)
}

/// The candidate layouts of `order` in `rect`, each a list of spots.
fn spots(order: Order, v: &Vocab, p: usize, rect: [i64; 4], reserve: i64) -> Vec<Vec<Spot>> {
    let [x0, y0, x1, y1] = rect;
    let (w, d) = (x1 - x0, y1 - y0);
    match order {
        Order::Pews => {
            let (pw, _) = dims(v, p, 0);
            // A central aisle of one square (odd widths) or two (even).
            let (al, ar) = if w >= 5 {
                let c = x0 + (w - 1) / 2;
                (c, if w % 2 == 0 { c + 2 } else { c + 1 })
            } else {
                (x0 + pw, x1)
            };
            let mut cols = Vec::new();
            let mut x = al - pw;
            while x >= x0 {
                cols.push(x);
                x -= pw;
            }
            let mut x = ar;
            while x + pw <= x1 {
                cols.push(x);
                x += pw;
            }
            cols.sort_unstable();
            let mut rows = Vec::new();
            let mut y = y0 + 1;
            while y + 1 < y1 - reserve {
                rows.extend(cols.iter().map(|&x| (x, y, 0)));
                y += 2;
            }
            vec![rows]
        }
        Order::Walls { gap } => {
            let long_x = w >= d;
            let sides: [u8; 2] = if long_x { [0, 2] } else { [3, 1] };
            let runs = sides
                .iter()
                .filter(|o| v.pieces[p].orients.contains(o))
                .flat_map(|&o| {
                    let (rw, rh) = dims(v, p, o);
                    let mut run = Vec::new();
                    if long_x {
                        let y = if o == 0 { y0 } else { y1 - rh };
                        let mut x = x0 + 1;
                        while x + rw < x1 {
                            run.push((x, y, o));
                            x += rw + gap;
                        }
                    } else {
                        let x = if o == 3 { x0 } else { x1 - rw };
                        let mut y = y0 + 1;
                        while y + rh < y1 {
                            run.push((x, y, o));
                            y += rh + gap;
                        }
                    }
                    run
                })
                .collect();
            vec![runs]
        }
        Order::Rows { gap } => {
            // Every orientation and offset: one square of passage at each
            // end, a walk before each row. The best fit wins in `place`.
            let mut all = Vec::new();
            for o in [0_u8, 1]
                .into_iter()
                .filter(|o| v.pieces[p].orients.contains(o))
            {
                let (rw, rh) = dims(v, p, o);
                for (dx, dy) in [(1, 1), (2, 1), (1, 2), (2, 2)] {
                    let mut spots = Vec::new();
                    let mut y = y0 + dy;
                    while y + rh <= y1 {
                        let mut x = x0 + dx;
                        while x + rw < x1 {
                            spots.push((x, y, o));
                            x += rw + gap;
                        }
                        y += rh + 1;
                    }
                    all.push(spots);
                }
            }
            all
        }
    }
}

struct Ctx<'a, 'b> {
    v: &'a Vocab,
    f: &'a Frame<'b>,
    room: usize,
    /// Squares the circulation keeps open, doorways and entrances.
    blocked: &'a [bool],
    /// Squares already fixed ([`TAKEN`]) or kept walkable for a fixed
    /// piece's front ([`FRONT`]).
    taken: Vec<u8>,
}

/// A square a fixed piece stands on.
const TAKEN: u8 = 1;
/// A square a fixed piece must be worked from.
const FRONT: u8 = 2;

impl Ctx<'_, '_> {
    /// Keeps the passages at both ends of free-standing rows walkable: the
    /// end columns of rows running across the room, the end rows of rows
    /// running down it, so every walk between rows opens onto them.
    fn passages(&mut self, rect: [i64; 4], across: bool) {
        let [x0, y0, x1, y1] = rect;
        let ends: Vec<(i64, i64)> = if across {
            (y0..y1).flat_map(|y| [(x0, y), (x1 - 1, y)]).collect()
        } else {
            (x0..x1).flat_map(|x| [(x, y0), (x, y1 - 1)]).collect()
        };
        for (x, y) in ends {
            if let Some(i) = self.f.idx(x, y) {
                if self.taken[i] == 0 && self.f.layout.cell_room[i] == self.room {
                    self.taken[i] = FRONT;
                }
            }
        }
    }

    /// Fixes `spot` when it fits; returns its squares and tiles.
    fn put(&mut self, p: usize, spot: Spot) -> Option<Squares> {
        let (sq, fronts) = self.fit(p, spot)?;
        for &(i, _) in &sq {
            self.taken[i] = TAKEN;
        }
        for j in fronts {
            self.taken[j] = FRONT;
        }
        Some(sq)
    }

    /// The squares and tiles of `spot` and the squares its front needs,
    /// when it fits.
    fn fit(&self, p: usize, spot: Spot) -> Option<(Squares, Vec<usize>)> {
        let (ax, ay, o) = spot;
        let (rw, rh) = dims(self.v, p, o);
        let lay = self.f.layout;
        let mut out = Vec::new();
        let mut fronts = Vec::new();
        for py in 0..rh {
            for px in 0..rw {
                let (x, y) = (ax + px, ay + py);
                let i = self.f.idx(x, y)?;
                if lay.cell_room[i] != self.room || self.blocked[i] || self.taken[i] != 0 {
                    return None;
                }
                let t = tile(self.v, p, o, px, py)?;
                for d in Dir::ALL {
                    let Side::Need(need) = self.v.sides[t][d.index()] else {
                        continue;
                    };
                    let kind = self.f.kind(x, y, d);
                    let ok = match need {
                        Need::Wall => kind == PART || kind == SHELL,
                        Need::Walk => {
                            let (dx, dy) = d.step();
                            let front = self.f.idx(x + dx, y + dy);
                            if let Some(j) = front {
                                fronts.push(j);
                            }
                            kind == OPEN && front.is_some_and(|j| self.taken[j] != TAKEN)
                        }
                        Need::Any => true,
                        Need::Table | Need::Door | Need::Plain => false,
                    };
                    if !ok {
                        return None;
                    }
                }
                out.push((i, t));
            }
        }
        Some((out, fronts))
    }
}

/// Lays out every ordered item of `prog` in its rooms. `blocked` marks
/// the squares that must stay open (circulation, doorways, entrances).
#[must_use]
pub fn place(prog: &Programme, v: &Vocab, f: &Frame<'_>, blocked: &[bool]) -> Fixed {
    let n = usize::try_from(f.w * f.d).unwrap_or(0);
    let mut fixed = Fixed::default();
    let mut taken = vec![0_u8; n];
    for (r, room) in f.layout.rooms.iter().enumerate() {
        let Some(z) = prog.zones.get(room.zone) else {
            continue;
        };
        // Rooms that must still seat a wall piece with a walk before it
        // (an altar in a shrine's nave) keep their back rows free.
        let reserve = i64::from(
            z.items
                .iter()
                .any(|it| it.order.is_none() && it.need > 0 && it.piece.blocking),
        );
        for it in &z.items {
            let (Some(order), Some(p)) = (it.order, v.piece(it.piece.name)) else {
                continue;
            };
            let mut best: Option<Trial> = None;
            for spots in spots(order, v, p, room.rect, reserve) {
                let mut ctx = Ctx {
                    v,
                    f,
                    room: r,
                    blocked,
                    taken: taken.clone(),
                };
                let (mut placed, mut cells) = (0, Vec::new());
                let mut across = None;
                for spot in spots {
                    if let Some(sq) = ctx.put(p, spot) {
                        across = Some(spot.2 % 2 == 0);
                        cells.extend(sq);
                        placed += 1;
                    }
                }
                if let (Order::Rows { .. }, Some(across)) = (order, across) {
                    ctx.passages(room.rect, across);
                }
                // Rows across the room (orientation 0) read best; turned
                // rows only when none fit across.
                let score = placed + usize::from(placed > 0 && across == Some(true)) * 10_000;
                if best.as_ref().is_none_or(|b| score > b.0) {
                    best = Some((score, cells, ctx.taken));
                }
            }
            let Some((score, cells, now)) = best else {
                continue;
            };
            taken = now;
            fixed.cells.extend(cells);
            if score > 0 {
                fixed.groups.push((r, p));
            }
        }
    }
    fixed.open = (0..n).filter(|&i| taken[i] == FRONT).collect();
    fixed
}

/// Applies `fixed` to the pass-B problem: fixed squares take their tile,
/// the rest of each group's room loses the piece, anchors count what the
/// rows already hold and limits make room for them.
pub fn apply(
    fixed: &Fixed,
    v: &Vocab,
    f: &Frame<'_>,
    domains: &mut [TileSet],
    anchors: &mut Vec<Anchor>,
    limits: &mut [Limit],
) {
    if fixed.cells.is_empty() {
        return;
    }
    let mut is_fixed = vec![false; domains.len()];
    for &(i, t) in &fixed.cells {
        if let Some(d) = domains.get_mut(i) {
            *d = TileSet::single(t);
            is_fixed[i] = true;
        }
    }
    let walk = (0..v.tiles.len())
        .filter(|&t| v.walkable(t))
        .fold(TileSet::empty(), |mut s, t| {
            s.insert(t);
            s
        });
    for &i in &fixed.open {
        if let Some(d) = domains.get_mut(i) {
            if !is_fixed[i] && d.meets(&walk) {
                *d = d.and(walk);
            }
        }
    }
    for &(r, p) in &fixed.groups {
        let tiles = v.tiles_of(p);
        for (i, d) in domains.iter_mut().enumerate() {
            if !is_fixed[i] && f.layout.cell_room.get(i) == Some(&r) {
                *d = d.minus(tiles);
            }
        }
    }
    for a in anchors.iter_mut() {
        let held = a
            .options
            .iter()
            .filter(|(i, ts)| is_fixed[*i] && ts.meets(&domains[*i]))
            .count();
        a.count = a
            .count
            .saturating_sub(u32::try_from(held).unwrap_or(u32::MAX));
        a.options
            .retain(|(i, ts)| !is_fixed[*i] && ts.meets(&domains[*i]));
    }
    anchors.retain(|a| a.count > 0);
    let heads: Vec<TileSet> = fixed.groups.iter().map(|&(_, p)| v.anchors_of(p)).collect();
    for l in limits.iter_mut() {
        let forced = fixed
            .cells
            .iter()
            .filter(|&&(i, t)| l.tiles.contains(t) && (l.cells.is_empty() || l.cells.contains(&i)))
            .count();
        let forced = u32::try_from(forced).unwrap_or(u32::MAX);
        if forced == 0 {
            continue;
        }
        // A piece's own cap grows to its rows; a room's furniture budget
        // keeps a little room for the rest.
        let slack = if heads.contains(&l.tiles) { 0 } else { 6 };
        l.max = l.max.max(forced.saturating_add(slack));
    }
}

/// A room's rectangle and the anchors `(x, y, orientation)` of one piece
/// in it, canonical squares.
pub type Laid = ([i64; 4], Vec<(i64, i64, u8)>);

/// Where piece `name` stands in the rooms tagged `tag` of a solved
/// building (for review and tests).
#[must_use]
pub fn laid_out(p: &super::indoor::Plan<'_>, tag: &str, name: &str) -> Vec<Laid> {
    let (v, f) = &p.furnished;
    let w = p
        .layout
        .rooms
        .iter()
        .map(|r| r.rect[2])
        .max()
        .unwrap_or(1)
        .max(1);
    p.layout
        .rooms
        .iter()
        .enumerate()
        .filter(|(_, r)| p.prog.zones.get(r.zone).is_some_and(|z| z.tag == tag))
        .map(|(ri, r)| {
            let mut out = Vec::new();
            for (i, &t) in f.tiles.iter().enumerate() {
                let Some(tile) = v.tiles.get(usize::from(t)) else {
                    continue;
                };
                if tile.anchor()
                    && v.pieces[tile.piece].name == name
                    && p.layout.cell_room.get(i) == Some(&ri)
                {
                    let i = i64::try_from(i).unwrap_or(0);
                    out.push((i % w, i / w, tile.o));
                }
            }
            (r.rect, out)
        })
        .collect()
}

/// Whether pieces stand in rows: every anchor shares a row or a column
/// with another (for review and tests).
#[must_use]
pub fn lined_up(pieces: &[(i64, i64, u8)]) -> bool {
    pieces
        .iter()
        .all(|a| pieces.iter().any(|b| b != a && (b.0 == a.0 || b.1 == a.1)))
}
