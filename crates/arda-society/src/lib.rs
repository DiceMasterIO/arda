//! # arda-society
//!
//! The society layer above individual NPCs: realm politics, economy,
//! factions, a simulated history whose outcome is the present, campaign
//! hooks and the notable NPC slots the NPC generator fills (goal-prompt
//! goals 36, 38 and 51–57; `docs/capstone/logic/06-society-generation.md`).
//!
//! Entry point: [`simulate_society`]. Input records mirror the settlement
//! stage's `society/*.json` field for field; see [`input`].
//!
//! All generated text is original, written from the templates in `data/`;
//! stat-block and class names are SRD 5.1 (CC-BY-4.0).

#![deny(unsafe_code)]
#![deny(missing_docs)]

pub mod buildings;
pub mod ctx;
pub mod economy;
pub mod error;
pub mod gazetteer;
pub mod graph;
pub mod history;
pub mod hooks;
pub mod ids;
pub mod input;
pub mod names;
pub mod num;
pub mod output;
pub mod politics;
pub mod read;
pub mod refs;
pub mod rng;
pub mod roles;
pub mod synthetic;
pub mod tables;
pub mod text;

use buildings::BuildingRef;
use ctx::Ctx;
use economy::{Economy, SettlementEconomy};
use history::lore::HistoryHook;
use history::prosperity::Prosperity;
use history::History;
use hooks::{Hook, HookWorld};
use politics::factions::{Faction, FactionRelation, Local};
use politics::realm::RealmState;
use politics::relations::Relation;
use roles::NpcRole;
use serde::{Deserialize, Serialize};

pub use error::SocietyError;
pub use input::WorldSettlements;

/// Output format version.
pub const FORMAT_VERSION: u32 = 1;

/// Society of one settlement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementSociety {
    /// Settlement id.
    #[serde(with = "crate::ids::str")]
    pub id: u64,
    /// Name.
    pub name: String,
    /// Realm today.
    #[serde(with = "crate::ids::str")]
    pub realm_id: u64,
    /// Year founded.
    pub founded: i32,
    /// Settlement whose settlers founded it.
    #[serde(default, with = "crate::ids::opt")]
    pub founded_from: Option<u64>,
    /// Founding event id.
    pub founding_event: u32,
    /// Buildings with ids.
    pub buildings: Vec<BuildingRef>,
    /// Economy.
    pub economy: SettlementEconomy,
    /// Prosperity over time.
    pub prosperity: Prosperity,
    /// Factions.
    pub factions: Vec<Faction>,
    /// Stances between its factions.
    pub faction_relations: Vec<FactionRelation>,
    /// Notable NPC slots.
    pub roles: Vec<NpcRole>,
    /// 2–4 lines of history.
    pub history_hooks: Vec<HistoryHook>,
    /// 3–5 plot hooks.
    pub hooks: Vec<Hook>,
}

/// A realm today with its region-level plot hooks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealmSociety {
    /// State.
    #[serde(flatten)]
    pub state: RealmState,
    /// 3–5 plot hooks.
    pub hooks: Vec<Hook>,
}

/// The whole society layer of a world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Society {
    /// Output format version.
    pub format_version: u32,
    /// World seed.
    #[serde(with = "crate::ids::str")]
    pub seed: u64,
    /// Realms, by id.
    pub realms: Vec<RealmSociety>,
    /// Relations, one per realm pair.
    pub relations: Vec<Relation>,
    /// Settlements, in input order.
    pub settlements: Vec<SettlementSociety>,
    /// World economy.
    pub economy: Economy,
    /// History.
    pub history: History,
}

/// Simulates the society of `world` for `seed`. Deterministic: the same
/// inputs give byte-identical serialised output.
///
/// # Errors
/// [`SocietyError::Input`] for inconsistent input, [`SocietyError::Table`]
/// or [`SocietyError::TableContent`] if an embedded table is broken.
pub fn simulate_society(seed: u64, world: &WorldSettlements) -> Result<Society, SocietyError> {
    let tables = tables::Tables::load()?;
    let ctx = Ctx::new(seed, world, &tables)?;
    let econ = economy::run(&ctx);
    let regimes = politics::government::choose(&ctx, &econ);
    let hist = history::run::run(&ctx, &econ, &regimes);
    let states = politics::realm::realms(&ctx, &regimes, &hist, &econ);
    let hereditary: Vec<bool> = regimes
        .iter()
        .map(|r| {
            tables
                .politics
                .governments
                .get(&r.government)
                .is_none_or(|g| g.hereditary)
        })
        .collect();
    let relations = politics::relations::relations(&ctx, &hist, &econ, &hereditary);
    let mut factions = Vec::with_capacity(ctx.n());
    let mut faction_rel = Vec::with_capacity(ctx.n());
    let mut roles = Vec::with_capacity(ctx.n());
    let mut seen_names = std::collections::BTreeSet::new();
    for i in 0..ctx.n() {
        let st = &states[ctx.realm_of(i)];
        let sid = ctx.s(i).id;
        let lord_house = if ctx.is_seat(i) {
            Some(st.ruler.house.clone())
        } else {
            st.vassals
                .iter()
                .find(|v| v.settlement == sid)
                .map(|v| v.house.clone())
        };
        let local = Local {
            lord_house,
            tax_pct: st.tax_pct,
            history: &hist.history,
        };
        let (mut f, r) = politics::factions::factions(&ctx, i, &econ, &local, &mut seen_names);
        let rs = roles::roles(&ctx, i, &f, st);
        for fac in &mut f {
            // A faction meets where its leader works.
            if let Some(b) = rs
                .iter()
                .find(|x| x.id == fac.leader_role)
                .and_then(|x| x.building)
            {
                fac.seat_building = Some(b);
            }
        }
        roles.push(rs);
        factions.push(f);
        faction_rel.push(r);
    }
    let w = HookWorld {
        history: &hist.history,
        econ: &econ,
        realms: &states,
        relations: &relations,
        factions: &factions,
        faction_rel: &faction_rel,
        roles: &roles,
    };
    let settlement_hooks: Vec<Vec<Hook>> = (0..ctx.n())
        .map(|i| hooks::settlement::hooks(&ctx, &w, i))
        .collect();
    let realms: Vec<RealmSociety> = (0..ctx.realms.len())
        .map(|ri| RealmSociety {
            hooks: hooks::region::hooks(&ctx, &w, ri),
            state: states[ri].clone(),
        })
        .collect();
    let mut settlements = Vec::with_capacity(ctx.n());
    let parts = factions
        .into_iter()
        .zip(faction_rel)
        .zip(roles)
        .zip(settlement_hooks)
        .zip(econ.per_node.iter().cloned())
        .zip(hist.prosperity.iter().cloned())
        .zip(hist.lore.iter().cloned());
    for (i, ((((((f, fr), r), hk), ec), pr), lore)) in parts.enumerate() {
        let s = ctx.s(i);
        settlements.push(SettlementSociety {
            id: s.id,
            name: s.name.clone(),
            realm_id: s.realm_id,
            founded: hist.founding.year[i],
            founded_from: hist.founding.parent[i].map(|p| ctx.s(p).id),
            founding_event: hist.founding.event[i],
            buildings: ctx.buildings_of(i).to_vec(),
            economy: ec,
            prosperity: pr,
            factions: f,
            faction_relations: fr,
            roles: r,
            history_hooks: lore,
            hooks: hk,
        });
    }
    Ok(Society {
        format_version: FORMAT_VERSION,
        seed,
        realms,
        relations,
        settlements,
        economy: econ.economy,
        history: hist.history,
    })
}
