//! Pass B: doorways and furniture by WFC over the zone's typed pieces.
//!
//! Walls are fixed: the shell and exterior doors from the plan footprint,
//! the partitions from pass A. Edge kinds tell the pieces what stands on
//! each side (open floor, partition, shell, exterior door). Doorways are
//! anchored first, one per edge of the room tree, at a position the WFC
//! draws; then each room's required pieces; then the free fill under the
//! room's caps. The finished grid must leave every walkable square
//! reachable from the exterior doors and every furniture square beside a
//! reachable one (goal 44: interiors usable as battle maps).

use super::catalogue::{DOOR, FLOOR};
use super::piece::{EdgeModel, Piece, Vocab, What};
use super::programme::Programme;
use super::rooms::Layout;
use arda_wfc::{solve, Anchor, Dir, Failure, Limit, Problem, TileSet};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Same room.
pub const OPEN: u8 = 0;
/// A partition between rooms.
pub const PART: u8 = 1;
/// The building's shell.
pub const SHELL: u8 = 2;
/// An exterior door in the shell.
pub const EXT: u8 = 3;

/// The indoor edge model.
pub const MODEL: EdgeModel = EdgeModel {
    kinds: 4,
    open: OPEN as usize,
    walls: &[PART as usize, SHELL as usize],
    partition: Some(PART as usize),
    walk_only: &[EXT as usize],
    seam: None,
};

/// The vocabulary of a programme: floor, doorway, then every piece its
/// zones name, in first-use order. Vocabularies are cached by piece list.
///
/// # Errors
/// More than 256 tiles.
pub fn vocab(prog: &Programme) -> Result<Arc<Vocab>, Failure> {
    let mut pieces: Vec<Piece> = vec![FLOOR, DOOR];
    for z in &prog.zones {
        for it in &z.items {
            if !pieces.iter().any(|p| p.name == it.piece.name) {
                pieces.push(it.piece);
            }
        }
    }
    let names: Vec<&'static str> = pieces.iter().map(|p| p.name).collect();
    static CACHE: OnceLock<Mutex<BTreeMap<Vec<&'static str>, Arc<Vocab>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    if let Some(v) = cache.lock().ok().and_then(|c| c.get(&names).cloned()) {
        return Ok(v);
    }
    let v = Arc::new(Vocab::build(pieces, &MODEL).map_err(|_| Failure {
        attempts: 0,
        rejected: 0,
    })?);
    if let Ok(mut c) = cache.lock() {
        c.insert(names, Arc::clone(&v));
    }
    Ok(v)
}

/// The canonical grid and its edges.
pub struct Frame<'a> {
    /// Width.
    pub w: i64,
    /// Depth.
    pub d: i64,
    /// Rooms.
    pub layout: &'a Layout,
    /// Exterior door edges `(x, y, horizontal)` in canonical edge keys.
    pub doors: &'a [(i64, i64, bool)],
}

impl Frame<'_> {
    /// Index of canonical cell `(x, y)`.
    #[must_use]
    pub fn idx(&self, x: i64, y: i64) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.w && y < self.d)
            .then(|| usize::try_from(y * self.w + x).ok())
            .flatten()
    }

    /// Canonical edge key of side `dir` of cell `(x, y)`.
    #[must_use]
    pub const fn edge_key(x: i64, y: i64, dir: Dir) -> (i64, i64, bool) {
        match dir {
            Dir::N => (x, y, true),
            Dir::S => (x, y + 1, true),
            Dir::W => (x, y, false),
            Dir::E => (x + 1, y, false),
        }
    }

    /// Edge kind on side `dir` of cell `(x, y)`.
    #[must_use]
    pub fn kind(&self, x: i64, y: i64, dir: Dir) -> u8 {
        let (dx, dy) = dir.step();
        let Some(i) = self.idx(x, y) else {
            return SHELL;
        };
        match self.idx(x + dx, y + dy) {
            None => {
                if self.doors.contains(&Self::edge_key(x, y, dir)) {
                    EXT
                } else {
                    SHELL
                }
            }
            Some(j) if self.layout.cell_room[i] == self.layout.cell_room[j] => OPEN,
            Some(_) => PART,
        }
    }
}

/// A furnished grid: tile per canonical cell.
#[derive(Debug, Clone)]
pub struct Furnished {
    /// Tile per cell, row-major.
    pub tiles: Vec<u16>,
    /// Attempts used.
    pub attempts: u8,
}

/// The pass-B problem's pieces: domains, edges, anchors and limits.
pub struct Setup {
    /// Initial domains.
    pub domains: Vec<TileSet>,
    /// Edge kinds.
    pub edges: Vec<[u8; 4]>,
    /// Anchors.
    pub anchors: Vec<Anchor>,
    /// Limits.
    pub limits: Vec<Limit>,
}

