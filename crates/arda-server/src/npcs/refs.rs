//! Wire forms of world people and buildings (logic/13 §npc-id, logic/16
//! §api-conventions):
//!
//! - an NPC reference `"<settlement>.<building>.<index>"` names the
//!   `index`-th resident of a building, notable or commoner, and is enough to
//!   regenerate them; the u64 [`NpcId`] is its hash
//!   (`NpcId::from_parts`), so it cannot be decoded without the settlement;
//! - a building reference `"<settlement>.<building>"` (building ids are
//!   unique only within their settlement);
//! - a page cursor `"<settlement>.<position>"`: where the next page of a
//!   query resumes, in settlement-id then skeleton order. Clients treat it
//!   as opaque.

use crate::error::{ServerError, ServerResult};
use arda_npc::{BuildingId, NpcId, SettlementId};
use std::fmt;

/// A resident by settlement, home building and index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NpcRef {
    /// The settlement.
    pub settlement: u64,
    /// The home building.
    pub building: u64,
    /// Index among the building's residents.
    pub index: u32,
}

impl NpcRef {
    /// The person's public id.
    #[must_use]
    pub fn id(self) -> NpcId {
        NpcId::from_parts(
            SettlementId(self.settlement),
            BuildingId(self.building),
            self.index,
        )
    }
}

impl fmt::Display for NpcRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.settlement, self.building, self.index)
    }
}

/// How an NPC path segment names someone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcKey {
    /// The u64 id (stored notables, or any inhabitant with `?settlement=`).
    Id(NpcId),
    /// The regenerable reference.
    Ref(NpcRef),
}

fn part<T: std::str::FromStr>(what: &str, text: &str, whole: &str) -> ServerResult<T> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ServerError::BadRequest(format!(
            "{what} {whole:?} is not in its documented decimal form"
        )));
    }
    text.parse()
        .map_err(|_| ServerError::BadRequest(format!("{what} {whole:?} is out of range")))
}

/// Parses `/v1/npc/{id}`: a decimal u64 id or `"<s>.<b>.<i>"`.
///
/// # Errors
/// [`ServerError::BadRequest`] for anything else.
pub fn parse_npc(text: &str) -> ServerResult<NpcKey> {
    let parts: Vec<&str> = text.split('.').collect();
    match parts.as_slice() {
        [id] => Ok(NpcKey::Id(NpcId(part("npc id", id, text)?))),
        [s, b, i] => Ok(NpcKey::Ref(NpcRef {
            settlement: part("npc ref", s, text)?,
            building: part("npc ref", b, text)?,
            index: part("npc ref", i, text)?,
        })),
        _ => Err(ServerError::BadRequest(format!(
            "npc {text:?} must be a decimal id or <settlement>.<building>.<index>"
        ))),
    }
}

/// Parses `/v1/buildings/{id}`: `"<s>.<b>"`, or a bare building id with
/// the settlement given separately.
///
/// # Errors
/// [`ServerError::BadRequest`] for malformed text or a missing settlement.
pub fn parse_building(text: &str, settlement: Option<u64>) -> ServerResult<(u64, u64)> {
    match text.split_once('.') {
        Some((s, b)) => {
            let s = part("building", s, text)?;
            if settlement.is_some_and(|q| q != s) {
                return Err(ServerError::BadRequest(format!(
                    "building {text:?} is not in the requested settlement"
                )));
            }
            Ok((s, part("building", b, text)?))
        }
        None => {
            let b = part("building", text, text)?;
            let s = settlement.ok_or_else(|| {
                ServerError::BadRequest(format!(
                    "building ids are unique only within a settlement: use \
                     <settlement>.{text} or ?settlement="
                ))
            })?;
            Ok((s, b))
        }
    }
}

/// Where a query page resumes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cursor {
    /// Settlement to resume in.
    pub settlement: u64,
    /// Skeleton position inside it.
    pub position: usize,
}

impl fmt::Display for Cursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.settlement, self.position)
    }
}

/// Parses a `?cursor=` value.
///
/// # Errors
/// [`ServerError::BadRequest`] for anything but a cursor this server issued.
pub fn parse_cursor(text: &str) -> ServerResult<Cursor> {
    let (s, p) = text.split_once('.').ok_or_else(|| {
        ServerError::BadRequest(format!("cursor {text:?} was not issued by this server"))
    })?;
    Ok(Cursor {
        settlement: part("cursor", s, text)?,
        position: part("cursor", p, text)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refs_round_trip_and_hash_to_the_npc_id() {
        let r = NpcRef {
            settlement: 118,
            building: 5,
            index: 2,
        };
        assert_eq!(r.to_string(), "118.5.2");
        assert_eq!(parse_npc("118.5.2").unwrap(), NpcKey::Ref(r));
        assert_eq!(
            r.id(),
            NpcId::from_parts(SettlementId(118), BuildingId(5), 2)
        );
        assert_eq!(parse_npc("42").unwrap(), NpcKey::Id(NpcId(42)));
        for bad in [
            "",
            "x",
            "1.2",
            "1.2.3.4",
            "1..3",
            "-1",
            "+1",
            "1.2.99999999999",
        ] {
            assert!(parse_npc(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn buildings_need_their_settlement() {
        assert_eq!(parse_building("118.5", None).unwrap(), (118, 5));
        assert_eq!(parse_building("5", Some(118)).unwrap(), (118, 5));
        assert_eq!(parse_building("118.5", Some(118)).unwrap(), (118, 5));
        assert!(parse_building("5", None).is_err());
        assert!(parse_building("118.5", Some(7)).is_err());
        assert!(parse_building("118.x", None).is_err());
    }

    #[test]
    fn cursors_round_trip() {
        let c = Cursor {
            settlement: 7,
            position: 250,
        };
        assert_eq!(parse_cursor(&c.to_string()).unwrap(), c);
        assert!(parse_cursor("7").is_err() && parse_cursor("a.1").is_err());
    }
}
