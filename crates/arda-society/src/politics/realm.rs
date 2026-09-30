//! Present-day realm state: ruler (the last reign of the history), vassal
//! lords of every other town, and their loyalty as history left it.

use super::government::Regime;
use crate::ctx::Ctx;
use crate::economy::EconomyRun;
use crate::history::run::HistoryRun;
use crate::names;
use crate::num::{i64_of, u8_of};
use crate::rng::Stream;
use crate::roles::{head_kind, role_id};
use crate::text::Slots;
use serde::{Deserialize, Serialize};

/// A conquest is still a live grievance for this many years.
pub const RECENT_CONQUEST: i32 = 100;

/// The present ruler of a realm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ruler {
    /// NPC slot at the seat.
    pub role: String,
    /// Given name.
    pub given: String,
    /// Regnal style.
    pub regnal: String,
    /// Title.
    pub title: String,
    /// Female ruler.
    pub female: bool,
    /// Ruling house id.
    pub dynasty: String,
    /// House name.
    pub house: String,
    /// Reigning since.
    pub since: i32,
}

/// A vassal lord.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vassal {
    /// Settlement held.
    #[serde(with = "crate::ids::str")]
    pub settlement: u64,
    /// NPC slot of the lord.
    pub role: String,
    /// Title.
    pub title: String,
    /// Suggested given name.
    pub given: String,
    /// Female lord.
    pub female: bool,
    /// House name.
    pub house: String,
    /// Loyalty to the ruler, 0–100.
    pub loyalty: u8,
    /// Grievances against the crown.
    pub grievances: Vec<String>,
    /// Whether the house once held the throne and could claim it again.
    pub claimant: bool,
}

/// One realm today.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealmState {
    /// Realm id.
    #[serde(with = "crate::ids::str")]
    pub id: u64,
    /// Realm name.
    pub name: String,
    /// Formal style, e.g. "the Duchy of Veld".
    pub style: String,
    /// Government key.
    pub government: String,
    /// Readable government.
    pub government_label: String,
    /// `kingdom`, `duchy` or `county`.
    pub rank: String,
    /// Seat settlement.
    #[serde(with = "crate::ids::str")]
    pub seat: u64,
    /// Year formed.
    pub founded: i32,
    /// Member settlements.
    #[serde(with = "crate::ids::vec")]
    pub members: Vec<u64>,
    /// Total population.
    pub population: u64,
    /// Culture of the seat.
    pub culture: String,
    /// Levy on trade, percent.
    pub tax_pct: u32,
    /// Yearly levy on the realm's trade, silver pieces.
    pub levy_sp: u64,
    /// Present ruler.
    pub ruler: Ruler,
    /// Vassal lords.
    pub vassals: Vec<Vassal>,
}

/// Realm states in [`Ctx::realms`] order.
#[must_use]
pub fn realms(
    ctx: &Ctx<'_>,
    regimes: &[Regime],
    hist: &HistoryRun,
    econ: &EconomyRun,
) -> Vec<RealmState> {
    let h = &hist.history;
    ctx.realms
        .iter()
        .enumerate()
        .map(|(ri, r)| {
            let seat = ctx.s(r.seat);
            let gov = ctx.t.politics.governments.get(&regimes[ri].government);
            let style_t = gov
                .and_then(|g| g.style.get(&regimes[ri].rank))
                .map_or("{name}", String::as_str);
            let reign = h.reigns.iter().rev().find(|x| x.realm_id == r.id);
            let dynasty = reign.and_then(|x| h.dynasties.iter().find(|d| d.id == x.dynasty_id));
            let ruler = Ruler {
                role: role_id(seat.id, "ruler"),
                given: reign.map_or_else(String::new, |x| x.given.clone()),
                regnal: reign.map_or_else(String::new, |x| x.regnal.clone()),
                title: reign.map_or_else(String::new, |x| x.title.clone()),
                female: reign.is_some_and(|x| x.female),
                dynasty: dynasty.map_or_else(String::new, |d| d.id.clone()),
                house: dynasty.map_or_else(String::new, |d| d.name.clone()),
                since: reign.map_or(hist.formed[ri], |x| x.from),
            };
            let members: Vec<u64> = r.members.iter().map(|&i| ctx.s(i).id).collect();
            let traded: u64 = econ
                .economy
                .flows
                .iter()
                .filter(|f| members.contains(&f.from) || members.contains(&f.to))
                .map(|f| f.value_sp)
                .sum();
            let tax_pct = gov.map_or(10, |g| g.tax_pct);
            RealmState {
                id: r.id,
                name: r.name.clone(),
                style: Slots::new().with("name", r.name.clone()).fill(style_t),
                government: regimes[ri].government.clone(),
                government_label: gov.map_or_else(String::new, |g| g.label.clone()),
                rank: regimes[ri].rank.clone(),
                seat: seat.id,
                founded: hist.formed[ri],
                population: r
                    .members
                    .iter()
                    .map(|&i| u64::from(ctx.s(i).population))
                    .sum(),
                culture: seat.culture.clone(),
                tax_pct,
                levy_sp: traded * u64::from(tax_pct) / 100,
                vassals: vassals(ctx, ri, regimes, hist, econ),
                members,
                ruler,
            }
        })
        .collect()
}

