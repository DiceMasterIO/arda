//! Streets, squares, yards, gardens, churchyards, greens, crofts and the
//! curtain wall by WFC (goal 44), consistent with the plan: each square's
//! class comes from the plan grid and the street it belongs to (a kerb
//! only along streets wide enough to have one, the carriageway kept
//! clear), and pieces stand only where their needs are met (benches and
//! woodpiles against buildings, nothing blocking a doorway, graves in
//! rows, arrow loops on the curtain's faces).
//!
//! Problems are solved per global **chunk** of [`CHUNK`] × [`CHUNK`]
//! squares. Across a chunk border only tiles whose side there asks for
//! nothing may stand, so any two chunks join legally without seeing each
//! other, and every block cut from the plan is pixel- and data-exact at
//! its seams (goal 42).

use super::outdoor_vocab::{palette, Class, CAPS, CLASSES, PIECES};
use super::piece::{EdgeModel, Need, Side, Vocab, What};
use crate::block::frame::Want;
use crate::block::interior::{Edge, GLight, GProp};
use crate::plan::grid::{Kind, SQUARE_M};
use crate::plan::TownPlan;
use arda_tactical::catalog::WallRole;
use arda_tactical::layout::EdgeAxis;
use arda_wfc::{solve, Dir, Limit, Problem, TileSet};
use std::collections::BTreeSet;
use std::sync::OnceLock;

/// Chunk edge, squares (global, aligned).
pub const CHUNK: i64 = 32;

const OPEN: u8 = 0;
const BUILDING: u8 = 1;
const CURTAIN: u8 = 2;
const BORDER: u8 = 3;
const DOOR: u8 = 4;
const SEAM: u8 = 5;

/// The outdoor edge model.
pub const MODEL: EdgeModel = EdgeModel {
    kinds: 6,
    open: OPEN as usize,
    walls: &[BUILDING as usize, CURTAIN as usize],
    partition: None,
    walk_only: &[DOOR as usize],
    seam: Some(SEAM as usize),
};

/// The shared outdoor vocabulary (`None` only if it outgrew the solver's
/// tile limit, which a test rules out).
pub fn vocab() -> Option<&'static Vocab> {
    static V: OnceLock<Option<Vocab>> = OnceLock::new();
    V.get_or_init(|| Vocab::build(PIECES.to_vec(), &MODEL).ok())
        .as_ref()
}

fn kind(plan: &TownPlan, x: i64, y: i64) -> Kind {
    crate::block::ground::kind(plan, x, y)
}

/// Plan-wide facts the classes need.
pub struct Site {
    /// Squares the market well and braziers take.
    pub fixed: BTreeSet<(i64, i64)>,
    /// Door edges `(outside x, outside y, direction to the building)`.
    pub doors: BTreeSet<(i64, i64, usize)>,
    /// Stall squares.
    pub stalls: BTreeSet<(i64, i64)>,
}

impl Site {
    /// Reads the facts of a plan.
    #[must_use]
    pub fn of(plan: &TownPlan) -> Self {
        let mut doors = BTreeSet::new();
        let mut stalls = BTreeSet::new();
        for b in &plan.buildings {
            for d in &b.doors {
                let (ox, oy) = d.outside();
                let dir = match d.side.opposite() {
                    crate::plan::grid::Side::North => Dir::N,
                    crate::plan::grid::Side::East => Dir::E,
                    crate::plan::grid::Side::South => Dir::S,
                    crate::plan::grid::Side::West => Dir::W,
                };
                doors.insert((ox, oy, dir.index()));
            }
            if b.function == crate::function::BuildingFunction::Stall {
                for y in b.rect.y0..b.rect.y1 {
                    for x in b.rect.x0..b.rect.x1 {
                        stalls.insert((x, y));
                    }
                }
            }
        }
        Self {
            fixed: crate::block::exterior::square_features(plan)
                .into_iter()
                .collect(),
            doors,
            stalls,
        }
    }
}

fn street_width(plan: &TownPlan, x: i64, y: i64) -> (f64, bool) {
    plan.grid
        .gidx(x, y)
        .and_then(|k| {
            plan.streets
                .get(usize::from(plan.grid.street[k]).checked_sub(1)?)
        })
        .map_or((0.0, false), |s| (s.width_m / SQUARE_M, s.paved))
}

