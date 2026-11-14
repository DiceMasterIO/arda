//! Where a level's traps lie (logic/18 §site-traps). Arda places them;
//! the consumer resolves each from the SRD 5.1 text of its kind.

use crate::rng::Rng;
use crate::{Dungeon, Kind};
use arda_scene::{Movement, Scene, Sq};
use arda_tactical::layout::EdgeAxis;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The SRD 5.1 sample traps. Closed: a consumer resolves rules from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrapKind {
    /// Collapsing roof.
    CollapsingRoof,
    /// Falling net.
    FallingNet,
    /// Fire-breathing statue; never placed (needs a statue and a facing).
    FireBreathingStatue,
    /// Simple pit.
    SimplePit,
    /// Hidden pit.
    HiddenPit,
    /// Locking pit.
    LockingPit,
    /// Spiked pit.
    SpikedPit,
    /// Poison darts.
    PoisonDarts,
    /// Poison needle, in a locked door's lock.
    PoisonNeedle,
    /// Rolling sphere; never placed (needs a path).
    RollingSphere,
    /// Sphere of annihilation; never placed (needs a statue).
    SphereOfAnnihilation,
}

/// One placed trap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trap {
    /// `t1`, `t2`, … in placement order; stable per level.
    pub id: String,
    /// Which SRD trap.
    pub kind: TrapKind,
    /// The squares that trigger it.
    pub squares: Vec<(u32, u32)>,
    /// The squares hit, where the SRD separates them from the trigger
    /// (falling net, collapsing roof); empty otherwise.
    pub affected: Vec<(u32, u32)>,
    /// The locked door whose lock holds a poison needle.
    pub door: Option<(u32, u32, EdgeAxis)>,
}

const SINGLE: [TrapKind; 5] = [
    TrapKind::SimplePit,
    TrapKind::HiddenPit,
    TrapKind::LockingPit,
    TrapKind::SpikedPit,
    TrapKind::PoisonDarts,
];
const AREA: [TrapKind; 2] = [TrapKind::FallingNet, TrapKind::CollapsingRoof];

/// How far, in squares (Chebyshev), every trap square stays from an exit
/// square: the arriving party stands there.
const EXIT_CLEARANCE: u32 = 2;

fn sides(x: u32, y: u32, axis: EdgeAxis) -> [Option<(u32, u32)>; 2] {
    match axis {
        EdgeAxis::Horizontal => [Some((x, y)), y.checked_sub(1).map(|y| (x, y))],
        EdgeAxis::Vertical => [Some((x, y)), x.checked_sub(1).map(|x| (x, y))],
    }
}

/// Places the level's traps: 1–2 at 32 × 24, 2–4 at 48 × 36, 3–6 above,
/// fewer when the level runs out of eligible squares. A trap square is
/// normal-movement dry floor more than [`EXIT_CLEARANCE`] squares from
/// every exit, on neither side of a door, and used by no other trap; a
/// poison needle takes the two sides of its own locked door.
#[must_use]
pub fn place(d: &Dungeon, scene: &Scene, seed: u64) -> Vec<Trap> {
    let mut rng = Rng::new(seed ^ 0x7_2A95);
    let (w, h) = (d.layout.width, d.layout.height);
    let (lo, hi) = match w * h {
        0..=768 => (1, 2),
        769..=1728 => (2, 4),
        _ => (3, 6),
    };
    let want = rng.range(lo, hi) as usize;
    let door_sides: BTreeSet<(u32, u32)> = d
        .doors
        .iter()
        .flat_map(|g| sides(g.x, g.y, g.axis))
        .flatten()
        .collect();
    let clear = |x: u32, y: u32| {
        scene.movement_at(Sq(x, y)) == Movement::Normal
            && d.layout.squares[(y * w + x) as usize].water_depth_ft == 0
            && d.exits
                .iter()
                .all(|e| x.abs_diff(e.x).max(y.abs_diff(e.y)) > EXIT_CLEARANCE)
    };
    let mut free: Vec<(u32, u32)> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|&(x, y)| clear(x, y) && !door_sides.contains(&(x, y)))
        .collect();
    rng.shuffle(&mut free);
    let mut locked: Vec<_> = d
        .doors
        .iter()
        .filter(|g| g.locked && !g.secret)
        .filter(|g| {
            sides(g.x, g.y, g.axis)
                .iter()
                .all(|s| s.is_some_and(|(x, y)| clear(x, y)))
        })
        .collect();
    rng.shuffle(&mut locked);
    let mut used: BTreeSet<(u32, u32)> = BTreeSet::new();
    let mut out: Vec<Trap> = Vec::new();
    for _ in 0..want {
        let mut kinds: Vec<TrapKind> = SINGLE.iter().chain(&AREA).copied().collect();
        if d.params.kind == Kind::Cave {
            kinds.retain(|k| *k != TrapKind::PoisonDarts);
        } else if !locked.is_empty() {
            kinds.push(TrapKind::PoisonNeedle);
        }
        let kind = kinds[rng.index(kinds.len())];
        let mut trap = Trap {
            id: format!("t{}", out.len() + 1),
            kind,
            squares: Vec::new(),
            affected: Vec::new(),
            door: None,
        };
        if kind == TrapKind::PoisonNeedle {
            if let Some(g) = locked.pop() {
                trap.squares = sides(g.x, g.y, g.axis).into_iter().flatten().collect();
                trap.door = Some((g.x, g.y, g.axis));
            }
        } else if AREA.contains(&kind) {
            // The trip wire, then the 10-ft square it brings down beside it.
            let fits = |&(x, y): &(u32, u32)| {
                let block = [(x + 1, y), (x + 2, y), (x + 1, y + 1), (x + 2, y + 1)];
                !used.contains(&(x, y))
                    && block.iter().all(|s| {
                        s.0 < w
                            && s.1 < h
                            && clear(s.0, s.1)
                            && !door_sides.contains(s)
                            && !used.contains(s)
                    })
            };
            if let Some(&(x, y)) = free.iter().find(|s| fits(s)) {
                trap.squares = vec![(x, y)];
                trap.affected = vec![(x + 1, y), (x + 2, y), (x + 1, y + 1), (x + 2, y + 1)];
            } else if let Some(&s) = free.iter().find(|s| !used.contains(s)) {
                // No room for the 10-ft square: a pit takes its place, so
                // the level keeps its count.
                trap.kind = SINGLE[rng.index(4)];
                trap.squares = vec![s];
            }
        } else if let Some(&s) = free.iter().find(|s| !used.contains(s)) {
            trap.squares = vec![s];
        }
        if trap.squares.is_empty() {
            continue;
        }
        used.extend(trap.squares.iter().chain(&trap.affected));
        out.push(trap);
    }
    out
}