fn vassals(
    ctx: &Ctx<'_>,
    ri: usize,
    regimes: &[Regime],
    hist: &HistoryRun,
    econ: &EconomyRun,
) -> Vec<Vassal> {
    let r = &ctx.realms[ri];
    let h = &hist.history;
    let p = &ctx.t.politics;
    let gov = p.governments.get(&regimes[ri].government);
    let monarchy = regimes[ri].government == "monarchy";
    let seat_id = ctx.s(r.seat).id;
    let ousted = h
        .dynasties
        .iter()
        .filter(|d| d.realm_id == r.id && d.to.is_some_and(|t| ctx.present - t <= 150))
        .max_by_key(|d| d.to);
    let mut holders: Vec<usize> = r
        .members
        .iter()
        .copied()
        .filter(|&i| i != r.seat && head_kind(ctx, i) == "lord")
        .collect();
    holders.sort_by_key(|&i| (std::cmp::Reverse(ctx.s(i).population), i));
    let pick = crate::rng::hash(ctx.seed, "claimant", r.id, 0);
    // Half the realms harbour an ousted house among their vassals.
    let claimant_at = if pick.is_multiple_of(2) && !holders.is_empty() {
        crate::num::usize_of(pick / 2 % crate::num::u64_of_usize(holders.len()))
    } else {
        usize::MAX
    };
    holders
        .iter()
        .enumerate()
        .map(|(k, &i)| {
            let s = ctx.s(i);
            let mut rng = Stream::new(ctx.seed, "vassal", s.id, 0);
            let female = rng.chance(300);
            let fx = usize::from(female);
            let key = if s.has(crate::input::Function::Fortress) {
                Some("fortress")
            } else if monarchy && ctx.is_border(i) && s.tier.is_urban() {
                Some("border")
            } else if !s.tier.is_urban() {
                Some("manor")
            } else {
                None
            };
            let title = key
                .and_then(|k| p.heads.get(k))
                .or_else(|| gov.map(|g| &g.vassal))
                .map_or_else(|| "Lord".to_string(), |t| t[fx].clone());
            let mut house = names::house(&ctx.t.names, &s.culture, &mut rng);
            let mut grievances = Vec::new();
            let mut claimant = false;
            let mut loyalty = 70 + rng.range(-8, 8);
            let km = i64_of(ctx.dist(i, r.seat) / 1000);
            loyalty -= (km / 4).min(30);
            let with_seat: u64 = econ
                .economy
                .flows
                .iter()
                .filter(|f| {
                    (f.from == s.id && f.to == seat_id) || (f.to == s.id && f.from == seat_id)
                })
                .map(|f| f.value_sp)
                .sum();
            loyalty += i64_of(with_seat / 2000).min(15);
            let sl = Slots::new()
                .with("settlement", s.name.clone())
                .with("realm", r.name.clone());
            let recent_shift = h
                .border_shifts
                .iter()
                .find(|b| b.settlement == s.id && ctx.present - b.year <= RECENT_CONQUEST);
            if let Some(b) = recent_shift {
                loyalty -= (40 - i64::from(ctx.present - b.year) / 5).max(10);
                let former = ctx
                    .realm_ix(b.from_realm)
                    .map_or(String::new(), |x| ctx.realms[x].name.clone());
                let t = p.grievances.get("conquered").map_or("", String::as_str);
                grievances.push(
                    sl.clone()
                        .with("former", former)
                        .with("year", b.year.to_string())
                        .fill(t),
                );
            }
            if let (true, Some(d)) = (k == claimant_at, ousted) {
                house.clone_from(&d.name);
                claimant = true;
                loyalty -= 25;
                let t = p.grievances.get("old_blood").map_or("", String::as_str);
                let until = d.to.unwrap_or(0).to_string();
                grievances.push(
                    sl.clone()
                        .with("house", d.name.clone())
                        .with("year", until)
                        .fill(t),
                );
            }
            if km > 60 {
                let t = p.grievances.get("distance").map_or("", String::as_str);
                grievances.push(sl.clone().fill(t));
            }
            Vassal {
                settlement: s.id,
                role: role_id(s.id, "lord"),
                title,
                given: names::given(&ctx.t.names, &s.culture, female, &mut rng),
                female,
                house,
                loyalty: u8_of(loyalty.clamp(5, 95)),
                grievances,
                claimant,
            }
        })
        .collect()
}
