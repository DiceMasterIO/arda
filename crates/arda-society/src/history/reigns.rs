//! Realm formation, ruling houses and reigns. A reign ends by death, battle
//! (only while its realm is at war), plague (only in a plague year at the
//! seat), deposition, abdication or a failed line; the last three and some
//! battle deaths bring a new house to power.

use super::founding::Founding;
use super::{event, Dynasty, EventKind, Reign, Timeline, War};
use crate::ctx::Ctx;
use crate::names;
use crate::num::{i32_of, u64_of_usize};
use crate::politics::government::Regime;
use crate::refs::EntityRef;
use crate::rng::Stream;
use crate::text::{pick_fill, roman, Slots};

/// Formation year per realm (in [`Ctx::realms`] order); pushes the events.
#[must_use]
pub fn form_realms(ctx: &Ctx<'_>, f: &Founding, tl: &mut Timeline) -> Vec<i32> {
    let t = &ctx.t.history;
    ctx.realms
        .iter()
        .map(|r| {
            let mut rng = Stream::new(ctx.seed, "realm_formed", r.id, 0);
            let seat_year = f.year[r.seat];
            let year = (seat_year + i32_of(rng.range(12, 40)))
                .min(ctx.present - 60)
                .max(seat_year);
            let joined: Vec<u64> = r
                .members
                .iter()
                .copied()
                .filter(|&i| f.year[i] <= year)
                .map(|i| ctx.s(i).id)
                .collect();
            let slots = Slots::new()
                .with("realm", r.name.clone())
                .with("settlement", ctx.s(r.seat).name.clone())
                .with("count", joined.len().saturating_sub(1).to_string());
            let list = |k: &str| t.events.get(k).map_or(&[][..], Vec::as_slice);
            let title = pick_fill(
                t.titles.get("realm_formed").map_or(&[][..], Vec::as_slice),
                &mut rng,
                &slots,
            );
            let mut e = event(
                EventKind::RealmFormed,
                year,
                title,
                pick_fill(list("realm_formed"), &mut rng, &slots),
            );
            e.realms.push(r.id);
            e.settlements = joined;
            e.severity = 2;
            tl.push(e);
            year
        })
        .collect()
}

