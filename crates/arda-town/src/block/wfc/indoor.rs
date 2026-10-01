//! A building's interior by WFC: the plan fixes the footprint, shell and
//! exterior doors; pass A partitions rooms, pass B places doorways and
//! furniture, and the shell's kit pieces (runs, windows) follow. On
//! failure the rule programme of `block::interior` is the relaxed fill,
//! marked for review (goal 47).

use super::furnish::{furnish, Frame as Grid};
use super::piece::{Need, Side, What};
use super::programme::{self, Programme};
use super::rooms::{self, Layout};
use super::shell::{self, Slot};
use crate::block::frame::{Canon, Frame, Want};
use crate::block::interior::{self, door_edge, light_on, to_global, wide_entrance, Interior};
use crate::block::kits;
use crate::function::BuildingFunction as F;
use crate::plan::{Building, TownPlan, WealthLevel};
use arda_tactical::catalog::WallRole;
use arda_tactical::layout::EdgeAxis;
use arda_wfc::Dir;

/// An interior and how it was made.
#[derive(Debug, Clone)]
pub struct Built {
    /// The interior in global squares.
    pub interior: Interior,
    /// Whether the relaxed fill (the rule programme) was used.
    pub relaxed: bool,
}

/// Stage keys of the WFC problems (the third key word).
const ROOMS: i64 = 1;
const FURNISH: i64 = 2;
const WALLS: i64 = 3;

/// The inside cell of a canonical shell edge.
fn inside(w: i64, d: i64, e: (i64, i64, bool)) -> (i64, i64) {
    let (x, y, hz) = e;
    if hz {
        (x, if y == 0 { 0 } else { d - 1 })
    } else {
        (if x == 0 { 0 } else { w - 1 }, y)
    }
}

/// Everything the WFC decided, before drawing.
pub struct Plan<'a> {
    /// The programme after fitting.
    pub prog: Programme,
    /// Rooms.
    pub layout: Layout,
    /// Exterior door edges, canonical.
    pub doors: Vec<(i64, i64, bool)>,
    /// Pass-B tiles and vocabulary.
    pub furnished: (
        std::sync::Arc<super::piece::Vocab>,
        super::furnish::Furnished,
    ),
    /// The building.
    pub b: &'a Building,
}

fn shell_canon(plan: &TownPlan, b: &Building) -> (Canon, Frame, Vec<(i64, i64, bool)>) {
    let frame = Frame::new(b.rect, b.front.turns());
    let kit = kits::shell(b, kits::tradition(&plan.culture));
    let mut c = Canon::new(frame.w, frame.d, kits::floor(b));
    c.shell(kit);
    let mut doors = Vec::new();
    for (k, d) in b.doors.iter().enumerate() {
        if let Some(e) = door_edge(&frame, d) {
            let role = if k == 0 && wide_entrance(b.function) {
                WallRole::Gate
            } else {
                WallRole::Door
            };
            c.walls.insert(e, (role, kit));
            doors.push(e);
        }
    }
    if b.function == F::MarketHall {
        // An open arcade: every other bay of the long walls is an opening.
        for x in (1..frame.w - 1).step_by(2) {
            for y in [0, frame.d] {
                c.h(x, y, WallRole::Gate, kit);
                doors.push((x, y, true));
            }
        }
    }
    (c, frame, doors)
}

/// The WFC pass that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Pass A, room partitions.
    Rooms,
    /// Pass B, doorways and furniture, with the solver's report.
    Furnish(arda_wfc::Failure),
}

/// Solves rooms and furniture for a walled building with a programme.
///
/// # Errors
/// The pass that needed the relaxed fill.
pub fn solve<'a>(plan: &TownPlan, b: &'a Building) -> Result<Option<Plan<'a>>, Stage> {
    if !b.function.walled() {
        return Ok(None);
    }
    let Some(prog) = programme::of(plan, b) else {
        return Ok(None);
    };
    let (c, _, doors) = shell_canon(plan, b);
    let prog = programme::fit(prog, c.w, c.d);
    let id = i64::try_from(b.id.0).unwrap_or(0);
    // The interior salt (goal 64) re-rolls homes that would repeat a
    // neighbour; it steps by four so the stage sub-keys never meet.
    let salt = i64::from(plan.interior_salt(b)) * 4;
    let cells: Vec<(i64, i64)> = doors.iter().map(|&e| inside(c.w, c.d, e)).collect();
    let layout = rooms::partition(&prog, c.w, c.d, &cells, plan.seed, [id, ROOMS, salt])
        .map_err(|_| Stage::Rooms)?;
    let grid = Grid {
        w: c.w,
        d: c.d,
        layout: &layout,
        doors: &doors,
    };
    let furnished =
        furnish(&prog, &grid, plan.seed, [id, FURNISH, salt]).map_err(Stage::Furnish)?;
    Ok(Some(Plan {
        prog,
        layout,
        doors,
        furnished,
        b,
    }))
}

/// The interior of one building under the WFC strategy.
#[must_use]
pub fn build(plan: &TownPlan, b: &Building) -> Built {
    match solve(plan, b) {
        Ok(Some(p)) => Built {
            interior: draw(plan, &p),
            relaxed: false,
        },
        Ok(None) => Built {
            interior: interior::build(plan, b),
            relaxed: false,
        },
        Err(_) => Built {
            interior: interior::build(plan, b),
            relaxed: true,
        },
    }
}

fn party(plan: &TownPlan, b: &Building, e: (i64, i64, EdgeAxis)) -> bool {
    let (x, y, axis) = e;
    let (p, q) = match axis {
        EdgeAxis::Horizontal => ((x, y - 1), (x, y)),
        EdgeAxis::Vertical => ((x - 1, y), (x, y)),
    };
    let out = if b.rect.contains(p.0, p.1) { q } else { p };
    plan.grid
        .gidx(out.0, out.1)
        .is_some_and(|k| plan.grid.building[k] != 0 && u64::from(plan.grid.building[k]) != b.id.0)
}

