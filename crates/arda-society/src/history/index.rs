//! Lookups over a finished [`History`], built once (review round 2 #29).
//!
//! Hooks, lore and relations asked "the events of settlement `s`" or "the
//! wars of realm `r`" by scanning every record per settlement or realm
//! pair, which is quadratic at world scale. Every list here keeps the
//! history's own record order, so a scan over it finds exactly what the full
//! scan found.

use super::{Event, History, Ruin, War};
use std::collections::BTreeMap;

/// Record indices of a [`History`] by settlement, realm and road.
#[derive(Debug, Clone, Default)]
pub struct HistoryIndex {
    events_by_settlement: BTreeMap<u64, Vec<usize>>,
    events_by_realm: BTreeMap<u64, Vec<usize>>,
    wars_by_realm: BTreeMap<u64, Vec<usize>>,
    wars_by_pair: BTreeMap<(u64, u64), Vec<usize>>,
    shifts_by_pair: BTreeMap<(u64, u64), usize>,
    first_shift: BTreeMap<u64, usize>,
    shifts_by_settlement: BTreeMap<u64, Vec<usize>>,
    first_ruin_near: BTreeMap<u64, usize>,
    first_ruin_on_road: BTreeMap<u64, usize>,
    first_war_by_event: BTreeMap<u32, usize>,
}

/// Pushes `i` once per distinct key of one record.
fn push_each(map: &mut BTreeMap<u64, Vec<usize>>, keys: &[u64], i: usize) {
    for (k, key) in keys.iter().enumerate() {
        if !keys[..k].contains(key) {
            map.entry(*key).or_default().push(i);
        }
    }
}

const fn pair(x: u64, y: u64) -> (u64, u64) {
    if x <= y {
        (x, y)
    } else {
        (y, x)
    }
}

impl HistoryIndex {
    /// Indexes `h`.
    #[must_use]
    pub fn new(h: &History) -> Self {
        let mut ix = Self::default();
        for (i, e) in h.events.iter().enumerate() {
            push_each(&mut ix.events_by_settlement, &e.settlements, i);
            push_each(&mut ix.events_by_realm, &e.realms, i);
        }
        for (i, w) in h.wars.iter().enumerate() {
            ix.first_war_by_event.entry(w.event).or_insert(i);
            push_each(&mut ix.wars_by_realm, &w.realms, i);
            ix.wars_by_pair
                .entry(pair(w.realms[0], w.realms[1]))
                .or_default()
                .push(i);
        }
        for (i, b) in h.border_shifts.iter().enumerate() {
            ix.first_shift.entry(b.settlement).or_insert(i);
            ix.shifts_by_settlement
                .entry(b.settlement)
                .or_default()
                .push(i);
            *ix.shifts_by_pair
                .entry(pair(b.from_realm, b.to_realm))
                .or_default() += 1;
        }
        for (i, r) in h.ruins.iter().enumerate() {
            ix.first_ruin_near.entry(r.near).or_insert(i);
            if let Some(road) = r.road {
                ix.first_ruin_on_road.entry(road).or_insert(i);
            }
        }
        ix
    }

    /// Events naming settlement `id`, in history order.
    pub fn events_of<'h>(
        &self,
        h: &'h History,
        id: u64,
    ) -> impl Iterator<Item = &'h Event> + use<'_, 'h> {
        Self::pick(&self.events_by_settlement, &h.events, id)
    }

    /// Events naming realm `id`, in history order.
    pub fn realm_events<'h>(
        &self,
        h: &'h History,
        id: u64,
    ) -> impl Iterator<Item = &'h Event> + use<'_, 'h> {
        Self::pick(&self.events_by_realm, &h.events, id)
    }

    /// Wars realm `id` fought, in history order.
    pub fn wars_of<'h>(
        &self,
        h: &'h History,
        id: u64,
    ) -> impl Iterator<Item = &'h War> + use<'_, 'h> {
        Self::pick(&self.wars_by_realm, &h.wars, id)
    }

    /// Wars between realms `x` and `y` (either order), in history order.
    pub fn wars_between<'h>(
        &self,
        h: &'h History,
        x: u64,
        y: u64,
    ) -> impl Iterator<Item = &'h War> + use<'_, 'h> {
        self.wars_by_pair
            .get(&pair(x, y))
            .into_iter()
            .flatten()
            .filter_map(|&i| h.wars.get(i))
    }

    /// The first war recorded under event `id`.
    #[must_use]
    pub fn war_of_event<'h>(&self, h: &'h History, id: u32) -> Option<&'h War> {
        self.first_war_by_event
            .get(&id)
            .and_then(|&i| h.wars.get(i))
    }

    /// How many border shifts moved settlements between `x` and `y`.
    #[must_use]
    pub fn shifts_between(&self, x: u64, y: u64) -> usize {
        self.shifts_by_pair.get(&pair(x, y)).copied().unwrap_or(0)
    }

    /// The first border shift of settlement `id`.
    #[must_use]
    pub fn first_shift<'h>(&self, h: &'h History, id: u64) -> Option<&'h super::BorderShift> {
        self.first_shift
            .get(&id)
            .and_then(|&i| h.border_shifts.get(i))
    }

    /// Border shifts of settlement `id`, in history order.
    pub fn shifts_of<'h>(
        &self,
        h: &'h History,
        id: u64,
    ) -> impl Iterator<Item = &'h super::BorderShift> + use<'_, 'h> {
        Self::pick(&self.shifts_by_settlement, &h.border_shifts, id)
    }

    /// The first ruin near settlement `id`.
    #[must_use]
    pub fn ruin_near<'h>(&self, h: &'h History, id: u64) -> Option<&'h Ruin> {
        self.first_ruin_near.get(&id).and_then(|&i| h.ruins.get(i))
    }

    /// The first ruin on road `id`.
    #[must_use]
    pub fn ruin_on_road<'h>(&self, h: &'h History, id: u64) -> Option<&'h Ruin> {
        self.first_ruin_on_road
            .get(&id)
            .and_then(|&i| h.ruins.get(i))
    }

    fn pick<'m, 'h, T>(
        map: &'m BTreeMap<u64, Vec<usize>>,
        list: &'h [T],
        id: u64,
    ) -> impl Iterator<Item = &'h T> + use<'m, 'h, T> {
        map.get(&id)
            .into_iter()
            .flatten()
            .filter_map(move |&i| list.get(i))
    }
}
