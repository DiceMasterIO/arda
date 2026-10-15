# Arda contract kit

The Arda side of the Arda ↔ DiceMaster contract test (1.0 task 10). It pins **which bytes** a
client of `/v1` may rely on, for one canonical world, one Arda release and the committed
placeholder library, and lets anyone prove that a running `arda-server` (a local build or the
published image) serves exactly them.

| Path | Holds |
|---|---|
| `manifest.json` | Kit format, fixture identity, and one entry per case |
| `golden/` | Response bodies, byte for byte as served (JSON only) |
| `requests/` | Bodies the `POST` cases send |
| `world.txt` | `<path>  <blake3>` of every file of the canonical world, sorted |
| `check.mjs` | The live checker: Node 22 or later, standard library only |
| `test/check.test.mjs` | The checker's own tests, against a local stub server |

About 2.8 MB in git. Large bodies (an area's 512 × 512 cells, the 2 × 1 window body) and every
image are stored as SHA-256 and size, not bytes.

The kit is versioned with Arda: `fixture.arda_version` and `fixture.server_version` in the
manifest name the release that recorded it, and a release bump re-records it
(`tools/release.sh`). A kit is valid for exactly that release; a client pins the kit together
with the image tag.

## The canonical world

One world, the same for every campaign: MICRO, seed 42, fine terrain at the release's default
recipe (9 from the recipe-9 change; 7 in 0.8.1), settled and society-built. With the release CLI:

```sh
arda generate --seed 42 --micro --terrain fine --out out/kit42
arda settle --world out/kit42
arda society build --world out/kit42
```

or with the image, writing into a host directory:

```sh
img=ghcr.io/dicemasterio/arda:<arda_version>
docker run --rm -v /abs/worlds:/worlds --entrypoint arda $img generate --seed 42 --micro --terrain fine --out /worlds/kit42
docker run --rm -v /abs/worlds:/worlds --entrypoint arda $img settle --world /worlds/kit42
docker run --rm -v /abs/worlds:/worlds --entrypoint arda $img society build --world /worlds/kit42
```

The result must equal `world.txt` file for file. With [`b3sum`](https://github.com/BLAKE3-team/BLAKE3):

```sh
(cd out/kit42 && find . -type f | sed 's|^\./||' | LC_ALL=C sort | xargs b3sum | awk '{print $2"  "$1}') \
  | diff - contract-kit/world.txt && echo identical
```

About a minute and a half on a desktop. The kit's own test (below) builds the same world
through the library API (`arda::generate_from_fine_source`, `arda_settle::generate`,
`arda_people::build`); with recipe 9 the CLI output and that fixture were byte-identical in all
47 files (as on 0.8.1 with recipe 7). Fine terrain is the CLI default since recipe 9, so
`--terrain fine` may be omitted; `arda generate --terrain legacy` is the legacy terrain and is
**not** the canonical world.

## Serving it

```sh
arda-server --world out/kit42 --port 8787 --library assets/tactical/placeholder
```

Default limits, no `--sheet-mapping`, the placeholder library (`library_version` in the
manifest). The image's default entry serves `/worlds/world` with the placeholder library when
no AI library is installed; name it explicitly so a stacked library can never sneak in:

```sh
docker run --rm -p 8787:8787 -e ARDA_LIBRARY=/opt/arda/library/placeholder \
  -v /abs/worlds/kit42:/worlds/world:ro ghcr.io/dicemasterio/arda:<arda_version>
```

## Checking a server

```sh
node contract-kit/check.mjs --base http://127.0.0.1:8787
node contract-kit/check.mjs --base http://127.0.0.1:8787 --only tactical-cell --only npc-notable
```

It replays every case in manifest order and compares the status, the `content-type`, each
listed header and the body (golden bytes, or SHA-256 and size). It prints one line per case,
the first differing JSON path when a JSON body differs, and a summary. Exit status: 0 when
every case matches, 1 on any difference, 2 when the server cannot be reached or the kit
cannot be read. Byte equality with goldens that Arda's test has validated against the served
schemas means the checker needs no JSON Schema library.

A cold server takes about 9 s for the whole kit (renders and the overview pyramid), a warm
one under a second; the bytes are the same either way. Two cases touch server queues:

- `tactical-prefetch` warms the open-sea neighbours of cell (5, 5); every neighbour is listed
  as `queued` only when the prefetch queue is idle, which it is unless another client is
  prefetching. Open-sea cells drain in milliseconds.
- `tactical-bake` posts an all-grass edit of cell (479, 1245) and `tactical-bake-delete`
  deletes that job again, so the kit can run any number of times.

`node --test contract-kit/test/` runs the checker's tests (pass, mismatch with its JSON path,
a changed image and its 304, an unreachable server).

## Manifest

```json
{
  "kit_format": 1,
  "fixture": {"seed": "42", "size": "micro", "terrain": "fine", "recipe_version": 9, "society": true,
              "arda_version": "0.8.1", "server_version": "0.8.1", "world_json_blake3": "…",
              "world_fingerprint": "world.txt", "library": "placeholder", "library_version": "0.1.0+seed58",
              "contract_version": 3, "api_version": "v1", "sheet_mapping": null},
  "cases": [{"id": "tactical-cell-png", "route": "GET /v1/tactical/cell/{gx}/{gy}.png",
             "method": "GET", "uri": "/v1/tactical/cell/479/1245.png?ppsq=64",
             "status": 200, "content_type": "image/png",
             "headers": {"cache-control": "no-cache", "etag": "\"…\""}, "schema": null,
             "body": {"sha256": "…", "bytes": 33607752}}]
}
```

- `route` is the route's row in `docs/capstone/versioning.md`; only **stable** routes are in
  the kit.
- `request` (`content_type`, `body` file) is sent with the `POST` cases.
- `if_none_match_of` names an earlier case whose `etag` the request sends as `If-None-Match`.
- `uri_from_location_of` marks a case whose `uri` is an earlier case's `location` header (the
  bake job to delete); the bless keeps it in step.