/// The class of a global square.
#[must_use]
pub fn class(plan: &TownPlan, site: &Site, x: i64, y: i64) -> Class {
    use crate::plan::croft::{parcel, rim, use_of, Use};
    let around = |ok: &dyn Fn(Kind) -> bool| {
        Dir::ALL.iter().all(|d| {
            let (dx, dy) = d.step();
            ok(kind(plan, x + dx, y + dy))
        })
    };
    let public = |k: Kind| matches!(k, Kind::Street | Kind::Square | Kind::Gate | Kind::Bridge);
    match kind(plan, x, y) {
        Kind::Street => {
            let (width, paved) = street_width(plan, x, y);
            if width >= 3.0 && !around(&public) {
                if paved {
                    Class::KerbPaved
                } else {
                    Class::KerbDirt
                }
            } else {
                Class::Carriage
            }
        }
        Kind::Square => {
            if site.fixed.contains(&(x, y)) {
                Class::Void
            } else if Dir::ALL.iter().any(|d| {
                let (dx, dy) = d.step();
                site.stalls.contains(&(x + dx, y + dy))
            }) {
                Class::Pitch
            } else if around(&|k| public(k) || k == Kind::Green) {
                Class::Plaza
            } else {
                Class::PlazaEdge
            }
        }
        Kind::Front => Class::Front,
        Kind::Yard => Class::Yard,
        // A bailey has no one building's trade: a work yard's stacks.
        Kind::Bailey => Class::Workyard,
        Kind::Garden => Class::Garden,
        Kind::Churchyard => Class::Churchyard,
        Kind::Green => Class::Green,
        Kind::Croft => {
            let s = plan.seed;
            if rim(s, |a, b| kind(plan, a, b) == Kind::Open, x, y) {
                return Class::Rim;
            }
            match use_of(parcel(s, x, y)) {
                Use::Paddock => Class::Paddock,
                Use::Orchard => Class::Orchard,
                Use::Kitchen => Class::Kitchen,
                Use::Meadow => Class::Meadow,
                Use::Yard => Class::Workyard,
            }
        }
        Kind::Wall => Class::Wall,
        Kind::Gate => Class::Gate,
        _ => Class::Void,
    }
}

/// The edge kind on side `d` of square `(x, y)`, the same from both
/// squares of the edge; `inside` tells whether a square belongs to the
/// chunk being solved.
fn edge(
    plan: &TownPlan,
    site: &Site,
    classes: &dyn Fn(i64, i64) -> Class,
    (x, y): (i64, i64),
    d: Dir,
    inside: &dyn Fn(i64, i64) -> bool,
) -> u8 {
    let (dx, dy) = d.step();
    let (bx, by) = (x + dx, y + dy);
    let (ka, kb) = (kind(plan, x, y), kind(plan, bx, by));
    if (ka == Kind::Building) != (kb == Kind::Building) {
        let door = site.doors.contains(&(x, y, d.index()))
            || site.doors.contains(&(bx, by, d.opposite().index()));
        return if door { DOOR } else { BUILDING };
    }
    if (ka == Kind::Wall) != (kb == Kind::Wall) {
        return CURTAIN;
    }
    if classes(bx, by) == Class::Void || classes(x, y) == Class::Void {
        return BORDER;
    }
    if inside(bx, by) && inside(x, y) {
        OPEN
    } else {
        SEAM
    }
}