/// Builds the pass-B problem data for a frame.
#[must_use]
pub fn setup(prog: &Programme, v: &Vocab, f: &Frame<'_>) -> Setup {
    let (wu, n) = (
        usize::try_from(f.w).unwrap_or(0),
        usize::try_from(f.w * f.d).unwrap_or(0),
    );
    let xy = |i: usize| {
        (
            i64::try_from(i % wu.max(1)).unwrap_or(0),
            i64::try_from(i / wu.max(1)).unwrap_or(0),
        )
    };
    let floor = v.tiles_of(0);
    let door = v.tiles_of(1);
    let mut zone_sets = Vec::new();
    for z in &prog.zones {
        let mut s = floor;
        for it in &z.items {
            if let Some(p) = v.piece(it.piece.name) {
                s = s.or(v.tiles_of(p));
            }
        }
        zone_sets.push(s);
    }
    let lay = f.layout;
    let domains: Vec<TileSet> = (0..n)
        .map(|i| {
            let room = lay.rooms[lay.cell_room[i]];
            zone_sets.get(room.zone).copied().unwrap_or(floor)
        })
        .collect();
    let edges: Vec<[u8; 4]> = (0..n)
        .map(|i| {
            let (x, y) = xy(i);
            Dir::ALL.map(|d| f.kind(x, y, d))
        })
        .collect();
    let mut domains = domains;
    // In an aisled room (a nave) pews stand in every other row, so each
    // row of legroom runs straight to the aisle.
    if let Some(pew) = v.piece("prop.pew") {
        let pews = v.tiles_of(pew);
        for (i, dom) in domains.iter_mut().enumerate() {
            let room = lay.rooms[lay.cell_room[i]];
            let aisled = prog.zones.get(room.zone).is_some_and(|z| z.aisle);
            if aisled && (xy(i).1 - room.rect[1]) % 2 == 0 {
                *dom = dom.minus(pews);
            }
        }
    }
    let mut anchors = Vec::new();
    // Doorways along each tree edge, on the child's side.
    for &(child, parent) in &lay.tree {
        let mut options = Vec::new();
        for (i, dom) in domains.iter_mut().enumerate() {
            if lay.cell_room[i] != child {
                continue;
            }
            let (x, y) = xy(i);
            for d in Dir::ALL {
                let (dx, dy) = d.step();
                if f.idx(x + dx, y + dy)
                    .is_some_and(|j| lay.cell_room[j] == parent)
                {
                    let t = door.iter().find(|&t| {
                        v.tiles
                            .get(t)
                            .is_some_and(|x| usize::from(x.o) == d.index())
                    });
                    if let Some(t) = t {
                        dom.insert(t);
                        options.push((i, TileSet::single(t)));
                    }
                }
            }
        }
        anchors.push(Anchor { options, count: 1 });
    }
    let mut limits = Vec::new();
    let blocking =
        (0..v.tiles.len())
            .filter(|&t| !v.walkable(t))
            .fold(TileSet::empty(), |mut s, t| {
                s.insert(t);
                s
            });
    for (r, room) in lay.rooms.iter().enumerate() {
        let cells: Vec<usize> = (0..n).filter(|&i| lay.cell_room[i] == r).collect();
        let Some(z) = prog.zones.get(room.zone) else {
            continue;
        };
        for it in &z.items {
            let Some(p) = v.piece(it.piece.name) else {
                continue;
            };
            let heads = v.anchors_of(p);
            if it.need > 0 && room.area() >= it.min_room {
                anchors.push(Anchor {
                    options: cells.iter().map(|&i| (i, heads)).collect(),
                    count: u32::from(it.need),
                });
            }
            if it.cap > 0 {
                limits.push(Limit {
                    tiles: heads,
                    cells: cells.clone(),
                    max: u32::from(it.cap),
                });
            }
        }
        let needed: i64 = z
            .items
            .iter()
            .filter(|it| it.piece.blocking && room.area() >= it.min_room)
            .map(|it| i64::from(it.need) * it.piece.w * it.piece.h)
            .sum();
        let needed = usize::try_from(needed).unwrap_or(0);
        limits.push(Limit {
            tiles: blocking,
            max: u32::try_from((cells.len() * usize::from(z.density) / 100).max(needed + 1))
                .unwrap_or(0),
            cells,
        });
    }
    Setup {
        domains,
        edges,
        anchors,
        limits,
    }
}

