//! A deterministic timeline of 200–500 years whose outcome is the present
//! world (deliverable 3). Stages run in causal order: founding → realm
//! formation → wars and border shifts → disasters → reigns → ruins →
//! prosperity → history hooks.

pub mod disasters;
pub mod founding;
pub mod lore;
pub mod prosperity;
pub mod reigns;
pub mod ruins;
pub mod run;
pub mod wars;

use crate::input::Tier;
use crate::refs::EntityRef;
use serde::{Deserialize, Serialize};

/// What happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// A settlement is founded.
    Founding,
    /// A realm is formed around its seat.
    RealmFormed,
    /// A new ruling house takes power.
    DynastyFounded,
    /// A ruler takes the throne.
    Accession,
    /// A war begins.
    War,
    /// A war ends in a peace.
    Peace,
    /// A settlement passes from one realm to another.
    BorderShift,
    /// A river flood.
    Flood,
    /// An epidemic.
    Plague,
    /// A town fire.
    Fire,
    /// A failed harvest.
    Famine,
    /// A mine disaster.
    MineCollapse,
    /// A settlement is abandoned and becomes a ruin.
    Abandonment,
}

/// One timeline entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Stable id, in timeline order.
    pub id: u32,
    /// Year it began.
    pub year: i32,
    /// Year it ended, for lasting events; `None` if instantaneous or ongoing.
    pub end_year: Option<i32>,
    /// Whether it is still going on at present.
    pub ongoing: bool,
    /// Kind.
    pub kind: EventKind,
    /// Short title.
    pub title: String,
    /// One or two sentences of original text.
    pub text: String,
    /// Settlements involved.
    #[serde(with = "crate::ids::vec")]
    pub settlements: Vec<u64>,
    /// Realms involved.
    #[serde(with = "crate::ids::vec")]
    pub realms: Vec<u64>,
    /// Further references.
    pub refs: Vec<EntityRef>,
    /// Event that caused this one.
    pub cause: Option<u32>,
    /// 1 (minor) to 3 (catastrophic).
    pub severity: u8,
}

/// A ruling house.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dynasty {
    /// Stable id (`d<realm>.<n>`).
    pub id: String,
    /// Realm ruled.
    #[serde(with = "crate::ids::str")]
    pub realm_id: u64,
    /// House name.
    pub name: String,
    /// First year in power.
    pub from: i32,
    /// Last year in power; `None` if ruling now.
    pub to: Option<i32>,
    /// How it came to power (`founding`, `usurpation`, `succession`, `election`, `conquest`).
    pub origin: String,
}

/// One reign.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reign {
    /// Realm ruled.
    #[serde(with = "crate::ids::str")]
    pub realm_id: u64,
    /// Ruling house.
    pub dynasty_id: String,
    /// Given name.
    pub given: String,
    /// Regnal style, e.g. "Oswin II the Patient".
    pub regnal: String,
    /// Female ruler.
    pub female: bool,
    /// Title held.
    pub title: String,
    /// First year.
    pub from: i32,
    /// Last year; `None` if reigning now.
    pub to: Option<i32>,
    /// How the reign ended (`died`, `fell`, `deposed`, `plague`, `abdicated`, `heirless`).
    pub end: Option<String>,
}

/// A settlement's change of realm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BorderShift {
    /// Settlement.
    #[serde(with = "crate::ids::str")]
    pub settlement: u64,
    /// Realm it belonged to before.
    #[serde(with = "crate::ids::str")]
    pub from_realm: u64,
    /// Realm it belongs to now.
    #[serde(with = "crate::ids::str")]
    pub to_realm: u64,
    /// Year it changed hands.
    pub year: i32,
    /// War that moved it.
    pub war_event: u32,
}

/// A war between two realms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct War {
    /// War event id.
    pub event: u32,
    /// Name.
    pub name: String,
    /// The two realms, lower id first.
    #[serde(with = "crate::ids::pair")]
    pub realms: [u64; 2],
    /// First year.
    pub from: i32,
    /// Last year; `None` if still fought.
    pub to: Option<i32>,
    /// Winner, once over.
    #[serde(default, with = "crate::ids::opt")]
    pub winner: Option<u64>,
}

/// A ruin of an abandoned settlement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ruin {
    /// Stable id, 1-based.
    pub id: u32,
    /// Name it bore.
    pub name: String,
    /// What it was (`hamlet`, `village`, `watchtower`, …).
    pub kind: String,
    /// Size it reached.
    pub former_tier: Tier,
    /// Position, metres.
    pub x_m: i64,
    /// Position, metres.
    pub y_m: i64,
    /// Nearest living settlement.
    #[serde(with = "crate::ids::str")]
    pub near: u64,
    /// Road it lies beside, if any.
    #[serde(default, with = "crate::ids::opt")]
    pub road: Option<u64>,
    /// Realm whose land it lies in.
    #[serde(with = "crate::ids::str")]
    pub realm_id: u64,
    /// Year founded.
    pub founded: i32,
    /// Year abandoned.
    pub abandoned: i32,
    /// Why (`plague`, `flood`, `war`, `famine`, `fire`, `mine_collapse`, `well_failed`, `raiders`, `enclosure`).
    pub cause: String,
    /// Event that emptied it, if any.
    pub cause_event: Option<u32>,
    /// Abandonment event id.
    pub event: u32,
}