/// Draws a solved building into global squares.
#[must_use]
pub fn draw(plan: &TownPlan, p: &Plan<'_>) -> Interior {
    let (c, frame) = draw_canon(plan, p);
    to_global(&c, &frame)
}

/// The interior of `b` in its canonical frame under the WFC strategy
/// (the rule programme where the WFC relaxes or has no programme).
#[must_use]
pub fn canon(plan: &TownPlan, b: &Building) -> (Canon, Frame) {
    match solve(plan, b) {
        Ok(Some(p)) => draw_canon(plan, &p),
        _ => interior::canon(plan, b, plan.interior_salt(b)),
    }
}

/// The variety of `plan`'s home interiors as the WFC draws them (goal 64).
#[must_use]
pub fn diversity(plan: &TownPlan) -> interior::variety::Diversity {
    interior::variety::diversity_by(plan, |b| {
        interior::variety::fingerprint_canon(&canon(plan, b).0)
    })
}

/// Draws a solved building in its canonical frame.
#[must_use]
pub fn draw_canon(plan: &TownPlan, p: &Plan<'_>) -> (Canon, Frame) {
    let b = p.b;
    let (mut c, frame, _) = shell_canon(plan, b);
    let kit = kits::shell(b, kits::tradition(&plan.culture));
    let pkit = kits::partition(b);
    let (w, d) = (c.w, c.d);
    let lay = &p.layout;
    for r in &lay.rooms {
        let z = &p.prog.zones[r.zone];
        c.room(z.tag, r.rect);
        if let Some(f) = z.floor {
            c.floor_rect(r.rect, f);
        }
    }
    let room_at = |x: i64, y: i64| usize::try_from(y * w + x).ok().map(|i| lay.cell_room[i]);
    for y in 0..d {
        for x in 0..w {
            if x + 1 < w && room_at(x, y) != room_at(x + 1, y) {
                c.v(x + 1, y, WallRole::Run, pkit);
            }
            if y + 1 < d && room_at(x, y) != room_at(x, y + 1) {
                c.h(x, y + 1, WallRole::Run, pkit);
            }
        }
    }
    let (v, f) = &p.furnished;
    let mut backed = Vec::new();
    for (i, &t) in f.tiles.iter().enumerate() {
        let t = usize::from(t);
        let (Some(tile), Ok(ii)) = (v.tiles.get(t), i64::try_from(i)) else {
            continue;
        };
        let (x, y) = (ii % w, ii / w);
        for dir in Dir::ALL {
            if v.sides[t][dir.index()] == Side::Need(Need::Wall) {
                backed.push(Grid::edge_key(x, y, dir));
            }
        }
        let piece = &v.pieces[tile.piece];
        if !tile.anchor() {
            continue;
        }
        let r = [x, y, x + tile.rw, y + tile.rh];
        let rot = super::piece::Vocab::rotation(tile.o);
        let placed = match piece.what {
            What::Prop(id) => c.put(Want::Id(id), r, rot),
            What::Query(tags) => c.put(
                Want::Query(tags.iter().map(|s| (*s).to_string()).collect()),
                r,
                rot,
            ),
            What::Ground(key) => {
                c.floor_rect(r, key);
                None
            }
            What::Door => {
                let e = Grid::edge_key(x, y, Dir::from_index(usize::from(tile.o)));
                c.walls.insert(e, (WallRole::Door, pkit));
                None
            }
            What::Plain | What::Loop => None,
        };
        if let Some((radius, colour)) = piece.light {
            light_on(&mut c, placed, radius, colour);
        }
    }
    windows(plan, b, &mut c, &frame, &backed, kit);
    (c, frame)
}

fn windows(
    plan: &TownPlan,
    b: &Building,
    c: &mut Canon,
    frame: &Frame,
    backed: &[(i64, i64, bool)],
    kit: &'static str,
) {
    let (w, d) = (c.w, c.d);
    let sides: Vec<Vec<(i64, i64, bool)>> = vec![
        (0..w).map(|x| (x, 0, true)).collect(),
        (0..d).map(|y| (w, y, false)).collect(),
        (0..w).map(|x| (x, d, true)).collect(),
        (0..d).map(|y| (0, y, false)).collect(),
    ];
    let slots: Vec<Vec<Slot>> = sides
        .iter()
        .map(|s| {
            let n = s.len();
            s.iter()
                .enumerate()
                .map(|(k, &e)| {
                    let fixed = c.walls.get(&e).is_some_and(|x| x.0 != WallRole::Run);
                    if fixed {
                        Slot::Fixed
                    } else if k == 0
                        || k + 1 == n
                        || backed.contains(&e)
                        || party(plan, b, frame.edge(e.0, e.1, e.2))
                    {
                        Slot::Run
                    } else {
                        Slot::Free
                    }
                })
                .collect()
        })
        .collect();
    let weight = match (b.storeys > 1, b.wealth_level) {
        (_, WealthLevel::Poor) => 1,
        (true, _) => 3,
        (false, _) => 2,
    };
    let id = i64::try_from(b.id.0).unwrap_or(0);
    let (rows, _) = shell::choose(&slots, weight, plan.seed, [id, WALLS, 0]);
    for (s, row) in sides.iter().zip(rows) {
        for (&e, piece) in s.iter().zip(row) {
            if piece == shell::WINDOW {
                c.walls.insert(e, (WallRole::Window, kit));
            }
        }
    }
}