/// Dynasties and reigns for every realm; pushes accession and dynasty events.
#[must_use]
pub fn reigns(
    ctx: &Ctx<'_>,
    regimes: &[Regime],
    formed: &[i32],
    wars: &[War],
    tl: &mut Timeline,
) -> (Vec<Dynasty>, Vec<Reign>) {
    let mut dynasties = Vec::new();
    let mut reigns = Vec::new();
    let plague_years: Vec<(i32, i32, Vec<u64>)> = tl
        .events
        .iter()
        .filter(|e| e.kind == EventKind::Plague)
        .map(|e| (e.year, e.end_year.unwrap_or(e.year), e.settlements.clone()))
        .collect();
    for (ri, r) in ctx.realms.iter().enumerate() {
        let seat = ctx.s(r.seat);
        let culture = seat.culture.as_str();
        let gov = ctx.t.politics.governments.get(&regimes[ri].government);
        let hereditary = gov.is_none_or(|g| g.hereditary);
        let titles = gov.and_then(|g| g.titles.get(&regimes[ri].rank)).cloned();
        let mut rng = Stream::new(ctx.seed, "reigns", r.id, 0);
        let mut year = formed[ri];
        let mut house_n = 0_usize;
        let mut house = new_house(
            ctx,
            r.id,
            culture,
            house_n,
            year,
            "founding",
            &mut dynasties,
        );
        push_dynasty_event(ctx, tl, r, &house, year, None);
        let first_reign = reigns.len();
        loop {
            let female = rng.chance(if hereditary { 330 } else { 400 });
            let given = names::given(&ctx.t.names, culture, female, &mut rng);
            let length = i32_of(rng.range(5, 34));
            let end = year + length;
            let title = titles
                .as_ref()
                .map_or_else(|| "Ruler".to_string(), |t| t[usize::from(female)].clone());
            let same = reigns[first_reign..]
                .iter()
                .filter(|x: &&Reign| x.given == given)
                .count();
            let mut regnal = if same > 0 {
                format!("{given} {}", roman(u32::try_from(same + 1).unwrap_or(1)))
            } else {
                given.clone()
            };
            if rng.chance(300) {
                if let Some(ep) = rng.pick(&ctx.t.history.epithets) {
                    regnal = format!("{regnal} {ep}");
                }
            }
            let slots = Slots::new()
                .with("ruler", regnal.clone())
                .with("title", title.clone())
                .with("realm", r.name.clone())
                .with("house", house.name.clone());
            let text = pick_fill(
                ctx.t
                    .history
                    .events
                    .get("accession")
                    .map_or(&[][..], Vec::as_slice),
                &mut rng,
                &slots,
            );
            let mut e = event(
                EventKind::Accession,
                year,
                format!("{title} {regnal} of {}", r.name),
                text,
            );
            e.realms.push(r.id);
            e.settlements.push(seat.id);
            e.refs.push(EntityRef::Dynasty {
                id: house.id.clone(),
            });
            tl.push(e);
            if end >= ctx.present {
                reigns.push(Reign {
                    realm_id: r.id,
                    dynasty_id: house.id.clone(),
                    given,
                    regnal,
                    female,
                    title,
                    from: year,
                    to: None,
                    end: None,
                });
                break;
            }
            let at_war = wars.iter().find(|w| {
                w.realms.contains(&r.id) && w.from <= end && w.to.is_none_or(|t| end <= t)
            });
            let plague = plague_years
                .iter()
                .any(|(a, b, s)| *a <= end && end <= *b && s.contains(&seat.id));
            let cause = if at_war.is_some() && rng.chance(300) {
                "fell"
            } else if plague && rng.chance(400) {
                "plague"
            } else {
                match rng.below(100) {
                    0..=69 => "died",
                    70..=79 => "deposed",
                    80..=86 => "abdicated",
                    _ => "heirless",
                }
            };
            reigns.push(Reign {
                realm_id: r.id,
                dynasty_id: house.id.clone(),
                given,
                regnal,
                female,
                title,
                from: year,
                to: Some(end),
                end: Some(cause.to_string()),
            });
            let lost = at_war.is_some_and(|w| w.winner.is_some_and(|win| win != r.id));
            let origin = match cause {
                "deposed" => Some("usurpation"),
                "heirless" => Some("succession"),
                "fell" if lost && rng.chance(500) => Some("conquest"),
                _ if !hereditary && rng.chance(550) => Some("election"),
                _ => None,
            };
            year = end;
            if let Some(origin) = origin {
                if let Some(d) = dynasties
                    .iter_mut()
                    .find(|d: &&mut Dynasty| d.id == house.id)
                {
                    d.to = Some(end);
                }
                house_n += 1;
                let cause_id = at_war.map(|w| w.event);
                house = new_house(ctx, r.id, culture, house_n, end, origin, &mut dynasties);
                push_dynasty_event(ctx, tl, r, &house, end, cause_id);
            }
        }
    }
    (dynasties, reigns)
}

fn new_house(
    ctx: &Ctx<'_>,
    realm: u64,
    culture: &str,
    n: usize,
    from: i32,
    origin: &str,
    out: &mut Vec<Dynasty>,
) -> Dynasty {
    let mut rng = Stream::new(ctx.seed, "house", realm, u64_of_usize(n));
    let mut name = names::house(&ctx.t.names, culture, &mut rng);
    while out.iter().any(|d| d.name == name) {
        name = names::house(&ctx.t.names, culture, &mut rng);
    }
    let d = Dynasty {
        id: format!("d{realm}.{}", n + 1),
        realm_id: realm,
        name,
        from,
        to: None,
        origin: origin.to_string(),
    };
    out.push(d.clone());
    d
}

fn push_dynasty_event(
    ctx: &Ctx<'_>,
    tl: &mut Timeline,
    r: &crate::ctx::RealmInfo,
    d: &Dynasty,
    year: i32,
    cause: Option<u32>,
) {
    let mut rng = Stream::new(
        ctx.seed,
        "dynasty_text",
        r.id,
        crate::num::u64_of(i64::from(year)),
    );
    let slots = Slots::new()
        .with("house", d.name.clone())
        .with("realm", r.name.clone());
    let key = format!("dynasty_{}", d.origin);
    let text = pick_fill(
        ctx.t
            .history
            .events
            .get(&key)
            .map_or(&[][..], Vec::as_slice),
        &mut rng,
        &slots,
    );
    let mut e = event(
        EventKind::DynastyFounded,
        year,
        format!("House {} rises in {}", d.name, r.name),
        text,
    );
    e.realms.push(r.id);
    e.settlements.push(ctx.s(r.seat).id);
    e.refs.push(EntityRef::Dynasty { id: d.id.clone() });
    e.cause = cause;
    tl.push(e);
}