/// The realm a settlement first belonged to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Allegiance {
    /// Settlement.
    #[serde(with = "crate::ids::str")]
    pub settlement: u64,
    /// Realm it swore to at founding (or at its realm's formation).
    #[serde(with = "crate::ids::str")]
    pub first_realm: u64,
}

/// The whole history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct History {
    /// Era name.
    pub era: String,
    /// Era abbreviation for dates.
    pub era_abbrev: String,
    /// First year.
    pub start_year: i32,
    /// Present year.
    pub present_year: i32,
    /// Timeline, in id (= chronological) order.
    pub events: Vec<Event>,
    /// Ruling houses.
    pub dynasties: Vec<Dynasty>,
    /// Reigns, per realm in order.
    pub reigns: Vec<Reign>,
    /// Wars.
    pub wars: Vec<War>,
    /// Border shifts.
    pub border_shifts: Vec<BorderShift>,
    /// Ruins.
    pub ruins: Vec<Ruin>,
    /// First allegiance of every settlement; replaying `border_shifts` on it
    /// gives the present partition ([`History::replay_partition`]).
    pub allegiances: Vec<Allegiance>,
}

impl History {
    /// Replays the border shifts in year order from the first allegiances
    /// and returns the resulting realm of every settlement, or the first
    /// shift whose `from_realm` disagrees with the replayed state.
    ///
    /// # Errors
    /// The inconsistent [`BorderShift`].
    pub fn replay_partition(&self) -> Result<std::collections::BTreeMap<u64, u64>, BorderShift> {
        let mut state: std::collections::BTreeMap<u64, u64> = self
            .allegiances
            .iter()
            .map(|a| (a.settlement, a.first_realm))
            .collect();
        let mut shifts = self.border_shifts.clone();
        shifts.sort_by_key(|b| (b.year, b.settlement));
        for b in shifts {
            match state.get_mut(&b.settlement) {
                Some(r) if *r == b.from_realm => *r = b.to_realm,
                _ => return Err(b),
            }
        }
        Ok(state)
    }
}

/// Growing event list with provisional ids, renumbered chronologically at
/// the end so ids follow timeline order.
#[derive(Debug, Default)]
pub struct Timeline {
    /// Events so far; `id` is provisional (the push index).
    pub events: Vec<Event>,
}

impl Timeline {
    /// Adds an event and returns its provisional id.
    pub fn push(&mut self, mut e: Event) -> u32 {
        let id = u32::try_from(self.events.len()).unwrap_or(u32::MAX);
        e.id = id;
        self.events.push(e);
        id
    }

    /// Sorts chronologically and returns the provisional → final id map.
    pub fn renumber(&mut self) -> Vec<u32> {
        let mut order: Vec<usize> = (0..self.events.len()).collect();
        order.sort_by(|&a, &b| {
            let (x, y) = (&self.events[a], &self.events[b]);
            (x.year, x.kind, x.settlements.first(), x.realms.first(), a).cmp(&(
                y.year,
                y.kind,
                y.settlements.first(),
                y.realms.first(),
                b,
            ))
        });
        let mut map = vec![0_u32; self.events.len()];
        for (new, &old) in order.iter().enumerate() {
            map[old] = u32::try_from(new + 1).unwrap_or(u32::MAX);
        }
        let mut sorted: Vec<Event> = order.iter().map(|&o| self.events[o].clone()).collect();
        for e in &mut sorted {
            e.id = remap(&map, e.id);
            e.cause = e.cause.map(|c| remap(&map, c));
            for r in &mut e.refs {
                if let EntityRef::Event { id } = r {
                    *id = remap(&map, *id);
                }
            }
        }
        self.events = sorted;
        map
    }
}

/// Final id of provisional id `old`.
#[must_use]
pub fn remap(map: &[u32], old: u32) -> u32 {
    map.get(crate::num::usize_of(u64::from(old)))
        .copied()
        .unwrap_or(0)
}

/// An event skeleton with defaults for the optional fields.
#[must_use]
pub fn event(kind: EventKind, year: i32, title: String, text: String) -> Event {
    Event {
        id: 0,
        year,
        end_year: None,
        ongoing: false,
        kind,
        title,
        text,
        settlements: Vec::new(),
        realms: Vec::new(),
        refs: Vec::new(),
        cause: None,
        severity: 1,
    }
}