/// One solved chunk in global squares.
#[derive(Debug, Clone, Default)]
pub struct Chunk {
    /// Props.
    pub props: Vec<GProp>,
    /// Lights, owners indexing `props`.
    pub lights: Vec<GLight>,
    /// Ground overrides.
    pub ground: Vec<((i64, i64), &'static str)>,
    /// Curtain-wall pieces chosen (arrow loops).
    pub walls: Vec<Edge>,
    /// Whether the relaxed fill (plain ground) was used.
    pub relaxed: bool,
    /// Whether the chunk has any square to fill.
    pub any: bool,
    /// Tiles, row-major (for tests).
    pub tiles: Vec<u16>,
    /// Edge kinds, row-major (for tests).
    pub edges: Vec<[u8; 4]>,
    /// Domains, row-major (for tests).
    pub domains: Vec<TileSet>,
}

fn tiles_of(v: &Vocab, c: Class) -> TileSet {
    let mut s = TileSet::empty();
    for &(name, _) in palette(c) {
        if let Some(p) = v.piece(name) {
            s = s.or(v.tiles_of(p));
        }
    }
    s
}

/// Solves chunk `(cx, cy)` of a plan.
#[must_use]
pub fn chunk(plan: &TownPlan, site: &Site, cx: i64, cy: i64) -> Chunk {
    let Some(v) = vocab() else {
        return Chunk::default();
    };
    let (x0, y0) = (cx * CHUNK, cy * CHUNK);
    let n = usize::try_from(CHUNK * CHUNK).unwrap_or(0);
    let w = usize::try_from(CHUNK).unwrap_or(0);
    let at = |i: usize| {
        let i = i64::try_from(i).unwrap_or(0);
        (x0 + i % CHUNK, y0 + i / CHUNK)
    };
    let classes: Vec<Class> = (0..n)
        .map(|i| {
            let (x, y) = at(i);
            class(plan, site, x, y)
        })
        .collect();
    let mut out = Chunk {
        any: classes.iter().any(|&c| c != Class::Void),
        ..Chunk::default()
    };
    if !out.any {
        return out;
    }
    let cls = |x: i64, y: i64| {
        if (x0..x0 + CHUNK).contains(&x) && (y0..y0 + CHUNK).contains(&y) {
            let i = usize::try_from((y - y0) * CHUNK + (x - x0)).unwrap_or(0);
            classes[i]
        } else {
            class(plan, site, x, y)
        }
    };
    let inside = |x: i64, y: i64| (x0..x0 + CHUNK).contains(&x) && (y0..y0 + CHUNK).contains(&y);
    let sets: Vec<TileSet> = CLASSES.iter().map(|&c| tiles_of(v, c)).collect();
    let ground = TileSet::single(0);
    let domains: Vec<TileSet> = classes
        .iter()
        .map(|&c| {
            if c == Class::Void {
                ground
            } else {
                CLASSES
                    .iter()
                    .position(|&k| k == c)
                    .map_or(ground, |k| sets[k])
            }
        })
        .collect();
    let edges: Vec<[u8; 4]> = (0..n)
        .map(|i| {
            let p = at(i);
            Dir::ALL.map(|d| edge(plan, site, &cls, p, d, &inside))
        })
        .collect();
    let weight = |i: usize, t: usize| -> u32 {
        let name = v.piece_of(t).map_or("", |p| p.name);
        palette(classes[i])
            .iter()
            .find(|(k, _)| *k == name)
            .map_or(0, |(_, w)| *w)
    };
    let limits = CAPS
        .iter()
        .filter_map(|&(name, max)| {
            Some(Limit {
                tiles: v.anchors_of(v.piece(name)?),
                cells: Vec::new(),
                max,
            })
        })
        .collect();
    let problem = Problem {
        rules: &v.rules,
        w,
        h: w,
        domains,
        edges,
        weight: &weight,
        anchors: Vec::new(),
        limits,
        seed: plan.seed,
        key: [cx, cy, 0x0D00],
    };
    let tiles = match solve(&problem, arda_wfc::MAX_ATTEMPTS, &|_| true) {
        Ok(s) => s.tiles,
        Err(_) => {
            out.relaxed = true;
            vec![0; n]
        }
    };
    decode(plan, v, &classes, &tiles, (x0, y0), &mut out);
    out.tiles = tiles;
    out.edges = problem.edges;
    out.domains = problem.domains;
    out
}

fn decode(
    plan: &TownPlan,
    v: &Vocab,
    classes: &[Class],
    tiles: &[u16],
    (x0, y0): (i64, i64),
    out: &mut Chunk,
) {
    let plaza_flag = plan.tier == crate::site::Tier::City;
    for (i, &t) in tiles.iter().enumerate() {
        let ii = i64::try_from(i).unwrap_or(0);
        let (x, y) = (x0 + ii % CHUNK, y0 + ii / CHUNK);
        match classes[i] {
            Class::KerbPaved => out.ground.push(((x, y), "flagstone")),
            Class::PlazaEdge => out
                .ground
                .push(((x, y), if plaza_flag { "cobbles" } else { "flagstone" })),
            _ => {}
        }
        let t = usize::from(t);
        let (Some(tile), Some(piece)) = (v.tiles.get(t), v.piece_of(t)) else {
            continue;
        };
        if !tile.anchor() {
            continue;
        }
        match piece.what {
            // Art-free pieces keep their place in the solve (the kerb's
            // rhythm) but stay out of the layout until a library draws them.
            What::Prop(id) if super::outdoor_vocab::ART_FREE.contains(&id) => {}
            What::Prop(id) => {
                #[allow(clippy::cast_precision_loss)]
                let e = [
                    x as f64,
                    y as f64,
                    (x + tile.rw) as f64,
                    (y + tile.rh) as f64,
                ];
                out.props.push(GProp {
                    want: Want::Id(id),
                    x: (e[0] + e[2]) * 0.5,
                    y: (e[1] + e[3]) * 0.5,
                    rot: Vocab::rotation(tile.o),
                    extent: e,
                });
                if let Some((radius_ft, colour)) = piece.light {
                    #[allow(clippy::cast_precision_loss)]
                    out.lights.push(GLight {
                        x: x as f64 + 0.5,
                        y: y as f64 + 0.5,
                        radius_ft,
                        colour,
                        owner: Some(out.props.len() - 1),
                    });
                }
            }
            What::Loop => {
                for d in Dir::ALL {
                    if v.sides[t][d.index()] == Side::Need(Need::Wall) {
                        let key = match d {
                            Dir::N => (x, y, EdgeAxis::Horizontal),
                            Dir::S => (x, y + 1, EdgeAxis::Horizontal),
                            Dir::W => (x, y, EdgeAxis::Vertical),
                            Dir::E => (x + 1, y, EdgeAxis::Vertical),
                        };
                        out.walls.push((key, (WallRole::Window, "city_wall")));
                    }
                }
            }
            _ => {}
        }
    }
}

/// Chunks whose squares or props can touch `x0..x1 × y0..y1`.
#[must_use]
pub fn chunks_near(x0: i64, y0: i64, x1: i64, y1: i64) -> Vec<(i64, i64)> {
    let m = 3;
    let (cx0, cy0) = ((x0 - m).div_euclid(CHUNK), (y0 - m).div_euclid(CHUNK));
    let (cx1, cy1) = (
        (x1 + m - 1).div_euclid(CHUNK),
        (y1 + m - 1).div_euclid(CHUNK),
    );
    (cy0..=cy1)
        .flat_map(|cy| (cx0..=cx1).map(move |cx| (cx, cy)))
        .collect()
}

/// Illegal places inside a solved chunk (for review and tests).
#[must_use]
pub fn violations(ch: &Chunk) -> usize {
    let Some(v) = vocab() else { return usize::MAX };
    let w = usize::try_from(CHUNK).unwrap_or(0);
    let weight = |_: usize, _: usize| 1;
    let p = Problem {
        rules: &v.rules,
        w,
        h: w,
        domains: ch.domains.clone(),
        edges: ch.edges.clone(),
        weight: &weight,
        anchors: Vec::new(),
        limits: Vec::new(),
        seed: 0,
        key: [0; 3],
    };
    arda_wfc::violations(&p, &ch.tiles).len()
}

/// Illegal pairs across the seam between chunk `a` and the chunk east of
/// it (`east`) or south of it: tiles on either side of a seam edge must be
/// legal as if the two chunks were one.
#[must_use]
pub fn seam_violations(a: &Chunk, b: &Chunk, east: bool) -> usize {
    let Some(v) = vocab() else { return usize::MAX };
    let c = usize::try_from(CHUNK).unwrap_or(0);
    let mut bad = 0;
    for k in 0..c {
        let (i, j, d) = if east {
            (k * c + c - 1, k * c, Dir::E)
        } else {
            ((c - 1) * c + k, k, Dir::S)
        };
        let (Some(ea), Some(&ta), Some(&tb)) = (a.edges.get(i), a.tiles.get(i), b.tiles.get(j))
        else {
            continue;
        };
        if ea[d.index()] == SEAM
            && !v
                .rules
                .legal(usize::from(ta), d, usize::from(tb), usize::from(OPEN))
        {
            bad += 1;
        }
    }
    bad
}
