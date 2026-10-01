//! Campaign hooks built from real tensions (deliverable 4). Every tension a
//! settlement or realm has becomes a scored candidate; the strongest 3–5 of
//! distinct kinds are written up from `hooks.json`, each referencing the
//! entities it is about.

pub mod region;
pub mod settlement;

use crate::ctx::Ctx;
use crate::economy::EconomyRun;
use crate::history::index::HistoryIndex;
use crate::history::History;
use crate::politics::factions::{Faction, FactionRelation};
use crate::politics::realm::RealmState;
use crate::politics::relations::Relation;
use crate::refs::EntityRef;
use crate::rng::Stream;
use crate::roles::NpcRole;
use crate::text::Slots;
use serde::{Deserialize, Serialize};

/// Fewest hooks per settlement or realm.
pub const MIN_HOOKS: usize = 3;
/// Most hooks per settlement or realm.
pub const MAX_HOOKS: usize = 5;
/// Candidates scoring below this only fill up to [`MIN_HOOKS`].
const STRONG: i64 = 30;

/// One plot hook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Hook {
    /// Stable id (`h<settlement>.<kind>` or `hr<realm>.<kind>`).
    pub id: String,
    /// Kind key.
    pub kind: String,
    /// Title.
    pub title: String,
    /// Two or three sentences for the referee.
    pub text: String,
    /// How hot the tension is, 0–100.
    pub tension: u8,
    /// Entities involved.
    pub refs: Vec<EntityRef>,
}

/// A scored tension before it is written up.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// Score (higher first).
    pub score: i64,
    /// Hook kind key in `hooks.json`.
    pub kind: &'static str,
    /// Template slots.
    pub slots: Slots,
    /// References.
    pub refs: Vec<EntityRef>,
}

impl Candidate {
    /// A new candidate.
    #[must_use]
    pub fn new(score: i64, kind: &'static str, slots: Slots, refs: Vec<EntityRef>) -> Self {
        Self {
            score,
            kind,
            slots,
            refs,
        }
    }
}

/// Lookups every hook shares, built once (review round 2 #29: a linear
/// scan per road, relation or realm made the hooks quadratic).
#[derive(Debug, Clone, Default)]
pub struct HookLookups {
    traffic: std::collections::BTreeMap<u64, u64>,
    relations_of: std::collections::BTreeMap<u64, Vec<usize>>,
    relation_at: std::collections::BTreeMap<(u64, u64), usize>,
}

impl HookLookups {
    /// Indexes road traffic and relations; the first record of a key wins,
    /// as the scans it replaces found it.
    #[must_use]
    pub fn new(econ: &EconomyRun, relations: &[Relation]) -> Self {
        let mut l = Self::default();
        for t in &econ.economy.road_traffic {
            l.traffic.entry(t.road).or_insert(t.value_sp);
        }
        for (k, r) in relations.iter().enumerate() {
            l.relations_of.entry(r.a).or_default().push(k);
            if r.b != r.a {
                l.relations_of.entry(r.b).or_default().push(k);
            }
            l.relation_at.entry((r.a, r.b)).or_insert(k);
        }
        l
    }
}

/// Everything hooks read.
#[derive(Debug, Clone, Copy)]
pub struct HookWorld<'a> {
    /// History.
    pub history: &'a History,
    /// Economy.
    pub econ: &'a EconomyRun,
    /// Realm states, [`Ctx::realms`] order.
    pub realms: &'a [RealmState],
    /// Relations.
    pub relations: &'a [Relation],
    /// Factions per node.
    pub factions: &'a [Vec<Faction>],
    /// Faction relations per node.
    pub faction_rel: &'a [Vec<FactionRelation>],
    /// Roles per node.
    pub roles: &'a [Vec<NpcRole>],
    /// Lookups over `history`.
    pub index: &'a HistoryIndex,
    /// Lookups over traffic and relations.
    pub lookups: &'a HookLookups,
}

impl<'a> HookWorld<'a> {
    /// Relations naming realm `id`, in list order.
    pub fn relations_of(&self, id: u64) -> impl Iterator<Item = &'a Relation> + use<'a> {
        let list = self.relations;
        self.lookups
            .relations_of
            .get(&id)
            .into_iter()
            .flatten()
            .filter_map(move |&k| list.get(k))
    }

    /// The relation between realms `x` and `y` in either order
    /// ([`crate::politics::relations::relation`], by index).
    #[must_use]
    pub fn relation(&self, x: u64, y: u64) -> Option<&'a Relation> {
        let key = (x.min(y), x.max(y));
        self.lookups
            .relation_at
            .get(&key)
            .and_then(|&k| self.relations.get(k))
    }

    /// Role of `kind` at node `i`, if the settlement has one.
    #[must_use]
    pub fn role<'r>(&'r self, i: usize, kind: &str) -> Option<&'r NpcRole> {
        self.roles.get(i)?.iter().find(|r| r.kind == kind)
    }

    /// The head role (ruler, lord, reeve or elder) of node `i`.
    #[must_use]
    pub fn head<'r>(&'r self, ctx: &Ctx<'_>, i: usize) -> Option<&'r NpcRole> {
        self.role(i, crate::roles::head_kind(ctx, i))
    }

    /// Traffic value on road `id`.
    #[must_use]
    pub fn traffic(&self, id: u64) -> u64 {
        self.lookups.traffic.get(&id).copied().unwrap_or(0)
    }
}

/// Picks and writes up the hooks from `cands`; `prefix` forms the ids.
#[must_use]
pub fn select(ctx: &Ctx<'_>, prefix: &str, key: u64, mut cands: Vec<Candidate>) -> Vec<Hook> {
    // Weak tensions only fill up to MIN_HOOKS; jitter their order per place
    // so the fillers vary from one settlement to the next.
    for c in &mut cands {
        if c.score < STRONG {
            let j = crate::rng::hash(ctx.seed, c.kind, key, 0) % 20;
            c.score = (c.score / 3 + crate::num::i64_of(j)).min(STRONG - 1);
        }
    }
    cands.sort_by(|a, b| b.score.cmp(&a.score).then(a.kind.cmp(b.kind)));
    let mut out: Vec<Hook> = Vec::new();
    let mut rng = Stream::new(
        ctx.seed,
        "hooks",
        key,
        crate::num::u64_of_usize(prefix.len()),
    );
    for c in cands {
        if out.len() >= MAX_HOOKS || (out.len() >= MIN_HOOKS && c.score < STRONG) {
            break;
        }
        if out.iter().any(|h| h.kind == c.kind) {
            continue;
        }
        let Some(def) = ctx.t.hooks.hooks.get(c.kind) else {
            continue;
        };
        let pair = rng.pick(&def.variants);
        let mut refs = c.refs;
        refs.sort();
        refs.dedup();
        out.push(Hook {
            id: format!("{prefix}.{}", c.kind),
            kind: c.kind.to_string(),
            title: pair.map_or_else(String::new, |p| c.slots.fill(&p[0])),
            text: pair.map_or_else(String::new, |p| c.slots.fill(&p[1])),
            tension: crate::num::u8_of(c.score.clamp(0, 100)),
            refs,
        });
    }
    out
}

/// Pushes a role reference (and its title into `slots` under `slot`).
pub fn cite_role(
    refs: &mut Vec<EntityRef>,
    slots: &mut Slots,
    slot: &'static str,
    role: Option<&NpcRole>,
) {
    if let Some(r) = role {
        refs.push(EntityRef::role(&r.id));
        slots.set(slot, r.title.clone());
    } else {
        slots.set(slot, "local headman".to_string());
    }
}
