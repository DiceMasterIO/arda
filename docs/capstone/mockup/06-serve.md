---
generated_date: 2026-09-07
scenarios: [serve-vtt]
generated_at_commit: 8d3c9d9
capstone_version: 6.4
---

# 06 — `arda serve` (CLI/docker): read-only HTTP API

> Retained design and dated implementation history. Current implementation: `../01-architecture.md`, `../04-data-flow.md`.

Added at the build gate: the user's VTT consumes areas/blocks
over HTTP instead of linking the crate. Read-only, synchronous server
(no async runtime), same renderers/serializers as
export. Endpoint paths assumed.

## Layout

```text
$ docker run -p 8080:8080 -v $PWD/worlds:/worlds ghcr.io/<owner>/arda \
    serve /worlds/w42 --port 8080
arda serve — world w42 (190 areas), listening on:8080  (read-only)

GET /manifest                            → world.json content
GET /areas/03_11/cells.json              → area cell properties
GET /areas/03_11/objects.json            → settlements, rivers, roads, buildings, npcs
GET /areas/03_11/map.png                 → cartographic area render
GET /areas/03_11/cells/300,128/block.json→ tactical grid + legend attrs + poi
GET /areas/03_11/cells/300,128/block.png → symbolic render
GET /realms.json                         → realm borders + members
GET /settlements/{id}/npcs.json          → notables; ?commoner=<n> derives on demand
```

## Elements

| Element | Does | Status / notes |
| --- | --- | --- |
| `serve` subcommand | Serves one world directory, read-only; renders on demand with the export code paths | Confirmed design |
| `--port` (default 8080) | Bind port; docker EXPOSEs it | (assumed) |
| Caching | Deterministic outputs → strong ETags from (seed, path) (assumed) | Deterministic design |

## States

- **Success**: 200 with JSON/PNG; identical bytes for identical requests.
- **Not found**: unknown coords → 404 carrying valid ranges (mirrors logic/05 range errors).
- **Partial world**: refuses to start, same message as export (logic/04).
- **Writes**: none — no mutating endpoint exists; world stays immutable.