- `content_type` and each `headers` value are exact; `null` means the header must be absent.
- `schema` names the served schema (`/v1/schema/{schema}.json`) the body validates against.
- Exactly one of `golden` (a file under `golden/`) and `body` (`sha256`, `bytes`).

## Cases

54 cases over 41 of the 42 stable routes of `docs/capstone/versioning.md`, at MICRO-42 places
picked from the fixture: village 10
(Ifyil, realm 3), its cell (479, 1245), building 10.2, commoner `10.2.1`, the open-sea cell
(5, 5); the scene cases read town 2's cell (732, 1081), where notables stand by day and by
night, so the kit exercises token shapes.

| Need | Cases |
|---|---|
| Health, world | `health`, `world` |
| One place | `cell`, `point` |
| Area in bulk | `area-cells` (JSON, by hash), `area-cells-bin`, `area-rivers`, `area-lakes` |
| Continent and relief | `overview-png`, `overview-tile-webp` (+ `-304`, `-nolabels`), `overview-tile-png`, `relief-tile` |
| Names, realms | `gazetteer`, `realms`, `realm` |
| Settlements | `settlements`, `settlements-towns`, `settlement`, `settlement-plan`, `settlement-npcs` |
| Battle map | `tactical-cell`, `tactical-scene-day`, `tactical-scene-night`, `tactical-cell-png` (+ `-304`), `tactical-cell-tile`, `tactical-window` (2 × 1), `tactical-window-png`, `tactical-window-tile` |
| Warm, bake, library | `tactical-prefetch`, `tactical-bake`, `tactical-bake-delete`, `tactical-library` |
| Dungeon | `dungeon`, `dungeon-png`, `dungeon-scene` |
| People | `npc-notable`, `npc-commoner-ref`, `npc-commoner-id`, `npcs`, `npcs-realm-job`, `building-residents`, `building-workers`, `npc-population`, `npc-generate` |
| Shapes | `schema-index`, `schema-file` |
| Errors | `error-no-block`, `error-out-of-range`, `error-npc-not-found`, `error-bad-request`, `error-bake-not-found` |

Not in the kit: `GET /v1/tactical/bake/{id}` with a live job and its tiles
(`/bake/{id}/tiles/…`), whose state and existence depend on when the background bake finishes.
The test `every_stable_route_of_the_policy_has_a_case` keeps the kit in step with the policy
page: a new stable route without a case fails it.

## Arda's side

`crates/arda-server/tests/contract_kit.rs` replays every case in-process against the stamped
fixture `target/arda-server-fixture/micro42-society` with the CLI's default limits and the
placeholder library, and checks that every JSON golden validates against its served schema,
that `fixture` is the served world (versions, `library_version`, the BLAKE3 of `world.json`),
and that `world.txt` is the fixture's fingerprint. It runs in `cargo test --workspace`.

After an intended change of a body (a content change in a minor release, a version bump):

```sh
ARDA_BLESS_CONTRACT=1 cargo test --release -p arda-server --test contract_kit
```

rewrites the goldens, hashes, statuses and listed headers, the fixture identity and
`world.txt`. It never adds or removes cases; edit `manifest.json` by hand for that (add the
entry with a placeholder `golden` or `body`, then bless). `tools/release.sh <version>` runs the
bless as part of a bump, and `tools/release.sh --check` fails when the manifest's versions are
not the workspace's.

## For DiceMaster

Written for DiceMaster's Arda tests (`capstone/06-testing.md`), which do not exist yet.

- **Pinning.** DiceMaster pins `ghcr.io/dicemasterio/arda:<arda_version>` and the kit of the
  same Arda tag. A version bump means a new kit; DiceMaster copies (or vendors by tag) the
  whole `contract-kit/` directory and never edits it.
- **World.** The canonical world above replaces `arda generate --seed 42 --micro --out
  infra/local/worlds/micro-42` (legacy terrain, no society). Generate it with the pinned image
  and check it against `world.txt`.
- **`arda.replay.test.ts`** replays `golden/` through the `world` adapter (keyed by case `uri`
  and `method`, with `requests/` bodies for the posts) instead of recording fixtures with
  `refresh-arda-fixtures.ts`. Hashed cases (images, the area columns) stay out of the replay
  or use a synthetic body of the right type.
- **`arda.contract.test.ts`** starts the pinned image serving the canonical world, runs
  `node contract-kit/check.mjs --base <arda url>` and requires exit 0, then runs its `world`
  adapter over the same cases and validates every consumed body with its own JSON Schema
  validator against `GET /v1/schema/{Name}.json`. The kit's cases also hold the inputs for
  its other assertions: `error-no-block` (open sea), the 304 cases (`If-None-Match`), scene
  tokens with `origin_gs` (`tactical-scene-day`).
- **`npc-seed.test.ts`** takes its notable from `golden/npc-notable.json` and its commoner from
  `golden/npc-commoner-ref.json` (`10.2.1`, the same person as `npc-commoner-id`).
