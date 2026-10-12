//! Names, causes and backstories for the major events: shared helpers the
//! disaster, war, reign and ruin stages call as they push events.

use super::{Event, EventKind, Timeline};
use crate::ctx::Ctx;
use crate::refs::EntityRef;
use crate::rng::Stream;
use crate::text::{capitalise_first, event_phrase, lower_the, pick_fill, pick_name, Slots};

/// A name no earlier event bears: `name`, else with the year, else with an
/// ordinal. Names compare in their capitalised form.
#[must_use]
pub fn unique_name(tl: &Timeline, name: &str, year: i32) -> String {
    let base = capitalise_first(name);
    let taken = |n: &str| tl.events.iter().any(|e| e.name.as_deref() == Some(n));
    if !taken(&base) {
        return base;
    }
    let with_year = format!("{base} of {year}");
    if !base.contains(&year.to_string()) && !taken(&with_year) {
        return with_year;
    }
    ["Second", "Third", "Fourth", "Fifth", "Sixth"]
        .iter()
        .map(|o| format!("{base}, the {o}"))
        .find(|n| !taken(n))
        .unwrap_or(base)
}

/// The river a realm's floods and hungers are named for (stable per realm).
#[must_use]
pub fn river(ctx: &Ctx<'_>, realm_id: u64) -> String {
    let culture = ctx
        .realm_ix(realm_id)
        .map_or("", |r| ctx.s(ctx.realms[r].seat).culture.as_str());
    let mut rng = Stream::new(ctx.seed, "river", realm_id, 0);
    crate::names::river(&ctx.t.names, culture, &mut rng)
}

/// Gives `e` a unique name from the `event_names` family `key`.
pub fn name_event(
    ctx: &Ctx<'_>,
    tl: &Timeline,
    e: &mut Event,
    key: &str,
    slots: &Slots,
    rng: &mut Stream,
) {
    let list = ctx
        .t
        .history
        .event_names
        .get(key)
        .map_or(&[][..], Vec::as_slice);
    let raw = pick_name(list, rng, slots);
    if !raw.is_empty() {
        e.name = Some(unique_name(tl, &raw, e.year));
    }
}

/// Links `e` to its `cause` and writes its backstory from family `key`.
/// `slots` carries the event's own slots; the name, the cause's phrase and
/// the gap in years are added here.
pub fn tell(
    ctx: &Ctx<'_>,
    e: &mut Event,
    key: &str,
    cause: Option<&Event>,
    slots: &Slots,
    rng: &mut Stream,
) {
    let mut sl = slots.clone().with("name", event_phrase(e));
    if let Some(c) = cause {
        e.cause = Some(c.id);
        e.refs.push(EntityRef::Event { id: c.id });
        sl.set("cause", event_phrase(c));
        sl.set("cause_year", c.year.to_string());
        sl.set("years", (e.year - c.year).max(0).to_string());
    }
    let list = ctx
        .t
        .history
        .backstory
        .get(key)
        .map_or(&[][..], Vec::as_slice);
    let text = pick_fill(list, rng, &sl);
    if !text.is_empty() {
        e.backstory = Some(text);
    }
}

/// The plague name slot: "the Grey Cough" from the stored "The Grey Cough".
#[must_use]
pub fn plague_slot(name: &str) -> String {
    lower_the(name)
}

/// Whether `e` is a kind that can empty a ruin or start a chain.
#[must_use]
pub const fn is_disaster(k: EventKind) -> bool {
    matches!(
        k,
        EventKind::Plague
            | EventKind::Flood
            | EventKind::Fire
            | EventKind::Famine
            | EventKind::MineCollapse
    )
}
