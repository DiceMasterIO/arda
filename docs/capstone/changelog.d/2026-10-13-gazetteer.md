# 2026-10-13 — Gazetteer: named seas, lakes, forests and ranges, and map labels

- **arda-names**: `PlaceKind::Sea` (keys `sea`, `gulf`), added last so every other kind keeps the number it seeds names with; bays, capes, islands, lakes and marshes are sometimes named for a founder, which gives them room for many names. Tests for natural-feature kinds (unique, deterministic, per preset), the seed numbers and the new key; README line.
- **arda-settle**: `gazetteer` module finds seas, bays, lakes, islands, capes, forests, marshes and mountain ranges on a 400 m coarse raster (components, exact distance transform, pole-of-inaccessibility anchors, centre-line label paths of at most 24 points), names them after regions so no earlier name changes, and writes `society/gazetteer.json`. `render::tilelabels` lays labels out once per zoom level and clips them to windows; the offline PNG now sets the same feature labels.
- **arda-people**: `SocietyFiles::gazetteer`.
- **arda-server**: `GET /v1/gazetteer[?kind=&bbox=&min_rank=]`; overview and relief tiles carry the labels by default, `?labels=0` leaves them off, cache keys include the flag; the oblique pyramid carries none. Schemas `Gazetteer`, `GazetteerFeature`, `GazetteerPoint`.
- **arda-midzoom**: `labels` module (labels on relief tiles).
- **Why**: a map reader and a game master expect "the Grey Sea" and "the Thornspine Mountains", and DiceMaster draws labels from Arda's JSON; closes the main part of OI-34.
- **Tests**: arda-names feature kinds; arda-settle made-world detection (two lakes, a bay, an island, a peninsula, a ridge), anchors and paths inside features, `names.json` bytes pinned to their pre-gazetteer values, tile-label seam and zoom tests; arda-midzoom seam test; arda-server `world_api/gazetteer.rs`.
