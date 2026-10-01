//! Pieces and their tiles: the shared tile vocabulary of the town WFC.
//!
//! A [`Piece`] is a floor, a door, a ground accent or a prop of `w × h`
//! squares with a **need** on each outer side (a wall behind it, open floor
//! in front, a table to sit at). Each allowed orientation and each square
//! of the piece is one tile; the squares of one piece are tied together by
//! links, so a table can only appear whole. Adjacency is derived from the
//! sides: two tiles meet when neither breaks a link and each side's need is
//! met by the edge kind and the tile across it (logic/10 §town-interiors,
//! goal 44).

use arda_wfc::{Dir, Rules, TileSet, WfcError};

/// What one side of a piece asks of its surroundings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Need {
    /// Anything.
    Any,
    /// A wall on this side (a partition or shell indoors, a building
    /// outdoors).
    Wall,
    /// Open, walkable ground across this side (fronts of hearths, ovens,
    /// altars, stalls).
    Walk,
    /// A table across this side (seats).
    Table,
    /// A partition across this side with walkable floor beyond (doorways).
    Door,
    /// Plain ground across this side, no other piece (keeps graves in
    /// rows and arrow loops apart).
    Plain,
}

/// What a piece puts on the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum What {
    /// Nothing but the floor or ground.
    Plain,
    /// A vocabulary prop or vegetation id.
    Prop(&'static str),
    /// A tag query placement.
    Query(&'static [&'static str]),
    /// A ground accent (e.g. `rug`).
    Ground(&'static str),
    /// A doorway on the piece's `Door` side.
    Door,
    /// An arrow loop (a `window` piece) on the piece's `Wall` side.
    Loop,
}

/// A piece of the vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    /// Stable name (unique within a vocabulary).
    pub name: &'static str,
    /// What it draws.
    pub what: What,
    /// Width at orientation 0, squares.
    pub w: i64,
    /// Depth at orientation 0, squares.
    pub h: i64,
    /// Needs on the N, E, S and W sides at orientation 0.
    pub needs: [Need; 4],
    /// Allowed orientations (quarter turns clockwise).
    pub orients: &'static [u8],
    /// Whether it blocks movement.
    pub blocking: bool,
    /// Whether seats may face it.
    pub table: bool,
    /// Light `(radius_ft, colour)` it gives off.
    pub light: Option<(u16, [u8; 3])>,
    /// Base choice weight of each of its tiles.
    pub weight: u32,
}

/// Every orientation.
pub const ALL: &[u8] = &[0, 1, 2, 3];
/// Orientations 0 and 1 (for symmetric pieces).
pub const TWO: &[u8] = &[0, 1];
/// Orientation 0 only.
pub const ONE: &[u8] = &[0];

impl Piece {
    /// A plain piece (floor or ground).
    #[must_use]
    pub const fn plain(name: &'static str, weight: u32) -> Self {
        Self {
            name,
            what: What::Plain,
            w: 1,
            h: 1,
            needs: [Need::Any; 4],
            orients: ONE,
            blocking: false,
            table: false,
            light: None,
            weight,
        }
    }

    /// A blocking prop with needs, all orientations.
    #[must_use]
    pub const fn prop(id: &'static str, w: i64, h: i64, needs: [Need; 4], weight: u32) -> Self {
        Self {
            name: id,
            what: What::Prop(id),
            w,
            h,
            needs,
            orients: ALL,
            blocking: true,
            table: false,
            light: None,
            weight,
        }
    }

    /// The same piece with other orientations.
    #[must_use]
    pub const fn orients(mut self, o: &'static [u8]) -> Self {
        self.orients = o;
        self
    }

    /// The same piece under another name.
    #[must_use]
    pub const fn named(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }

    /// The same piece with a light.
    #[must_use]
    pub const fn lit(mut self, radius_ft: u16, colour: [u8; 3]) -> Self {
        self.light = Some((radius_ft, colour));
        self
    }

    /// The same piece marked as a table.
    #[must_use]
    pub const fn table(mut self) -> Self {
        self.table = true;
        self
    }

    /// The same piece, walkable.
    #[must_use]
    pub const fn walkable(mut self) -> Self {
        self.blocking = false;
        self
    }

    /// The same piece with another weight.
    #[must_use]
    pub const fn weighted(mut self, weight: u32) -> Self {
        self.weight = weight;
        self
    }
}

/// One tile: a square of an oriented piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    /// Index of the piece in the vocabulary.
    pub piece: usize,
    /// Orientation (quarter turns clockwise).
    pub o: u8,
    /// Column within the oriented piece.
    pub px: i64,
    /// Row within the oriented piece.
    pub py: i64,
    /// Oriented width.
    pub rw: i64,
    /// Oriented depth.
    pub rh: i64,
}

