//! Friends, rivals and the full relationship list of a person.
//!
//! Friends and rivals are pairings: adults are shuffled with a settlement
//! seed and taken two at a time, so every tie is mutual by construction.
//! Pairs inside one household, or rivals who are already friends, are
//! dropped on both sides.

use crate::npc::{RelationKind, Relationship};
use crate::plan::Plan;

impl Plan<'_> {
    pub(super) fn pair_friends_and_rivals(&mut self) {
        let n = self.people.len();
        self.friend = vec![None; n];
        self.rival = vec![None; n];
        let adults: Vec<usize> = (0..n)
            .filter(|&p| self.people[p].age >= self.race(p).working_age)
            .collect();
        let key = crate::rng::SeedKey::settlement(self.seed, self.settlement.id);
        for (purpose, is_friend) in [("friends", true), ("rivals", false)] {
            let mut order = adults.clone();
            key.rng(purpose).shuffle(&mut order);
            for pair in order.as_chunks::<2>().0 {
                let (a, b) = (pair[0], pair[1]);
                if self.people[a].household == self.people[b].household {
                    continue;
                }
                if is_friend {
                    self.friend[a] = Some(b);
                    self.friend[b] = Some(a);
                } else if self.friend[a] != Some(b) {
                    self.rival[a] = Some(b);
                    self.rival[b] = Some(a);
                }
            }
        }
    }

    /// Employer of a person: the master of their workplace, for slot holders
    /// other than the master.
    pub(crate) fn employer(&self, person: usize) -> Option<usize> {
        let p = &self.people[person];
        match (p.workplace, p.slot) {
            (Some(b), Some(s)) if s > 0 => self.slots[b].first().copied().flatten(),
            _ => None,
        }
    }

    /// Every relationship of a person, sorted by kind, then by the other
    /// person's skeleton order (settlement, building id, index), so the order
    /// does not depend on the hashed public ids.
    pub(crate) fn relationships(&self, person: usize) -> Vec<Relationship> {
        let p = &self.people[person];
        let mut out: Vec<(RelationKind, usize)> = self.houses[p.household]
            .relations
            .iter()
            .filter(|&&(a, _, _)| a == person)
            .map(|&(_, kind, b)| (kind, b))
            .collect();
        if let Some(boss) = self.employer(person) {
            out.push((RelationKind::Employer, boss));
        }
        if let (Some(b), Some(0)) = (p.workplace, p.slot) {
            for &worker in self.slots[b].iter().skip(1).flatten() {
                out.push((RelationKind::Employee, worker));
            }
        }
        if let Some(f) = self.friend[person] {
            out.push((RelationKind::Friend, f));
        }
        if let Some(r) = self.rival[person] {
            out.push((RelationKind::Rival, r));
        }
        out.sort_unstable();
        out.dedup();
        out.into_iter()
            .map(|(kind, other)| Relationship {
                kind,
                other: self.id_of(other),
            })
            .collect()
    }
}
