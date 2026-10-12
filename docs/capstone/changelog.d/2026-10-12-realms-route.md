# 2026-10-12 — GET /v1/realms

- **arda-settle**: `realms::Realm` derives `JsonSchema` under the `schema` feature (id fields as strings).
- **arda-society**: schema derives on `RealmState`, `RealmSociety`, `Ruler`, `Vassal`, `Relation`, `Stance`, `Event`, `EventKind` and `Reign`; attribute lines only.
- **arda-people**: `SocietyFiles.realms: Option<RealmsFile>` (`None` when `realms.json` is absent) and `SocietyFiles::realm(id)`.
- **arda-server**: `GET /v1/realms` (summaries without borders, plus style, rank, government and ruler when `society.json` exists) and `GET /v1/realms/{id}` (record with borders, politics, relations, events, reigns), in `crates/arda-server/src/realms.rs`; schemas `Realm`, `RealmList`, `RealmDetail`, `RealmSociety`, `Relation`, `Event`.
- **Docs**: `API.md` Realms subsection, logic/16 routes, 09-interfaces, 02-models; OI-46 closed.

**Why**: DiceMaster saw only bare `realm_id` strings and could not label settlements or show who rules a realm.

**Tests**: `tests/world_api/realms.rs` (list vs `realms.json`, politics vs realms, detail of every realm, 404 and 400, identical bytes, no-society 404), `realm_bodies_validate` in `schemas.rs`, `arda-people` files unit tests.