impl Tile {
    /// Whether this is the top-left square (the piece's anchor).
    #[must_use]
    pub const fn anchor(&self) -> bool {
        self.px == 0 && self.py == 0
    }
}

/// What one side of a tile is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Tied to this tile of the same piece.
    Link(usize),
    /// An outer side with a need.
    Need(Need),
}

/// How edge kinds read for needs.
#[derive(Debug, Clone, Copy)]
pub struct EdgeModel {
    /// Number of edge kinds.
    pub kinds: usize,
    /// The open kind (same room, same yard).
    pub open: usize,
    /// Kinds that count as a wall for [`Need::Wall`].
    pub walls: &'static [usize],
    /// The partition kind for [`Need::Door`].
    pub partition: Option<usize>,
    /// Kinds only walkable tiles may touch (exterior doors).
    pub walk_only: &'static [usize],
    /// The seam kind: only tiles whose side there is [`Need::Any`].
    pub seam: Option<usize>,
}

/// A vocabulary: pieces, their tiles and the derived rules.
#[derive(Debug, Clone)]
pub struct Vocab {
    /// Pieces.
    pub pieces: Vec<Piece>,
    /// Tiles.
    pub tiles: Vec<Tile>,
    /// Sides of each tile, clockwise from north.
    pub sides: Vec<[Side; 4]>,
    /// Rules.
    pub rules: Rules,
}

impl Vocab {
    /// Builds the tiles and rules of `pieces` under an edge model.
    ///
    /// # Errors
    /// More than 256 tiles.
    pub fn build(pieces: Vec<Piece>, m: &EdgeModel) -> Result<Self, WfcError> {
        let mut tiles = Vec::new();
        for (k, p) in pieces.iter().enumerate() {
            for &o in p.orients {
                let (rw, rh) = if o % 2 == 0 { (p.w, p.h) } else { (p.h, p.w) };
                for py in 0..rh {
                    for px in 0..rw {
                        tiles.push(Tile {
                            piece: k,
                            o,
                            px,
                            py,
                            rw,
                            rh,
                        });
                    }
                }
            }
        }
        let find = |piece: usize, o: u8, x: i64, y: i64| {
            tiles
                .iter()
                .position(|t| t.piece == piece && t.o == o && t.px == x && t.py == y)
        };
        let sides: Vec<[Side; 4]> = tiles
            .iter()
            .map(|t| {
                let p = &pieces[t.piece];
                Dir::ALL.map(|d| {
                    let (dx, dy) = d.step();
                    let (nx, ny) = (t.px + dx, t.py + dy);
                    if nx >= 0 && ny >= 0 && nx < t.rw && ny < t.rh {
                        find(t.piece, t.o, nx, ny).map_or(Side::Need(Need::Any), Side::Link)
                    } else {
                        // The base side that this orientation turns onto `d`.
                        let base = d.turned(4 - usize::from(t.o % 4));
                        Side::Need(p.needs[base.index()])
                    }
                })
            })
            .collect();
        let mut rules = Rules::new(tiles.len(), m.kinds)?;
        let walk: Vec<bool> = tiles.iter().map(|t| !pieces[t.piece].blocking).collect();
        let table: Vec<bool> = tiles.iter().map(|t| pieces[t.piece].table).collect();
        let plain: Vec<bool> = tiles
            .iter()
            .map(|t| pieces[t.piece].what == What::Plain)
            .collect();
        let n = tiles.len();
        for a in 0..n {
            for d in Dir::ALL {
                for k in 0..m.kinds {
                    if !edge_ok(sides[a][d.index()], walk[a], k, m) {
                        rules.forbid_edge(a, d, k);
                    }
                }
            }
        }
        for a in 0..n {
            for d in [Dir::E, Dir::S] {
                for b in 0..n {
                    for k in 0..m.kinds {
                        let (sa, sb) = (sides[a][d.index()], sides[b][d.opposite().index()]);
                        let ok = match (sa, sb) {
                            (Side::Link(x), Side::Link(y)) => x == b && y == a && k == m.open,
                            (Side::Link(_), _) | (_, Side::Link(_)) => false,
                            (Side::Need(na), Side::Need(nb)) => {
                                need_ok(na, (walk[b], table[b], plain[b]), k, m)
                                    && need_ok(nb, (walk[a], table[a], plain[a]), k, m)
                            }
                        };
                        if ok {
                            rules.allow(a, d, b, k);
                        }
                    }
                }
            }
        }
        Ok(Self {
            pieces,
            tiles,
            sides,
            rules,
        })
    }

