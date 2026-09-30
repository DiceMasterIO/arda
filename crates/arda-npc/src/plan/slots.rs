//! Binding the society layer's notable slots to inhabitants (logic/14
//! §soc-offices, logic/13 §npc-notables; adapter A14).
//!
//! Slots bind in id order. A slot with a building takes that building's
//! master, else another of its workers, else a mature resident; a slot
//! without one (or whose building has nobody) takes an unbound notable,
//! else a mature household head, in skeleton order. The holder becomes a
//! stored notable, works at the slot's building, and takes the slot's
//! title, name and sex; a holder who heads a household passes the family
//! name to it.

use super::{Plan, Role};
use crate::input::NotableSlot;
use crate::npc::Sex;

impl Plan<'_> {
    /// Binds every slot it can; `self.bound[person]` names the slot.
    pub(super) fn bind_slots(&mut self, slots: &[NotableSlot]) {
        let mut order: Vec<usize> = (0..slots.len()).collect();
        order.sort_by(|&a, &b| slots[a].id.cmp(&slots[b].id).then(a.cmp(&b)));
        self.bound = vec![None; self.people.len()];
        for k in order {
            let slot = &slots[k];
            let building = slot
                .building
                .and_then(|id| self.buildings.iter().position(|b| b.id == id));
            let Some(p) = building
                .and_then(|b| self.holder_in(b))
                .or_else(|| self.holder_anywhere())
            else {
                continue;
            };
            self.bound[p] = Some(k);
            let person = &mut self.people[p];
            person.notable = true;
            if building.is_some() {
                person.workplace = building;
            }
            if let Some(female) = slot.female {
                person.sex = if female { Sex::Female } else { Sex::Male };
            }
        }
    }

    fn free(&self, p: usize) -> bool {
        self.bound.get(p).is_some_and(Option::is_none)
    }

    fn holder_in(&self, b: usize) -> Option<usize> {
        let staff = self.slots.get(b)?;
        staff
            .iter()
            .flatten()
            .copied()
            .find(|&p| self.free(p))
            .or_else(|| {
                (0..self.people.len())
                    .find(|&p| self.people[p].building == b && self.free(p) && self.mature(p))
            })
    }

    fn holder_anywhere(&self) -> Option<usize> {
        (0..self.people.len())
            .find(|&p| self.people[p].notable && self.free(p))
            .or_else(|| {
                (0..self.people.len())
                    .find(|&p| self.free(p) && self.people[p].role == Role::Head && self.mature(p))
            })
    }

    /// The slot bound to `person`, if any.
    pub(crate) fn slot_of<'s>(
        &self,
        person: usize,
        slots: &'s [NotableSlot],
    ) -> Option<&'s NotableSlot> {
        self.bound
            .get(person)
            .copied()
            .flatten()
            .and_then(|k| slots.get(k))
    }
}