/// Furnishes a partitioned frame: doorways first, then the circulation
/// kept open, then the furniture.
///
/// # Errors
/// Every attempt failed.
pub fn furnish(
    prog: &Programme,
    f: &Frame<'_>,
    seed: u64,
    key: [i64; 3],
) -> Result<(Arc<Vocab>, Furnished), Failure> {
    let v = vocab(prog)?;
    let s = setup(prog, &v, f);
    let zone_of = |i: usize| {
        f.layout
            .cell_room
            .get(i)
            .and_then(|&r| f.layout.rooms.get(r))
            .and_then(|r| prog.zones.get(r.zone))
    };
    let weight = |i: usize, t: usize| {
        if t == 0 {
            return zone_of(i).map_or(100, |z| z.floor_weight);
        }
        v.piece_of(t).map_or(1, |p| p.weight)
    };
    let (w, h) = (
        usize::try_from(f.w).unwrap_or(0),
        usize::try_from(f.d).unwrap_or(0),
    );
    let doors = v.tiles_of(1);
    let floor = v.tiles_of(0);
    let ndoors = f.layout.tree.len();
    // Stage 1: doorway positions.
    let first = Problem {
        rules: &v.rules,
        w,
        h,
        domains: s.domains.iter().map(|d| d.and(floor.or(doors))).collect(),
        edges: s.edges.clone(),
        weight: &weight,
        anchors: s.anchors[..ndoors].to_vec(),
        limits: Vec::new(),
        seed,
        key: [key[0], key[1], key[2] + 1],
    };
    let placed = solve(&first, arda_wfc::MAX_ATTEMPTS, &|_| true).map_err(|mut e| {
        // Doorway failures are told apart by an impossible reject count.
        e.rejected = u8::MAX;
        e
    })?;
    let mut entrances = Vec::new();
    let mut domains = s.domains;
    for (i, dom) in domains.iter_mut().enumerate() {
        let t = usize::from(placed.tiles[i]);
        if doors.contains(t) {
            *dom = TileSet::single(t);
            entrances.push(i);
            if let Some(tile) = v.tiles.get(t) {
                let d = Dir::from_index(usize::from(tile.o));
                let (dx, dy) = d.step();
                let (x, y) = (
                    i64::try_from(i % w).unwrap_or(0),
                    i64::try_from(i / w).unwrap_or(0),
                );
                if let Some(j) = f.idx(x + dx, y + dy) {
                    entrances.push(j);
                }
            }
        } else {
            *dom = dom.minus(doors);
        }
    }
    for y in 0..f.d {
        for x in 0..f.w {
            if Dir::ALL.iter().any(|&d| f.kind(x, y, d) == EXT) {
                if let Some(i) = f.idx(x, y) {
                    entrances.insert(0, i);
                }
            }
        }
    }
    let open = (0..v.tiles.len())
        .filter(|&t| {
            v.walkable(t)
                && v.piece_of(t)
                    .is_some_and(|p| matches!(p.what, What::Plain | What::Ground(_)))
        })
        .fold(TileSet::empty(), |mut s, t| {
            s.insert(t);
            s
        });
    let aisles: Vec<bool> = f
        .layout
        .rooms
        .iter()
        .map(|r| prog.zones.get(r.zone).is_some_and(|z| z.aisle))
        .collect();
    let keep = super::access::circulation(f, &entrances, &aisles);
    for (i, &k) in keep.iter().enumerate() {
        if k && !doors.meets(&domains[i]) {
            domains[i] = domains[i].and(open);
        }
    }
    // Ordered furniture (pews, dormitory beds, racks, bookshelves, stall
    // tables) stands in rows laid out by rule; the WFC fills around it.
    let blocked: Vec<bool> = (0..domains.len())
        .map(|i| keep[i] || entrances.contains(&i) || doors.meets(&domains[i]))
        .collect();
    let fixed = super::rows::place(prog, &v, f, &blocked);
    let mut anchors = s.anchors[ndoors..].to_vec();
    let mut limits = s.limits;
    super::rows::apply(&fixed, &v, f, &mut domains, &mut anchors, &mut limits);
    let problem = Problem {
        rules: &v.rules,
        w,
        h,
        domains,
        edges: s.edges,
        weight: &weight,
        anchors,
        limits,
        seed,
        key,
    };
    let check = |tiles: &[u16]| super::access::reachable(&v, f, tiles);
    let sol = solve(&problem, arda_wfc::MAX_ATTEMPTS, &check)?;
    Ok((
        Arc::clone(&v),
        Furnished {
            tiles: sol.tiles,
            attempts: sol.attempts.saturating_add(placed.attempts - 1),
        },
    ))
}