    /// Index of a piece by name.
    #[must_use]
    pub fn piece(&self, name: &str) -> Option<usize> {
        self.pieces.iter().position(|p| p.name == name)
    }

    /// Every tile of a piece.
    #[must_use]
    pub fn tiles_of(&self, piece: usize) -> TileSet {
        let mut s = TileSet::empty();
        for (i, t) in self.tiles.iter().enumerate() {
            if t.piece == piece {
                s.insert(i);
            }
        }
        s
    }

    /// The anchor tiles of a piece (one per orientation).
    #[must_use]
    pub fn anchors_of(&self, piece: usize) -> TileSet {
        let mut s = TileSet::empty();
        for (i, t) in self.tiles.iter().enumerate() {
            if t.piece == piece && t.anchor() {
                s.insert(i);
            }
        }
        s
    }

    /// Whether a tile can be walked on.
    #[must_use]
    pub fn walkable(&self, t: usize) -> bool {
        self.tiles
            .get(t)
            .is_some_and(|x| !self.pieces[x.piece].blocking)
    }

    /// The piece of a tile.
    #[must_use]
    pub fn piece_of(&self, t: usize) -> Option<&Piece> {
        self.tiles.get(t).map(|x| &self.pieces[x.piece])
    }

    /// Prop rotation of an orientation, degrees.
    #[must_use]
    pub const fn rotation(o: u8) -> u16 {
        90 * (o % 4) as u16
    }
}

fn edge_ok(side: Side, walk: bool, k: usize, m: &EdgeModel) -> bool {
    if m.walk_only.contains(&k) && !walk {
        return false;
    }
    match side {
        Side::Link(_) => k == m.open,
        Side::Need(n) => {
            if m.seam == Some(k) {
                return n == Need::Any;
            }
            match n {
                Need::Any => true,
                Need::Wall => m.walls.contains(&k),
                Need::Walk | Need::Table | Need::Plain => k == m.open,
                Need::Door => m.partition == Some(k),
            }
        }
    }
}

/// Whether a need is met across kind `k` by a tile that is
/// `(walkable, table, plain)`.
fn need_ok(n: Need, other: (bool, bool, bool), k: usize, m: &EdgeModel) -> bool {
    let (walk, table, plain) = other;
    match n {
        Need::Any => true,
        Need::Wall => m.walls.contains(&k),
        Need::Walk => k == m.open && walk,
        Need::Table => k == m.open && table,
        Need::Door => m.partition == Some(k) && walk,
        Need::Plain => k == m.open && plain,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Need::{Any, Table, Wall};

    const M: EdgeModel = EdgeModel {
        kinds: 3,
        open: 0,
        walls: &[1],
        partition: None,
        walk_only: &[2],
        seam: None,
    };

    #[test]
    fn pieces_hold_together_and_meet_their_needs() {
        let v = Vocab::build(
            vec![
                Piece::plain("floor", 10),
                Piece::prop("prop.table", 2, 1, [Any; 4], 1)
                    .orients(TWO)
                    .table(),
                Piece::prop("prop.chair", 1, 1, [Any, Any, Table, Any], 1),
                Piece::prop("prop.bed", 1, 2, [Wall, Any, Any, Any], 1),
            ],
            &M,
        )
        .unwrap();
        // floor 1 + table 2×2 + chair 4 + bed 4×2.
        assert_eq!(v.tiles.len(), 1 + 4 + 4 + 8);
        let t = v.piece("prop.table").unwrap();
        let left = v
            .tiles
            .iter()
            .position(|x| x.piece == t && x.o == 0 && x.px == 0)
            .unwrap();
        let right = v
            .tiles
            .iter()
            .position(|x| x.piece == t && x.o == 0 && x.px == 1)
            .unwrap();
        assert!(v.rules.legal(left, Dir::E, right, 0));
        assert!(!v.rules.legal(left, Dir::E, 0, 0));
        assert!(!v.rules.legal(left, Dir::E, right, 1));
        // A chair at orientation 0 sits north of a table.
        let chair = v.anchors_of(v.piece("prop.chair").unwrap());
        let c0 = chair.first().unwrap();
        assert!(v.rules.legal(c0, Dir::S, left, 0));
        assert!(!v.rules.legal(c0, Dir::S, 0, 0));
        // A bed's head needs a wall; nothing blocking touches a door edge.
        let bed = v.anchors_of(v.piece("prop.bed").unwrap()).first().unwrap();
        assert!(v.rules.edge_ok(bed, Dir::N, 1));
        assert!(!v.rules.edge_ok(bed, Dir::N, 0));
        assert!(!v.rules.edge_ok(bed, Dir::E, 2));
        assert!(v.rules.edge_ok(0, Dir::E, 2));
    }
}
