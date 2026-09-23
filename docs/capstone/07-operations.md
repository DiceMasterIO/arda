---
generated_at_commit: 757b2ab5418b
generated_date: 2026-09-23
content_hash: a290bff2f390
paths_covered: [":(top)Cargo.toml", ":(top)crates/*/Cargo.toml", ":(top)crates/*/src/**", ":(top)Dockerfile", ":(top).github/**", ":(top)rust-toolchain.toml", ":(top)deny.toml", ":(top)crates/*/examples/**"]
absorbed_from: features/2026-09-07-area-water-terrain-realism@2026-09-08, features/2026-09-22-geographical-rendering-first-pass@2026-09-22, features/2026-09-23-terrain-corrections@2026-09-23
---

# Operations

## Processes

| Process | Local command | Container command |
|---|---|---|
| Generate | `cargo run -p arda-cli --release -- generate --seed 42 --micro --out /tmp/arda-world` | `docker run --rm -v /host/worlds:/worlds arda-local generate --seed 42 --micro --out /worlds/w42` |
| Preview | `cargo run -p arda-cli --release -- preview --seed 42 --micro --out /tmp/arda-preview` | Same image and volume; `preview --seed 42 --micro --out /worlds/preview` |
| Export | `cargo run -p arda-cli --release -- export --world /tmp/arda-world --area 0,0 --format png --out /tmp/arda-export` | Same image; `export --world /worlds/w42 --overview --out /worlds/export` |

Commands resolve to [main.rs:23](../../crates/arda-cli/src/main.rs:23)
and run to completion. `generate` writes world data, including sampled tactical
blocks, without rendering PNGs. `preview` generates a new world under its output
directory and then renders an overview. `export` reads an existing world without
rerunning terrain or water. Build the local image with
`docker build -t arda-local .`; host-mounted destinations must be writable by the
container's UID 10001. `serve` remains absent from the command enum.
Source: [lib.rs:21](../../crates/arda/src/lib.rs:21),
[Dockerfile:1](../../Dockerfile:1).

## Configuration

Application configuration uses CLI arguments or `GenerateConfig`, with explicit
library resource limits. There is no application-owned runtime environment
configuration. Compile-time `CARGO_PKG_VERSION` labels the manifest and CLI;
third-party runtime controls such as `RAYON_NUM_THREADS` are distinct from saved
world configuration. Source:
[main.rs:17](../../crates/arda-cli/src/main.rs:17),
[orchestrator.rs:518](../../crates/arda-gen/src/orchestrator.rs:518).

| Name | Default | Consumer | Documentation |
|---|---|---|---|
| --seed | required | generate/preview | `crates/arda-cli/src/main.rs:23` |
| --size | 500x1000 km | generate/preview | `crates/arda-cli/src/main.rs:23` |
| --micro | false; selects 102×204 km MICRO, overriding --size | generate/preview | `crates/arda-cli/src/main.rs:23` |
| latitude / density | 35–55°N / 15 people per km² | CLI construction; other values through library config | `crates/arda-core/src/config.rs:68` |
| --quality | 8192 (8K); any integer 512–32768 pixels or integer k/K suffix, where 1K = 1024 | preview overview; export area/overview PNG | `crates/arda-cli/src/main.rs`, `crates/arda-render/src/quality.rs` |
| --style | omitted = Classic; explicit `classic` or `atlas` | preview and saved area/overview PNGs, including area `--detail` | `crates/arda-cli/src/main.rs:55` |
| --px | absent; legacy 1–512 pixels per area when supplied | preview overview; conflicts with --quality and explicit --style | `crates/arda-cli/src/main.rs` |
| --area | 0,0 | export area, unless block/overview | `crates/arda-cli/src/main.rs:23` |
| --format | png; choices png/json | export area/block | `crates/arda-cli/src/main.rs:85` |
| --detail | false; 4096×4096 area PNG; Classic uses `_detail`, Atlas uses the standard filename | export; rejected with JSON, overview, block or --quality | `crates/arda-cli/src/main.rs` |
| --overview | false | export saved world overview | `crates/arda-cli/src/main.rs:23` |
| --block | absent; ax,ay,cx,cy | export one saved sampled land block | `crates/arda-cli/src/main.rs:23` |
| --out / --world | required where applicable | command enum | `crates/arda-cli/src/main.rs:23` |

Area PNGs use the selected quality for both axes. Overview PNGs use it for the
long edge and round the shorter edge to the nearest pixel from the saved area-grid
aspect ratio. Preview also defaults to an 8192-pixel long edge; explicit `--px`
selects the older per-area sizing. Explicit `--quality` is rejected with JSON or
blocks before world loading/output creation; ordinary JSON exports without it
retain their existing behavior. Explicit `--style` with JSON or a tactical block
is rejected before world loading or output-directory creation. Preview `--px`
conflicts with explicit `--style`; Atlas preview uses `--quality` or its default.
Unknown style names fail CLI parsing. Source:
[main.rs:55](../../crates/arda-cli/src/main.rs:55),
[main.rs:123](../../crates/arda-cli/src/main.rs:123).

For an existing world, append `--style atlas` to either
`export --world /tmp/arda-world --overview --out /tmp/atlas-overview` or
`export --world /tmp/arda-world --area 2,0 --quality 8k --out /tmp/atlas-area`.
Use separate output directories to retain a Classic comparison: both styles
use `overview.png` or `area_XX_YY.png` for quality exports. The retained
[comparison panel](features/2026-09-22-geographical-rendering-first-pass/evidence/index.md)
shows actual saved-world output; extra PNG pixels still interpolate the same
100 m terrain and do not supply forests, roads or settlements.

Valid sizes are 64–4000 km per axis; latitude lies within −80° to 80° with south
strictly below north; density is 1–200 people/km². Loading revalidates the saved
config, exported dimensions and derived modeled extent rather than trusting
deserialized fields. The CLI has no config-file, memory-limit or serve-port flag.
Source: [config.rs:83](../../crates/arda-core/src/config.rs:83),
[world.rs:88](../../crates/arda/src/world.rs:88).

## Generation boundaries and capacities

Each exported area is 512² cells of 100 m. The existing area-count calculation
uses integer 51 km units; shared water therefore models the union rectangle with
each axis `max(requested_km × 10, exported_area_count × 512)`. Default 500×1000 km
has 171 exported areas and 200 prepared tiles covering 5000×10000 fine cells;
MICRO has eight areas covering 1024×2048 fine cells (102.4×204.8 km). Partial
right/bottom hydrology-only tiles affect upstream flow and reentry but produce no
extra final area. Climate sampling clamps at the available coarse rim when this
union preserves an existing exported overshoot. Source:
[prepared_domain.rs:29](../../crates/arda-gen/src/hydrology/prepared_domain.rs:29),
[forcing_sample.rs:37](../../crates/arda-gen/src/hydrology/forcing_sample.rs:37).

Generation evolves one complete modeled physical rectangle, persists immutable
area/fringe slices, releases the dense surface, solves shared annual water once,
indexes global features, then composes areas. Initial relief uses the radial
continent mask and four aligned area-detail lattice spacings of 40/20/10/5 cells.
Forty shared evolution iterations carry uplift, fractional contributing area,
creep and collapse across area cuts; implicit downstream-first incision uses
updated receiver beds. Only the actual modeled outer rim remains fixed. Sources:
[prepare.rs](../../crates/arda-gen/src/area/prepare.rs),
[evolution.rs](../../crates/arda-gen/src/area/evolution.rs),
[coast.rs](../../crates/arda-gen/src/continent/coast.rs),
[area_detail.rs](../../crates/arda-gen/src/continent/area_detail.rs).

Fine marine connectivity is separate
from a negative inland bed. Annual supply is `P`; annual Hamon evaporation `E` is
the sum of twelve climatological months; land loss `A = min(P/2,E)`, runoff
`R = P−A`, extra wet cost `D = E−A`. Partially supported shore bands use an explicit
marginal evaporation account and integer-mm representative surfaces. These
values describe a static annual-support model, not seasonal storage, elapsed
filling, groundwater or snow inventories. Source:
[annual_aggregation.rs:53](../../crates/arda-gen/src/hydrology/annual_aggregation.rs:53),
[area generation](logic/02-area-generation.md).

`arda::generate_with_limits` accepts `HydrologyLimits`; `generate` and the CLI use
its defaults. Admission runs before output creation, and actual stages also
enforce their admitted counts/work. Increasing a capacity changes admission,
not the terrain or annual model. Source:
[lib.rs:33](../../crates/arda/src/lib.rs:33),
[generation_limits.rs](../../crates/arda-gen/src/orchestrator/generation_limits.rs),
[types.rs:84](../../crates/arda-gen/src/hydrology/types.rs:84).

| Default capacity | Value |
|---|---:|
| Closed physical terminal leaves | 100,000 |
| Grouped annual bands | 6,000,000 |
| Saved feature records | 4,000,000 |
| Area-to-reach references, including halo and endpoint closure | 16,000,000 |
| Outward lake-boundary edges | 4,000,000 |
| Owned payload reservation | 16 GiB |
| Simultaneous spatial / combined scratch | 256 GiB each |
| Declared logical operations | 2^48 |
| Declared I/O bytes / filesystem operations | 2^64 / 2^54 |

Declared work includes shared terrain sampling and every bounded evolution pass,
including the flood heaps' logarithmic comparisons. It counts bounded transitions
and comparisons, not CPU instructions or seconds. Current default admission sums
1,881,932,212,727 such units, below the unchanged 2^48 allowance; a caller limit
that omits this terrain work is refused before output is created.

The previous per-area pinned rim/taper produced grid-aligned troughs. Current
shared evolution removes those internal boundary conditions, but natural-realism
acceptance remains open while the full corrected candidate is inspected. The
earlier observations remain preserved in [open items](open-items.md) and feature
evidence; they are not a claim that the current implementation still pins every
area rim.

The RAM figure is an owned-payload reservation, not a process-RSS guarantee.
Admission includes two dense `i32` terrain inputs and the shared kernel scratch:
53 bytes per modeled fine cell plus container headers on the current 64-bit
layout, conservatively added alongside coarse and water-stage reservations even
though the dense surface is released before annual solving. MICRO and default
500×1000 km requests fit the default 16 GiB RAM allowance. A valid 4000×4000 km
request is now refused by default RAM admission before output creation; the
library caller must provide explicitly larger limits to admit it. A focused
admission test accepts it with 128 GiB, which is not a measured maximum-world
runtime or successful-generation guarantee. Scratch caps do not include all final
world files, and actual basin/band/feature counts can still exceed their caps.
Exceeding a limit returns a
typed error; it does not discard basins, change thresholds or choose different
physics. Source:
[types.rs:80](../../crates/arda-gen/src/hydrology/types.rs:80),
[generation_limits.rs](../../crates/arda-gen/src/orchestrator/generation_limits.rs),
[generation_limits_tests.rs](../../crates/arda-gen/src/orchestrator/generation_limits_tests.rs).

## Publication, loading and export

Generation accepts an absent or empty destination, creates its exclusive marker,
writes layers and private scratch, and publishes the final manifest only after
all required stages succeed. The final same-directory manifest rename is the
last fallible publication operation. A failure leaves diagnostic partial output
without a readable completed manifest; retry uses a fresh destination. There is
no automatic checkpoint resume, in-place overwrite or power-loss durability
guarantee. Source:
[publication.rs:58](../../crates/arda-gen/src/orchestrator/publication.rs:58),
[publication.rs:122](../../crates/arda-gen/src/orchestrator/publication.rs:122).

Current saved format major is 4. Older incompatible worlds must be regenerated;
there is no migration command. `World::load` reads/validates manifest metadata and
creates empty caches, without eagerly loading all areas or global tables.
`area()` caches the requested area; `read_area()` returns an uncached owned read.
Reads validate exact cell-file length and bounded object length before allocation,
then validate record coordinates and copied context against the loaded domain.
Overview export propagates a failed area read. Source:
[formats/mod.rs](../../crates/arda-core/src/formats/mod.rs),
[world.rs:88](../../crates/arda/src/world.rs:88),
[world.rs:203](../../crates/arda/src/world.rs:203),
[lib.rs:144](../../crates/arda/src/lib.rs:144).

Area and overview PNG export default to 8K (8192 pixels); `--quality 512`,
`--quality 16k` and `--quality 32K` select other validated sizes. Area output is
square; overview output preserves the area-grid aspect ratio. The 512-pixel area
mode retains a faint mark for subpixel streams. Larger quality sizes and Classic legacy
`--detail` (4096 pixels, `_detail` filename) show physical channel coverage alone.
All use the same saved 100 m terrain. Saved global IDs, crossings and per-area
context preserve shared topology; rendering does not recalculate river widths
from local fragments. Invalid geometry or rendering-work caps return typed errors.
Atlas changes PNG presentation only: earthy height and sea-depth colors, fixed
directional relief and pixel-center interpolation from saved cells. It reads
the two nearest rows/columns and corners from existing in-world neighbors;
missing or corrupt neighbors fail the export. Out-of-world edges use one-sided
gradients. Atlas area PNGs keep physical lake depths and channel geometry;
the overview keeps categorical lake fill and discharge-band river symbols.
Classic remains the omitted-style behavior, including existing library entry
points. The additive styled APIs are
`export_area_with_quality_and_style` and
`export_overview_with_quality_and_style`. `--detail --style atlas` writes the
quality-path `area_XX_YY.png` at 4096 pixels; Classic `--detail` retains its
legacy `_detail` name. Sources:
[atlas.rs](../../crates/arda-render/src/atlas.rs),
[atlas facade](../../crates/arda/src/atlas.rs),
[export_quality.rs:27](../../crates/arda/src/export_quality.rs),
[main.rs:290](../../crates/arda-cli/src/main.rs:290).
Sources: [quality.rs](../../crates/arda-render/src/quality.rs),
[channels.rs](../../crates/arda-render/src/channels.rs),
[export_quality.rs](../../crates/arda/src/export_quality.rs).

For Linear×Linear output, Atlas reconstructs displayed land/sea ownership from four class-directed signed heights using exact rational pixel centers and i128 bilinear weights. Land samples are at least +1 mm and sea samples at most −1 mm. Positive chooses land, negative sea, and exact zero retains saved ownership. Saved lakes, quads touching lakes, alternating land/sea checkerboards, exact saved-cell centers and guarded one-cell islands/straits retain saved ownership. Any Box axis also retains the existing aggregation/ownership rule. Area colour and channel clipping and both overview paths use this same classifier; an internal AtlasSea feature prevents river-trunk widening over reconstructed sea. Classic remains unchanged (`crates/arda-render/src/atlas.rs:412`, `crates/arda-render/src/channels.rs:451`, `crates/arda-render/src/overview.rs:239`, `crates/arda-render/src/overview/streaming.rs:170`).

Quality exports, including the default area/overview CLI paths, stream into an
exclusive temporary sibling file and rename it to the final filename only after
encoding and flushing succeed. A failed render/write preserves any previous
completed PNG and attempts to remove its temporary file on ordinary error return.
This does not promise cleanup after abrupt process termination or power-loss
durability. Legacy `--detail`, block and JSON exports, and explicit preview `--px`,
retain their completed-buffer writes; an I/O failure in those paths can leave a
partial destination. Source:
[export_quality.rs](../../crates/arda/src/export_quality.rs),
[lib.rs](../../crates/arda/src/lib.rs).

The streaming overview renderer, `write_overview_png`, uses bounded bands and
supports square 32K output. Area streaming uses reusable rows and bounded channel
geometry/candidate storage. The buffered `OverviewRaster::new_exact` accepts at
most 32,768 pixels per axis and retains its 134,217,728-pixel total limit; square
16K and 32K images exceed that buffered budget. The regular buffered constructor's
512-pixels-per-area and 64-million-pixel limits remain unchanged. Sources:
[overview.rs](../../crates/arda-render/src/overview.rs),
[streaming.rs](../../crates/arda-render/src/overview/streaming.rs),
[carto.rs](../../crates/arda-render/src/carto.rs).

Atlas area export retains a 514×514 derived palette/light/class/height grid and fixed
516×516 height context. Neighbor area payloads are loaded one at a time while
their two-cell strips or corners are copied. Atlas overview keeps the existing
256-row output bands and can reread areas across bands; the cost therefore
depends on output height and area layout even though raster memory stays bounded.
The measured local Linux seed-42 200×300 km exports completed at 32K twice each:

| Atlas 32K PNG | Pixels | Elapsed runs | Peak child RSS runs |
|---|---:|---:|---:|
| Area (2,0) | 32768×32768 | 31.06 / 31.35 s | 44,312 / 43,956 KiB |
| Overview | 19661×32768 | 29.57 / 29.81 s | 66,604 / 66,204 KiB |

Each repeat pair has identical PNG bytes. RSS is Linux `wait4` child rusage,
not an all-platform or concurrent export budget. The saved input's 55 file
hashes remained unchanged. Sources:
[area run 1](features/2026-09-22-geographical-rendering-first-pass/evidence/atlas-area-32k-run1.time.txt),
[area run 2](features/2026-09-22-geographical-rendering-first-pass/evidence/atlas-area-32k-run2.time.txt),
[overview run 1](features/2026-09-22-geographical-rendering-first-pass/evidence/atlas-overview-32k-run1.time.txt),
[overview run 2](features/2026-09-22-geographical-rendering-first-pass/evidence/atlas-overview-32k-run2.time.txt),
[image hashes](features/2026-09-22-geographical-rendering-first-pass/evidence/images.json).

The earlier saved-world 16K exporter remains available as a workspace example:

```sh
cargo run -p arda --release --example export_world_16k -- worlds/w42 world-16k.png
```

It follows the manifest's area aspect ratio, rounds the shorter axis and uses
16,384 pixels on the long edge. A 9×19 world renders at 7,761×16,384. It refuses
an existing output and reads areas one at a time without generating terrain.
The CLI now supports this size directly with `export --overview --quality 16k`.
The [map legend](../map-legend.md) explains overview and area colours.
Source: [export_world_16k.rs](../../crates/arda/examples/export_world_16k.rs).

Tactical generation currently samples land cells at stride 64 in both area axes;
it does not materialize a block at every 100 m cell. Each current WFC block is
64² squares with the 24-tile vocabulary. `--block 1,2,64,64` names a sampled
coordinate, but availability still depends on its being land. The first block
query loads/decompresses its containing area's archive; later queries reuse the
cache. Per-block archive decompression is not implemented. Source:
[orchestrator.rs:136](../../crates/arda-gen/src/orchestrator.rs:136),
[world.rs:257](../../crates/arda/src/world.rs:257),
[tiles.rs](../../crates/arda-core/src/tiles.rs).

## Infrastructure

`Dockerfile:1` builds with `rust:1-bookworm`, copies the release binary to `debian:bookworm-slim`, runs UID 10001, declares `/worlds`, and uses entrypoint `arda`. It declares no ports, healthcheck or compose services. Multi-architecture image publication and crates.io/GHCR release automation remain designed but absent; `.github/workflows/` contains CI only.

### September 23 integrated measurements

The fixed seed-42 500×1000 km world completes in 1,640.71 s (27 min 21 s); seed-42 200×300 km takes 183.29 s and seed-99 MICRO 46.60 s. These are individual local release runs, not runtime guarantees. The default 32K Atlas overview is 15,522×32,768, 129,037,747 bytes, and takes 65.84 s; a byte-identical repeat takes 65.65 s with 57,240 KiB peak RSS. All 523 saved files remain unchanged during the repeat export. A 1000×1000 km world has not been benchmarked here; the conversational 55–75 minute estimate is not measured evidence.

Receipts and outputs: [evidence index](features/2026-09-23-terrain-corrections/evidence/README.md). Existing output directories retain the original and corrected worlds separately. Source compatibility and visual acceptance are recorded in [current status](open-items.md).

## Developer workflow

The exact test/lint/MSRV/dependency commands and dated results are in
[testing](06-testing.md).
CI runs `cargo test --workspace --all-features` once per platform, including the
workspace golden-world tests; the former second determinism invocation has been
removed. It retains the three-platform matrix and Ubuntu Rust 1.96.1 for the
dated latest-stable-minus-two target. Stable tooling includes rustfmt/Clippy;
`cargo bench -p arda-gen` invokes the two declared benches. No database migration
process or remote service is involved. Source:
[ci.yml:18](../../.github/workflows/ci.yml:18),
[rust-toolchain.toml](../../rust-toolchain.toml).

The previous exact-HEAD run checked September 22 still has the recorded Windows
golden-text LF/CRLF failure; Linux/macOS, lint, MSRV and dependency jobs passed.
The run and fingerprint comparison are recorded in [current status](open-items.md).
Configured gates and historical passing measurements do not imply a fresh green
run. The Atlas feature's subsequent local Linux workspace, formatting, strict
Clippy and Rust 1.96.1 checks passed at HEAD `342d03e55120`; the prior passing
dependency check remains applicable with unchanged manifests, lockfile and
policy. No new Windows run was made. See the dated [testing evidence](06-testing.md)
and [gate results](features/2026-09-22-geographical-rendering-first-pass/evidence/gates/results.json).

Forcing tables are checked-in runtime data. The explicit maintenance command
`python3 tools/generate_water_forcing_tables.py /tmp/arda-forcing-tables.rs`
writes an offline candidate and prints its SHA-256; normal builds do not execute
Python or download climate data. The generator records observed profile inputs
and astronomical/daylength provenance; its procedural analogs are not a claim
of global calibration. Source:
[generate_water_forcing_tables.py:1](../../tools/generate_water_forcing_tables.py:1),
[forcing_tables.rs:1](../../crates/arda-gen/src/hydrology/forcing_tables.rs:1).

## Observed timings — 2026-09-08

These release measurements were taken locally on Linux under concurrent host
load. They are exploratory observations, not service-level or maximum-world
guarantees. Saved export timings include CLI startup/read/write; the tactical
probe below separates in-process work.

### Current regional-detail correction: candidate06

The unprofiled five-world panel completed all 243 generation/export commands.
World times include terrain, shared water, all published areas and sampled blocks;
they exclude PNG rendering. The three default cases ran concurrently on this
Linux/WSL host, so these are observed runs rather than isolated benchmark promises.

| Operation | Observed wall time | Scope |
|---|---:|---|
| Complete default seed42 | 38 min 00 s | 500×1000 km, 171 areas |
| Complete default seed7 | 43 min 50 s | Same dimensions |
| Complete default seed436342 | 44 min 18 s | Same dimensions, reported world |
| Complete MICRO panel worlds | 53.61–55.35 s | Eight areas each |
| Initial independent MICRO42 | 45.99 s | Different concurrent host load |
| Reported world 16K export | 3.576 s | Native saved-data renderer, 7,761×16,384 |
| Four requested 4096² areas | 0.301–0.385 s each | Existing saved worlds, PNG only |

These timings used the native renderer adapter from candidate05. Its exact-size
capability now lives in the canonical renderer and the workspace example above;
the migrated example reproduced the C05 PNG byte for byte in 3.718 s. The public
CLI size/limit contract was unchanged at that measurement; the subsequent
`--quality` change described above adds the current 8K default and 32K maximum.
These measurements do not establish timings for that new path. The finer raster does not refine the
underlying 100 m terrain. The current saved format and JSON schema are unchanged;
old worlds still load and require regeneration to obtain the corrected terrain.
Exact commands, times and output hashes:
[C06 panel](features/2026-09-07-area-water-terrain-realism/verification/lake-district-correction/panel-and-seams-summary.json),
[render receipt](features/2026-09-07-area-water-terrain-realism/verification/lake-district-correction/render-and-seams-receipt.json).

### Previous terrain correction: candidate05

All five frozen worlds and 243 generation/export commands completed. Generation
writes saved terrain/water and sampled tactical blocks without PNGs. Default
500×1000 km driver times are **36m52s / 43m07s / 47m26s** for seeds42/7/436342;
GNU process wall records35m36s / 41m37s / 45m48s. Both clocks are preserved;
the host discrepancy remains unexplained. Peak default RSS is about1.762GiB per
process under up to three concurrent worlds. MICRO driver times are49.1–49.3s.

Actual profile stages separate coarse generation, shared fine terrain, water,
indexing and final area writing. Default shared terrain sampling/evolution takes
17.34–23.30min and shared water takes16.98–21.44min under this load. Once shared
state exists, mean area materialization including the current sampled WFC and
writing is0.486–0.563s across land/sea areas; this is not independent area generation.

Saved default PNG export adds0.947–1.084s for a world overview,14–129ms for a512
area preview, or166–547ms for a4096 detail. Area JSON export takes122–196ms.
These process observations include reads/encoding/writes with filesystem caches
left intact. The panel does not time a full image export of every area/block,
and browser/network performance is not inferred.

The delivery also renders the saved world at4608×9728 through the existing public
API, using512 pixels per area:2.134s and8.40MB. All523 saved-world files retain
their hashes. The user excluded tactical images from final delivery. See [complete timing breakdown](features/2026-09-07-area-water-terrain-realism/reports/terrain-correction--timings-c05.md)
and [current gallery](features/2026-09-07-area-water-terrain-realism/output/previous/terrain-correction--current-gallery-c05.md).

The additional world-only 16K export is 7761×16384 (127,156,224 pixels),
12,392,185 bytes, and took 3.670 s with 556,592 KiB peak RSS. It uses an isolated
copy of the native Rust overview renderer with exact dimensions and a bounded
134,217,728-pixel allocation. At the time, this did not extend the production API's
64-million-pixel limit; the current buffered and streaming limits are described
above. Saved 100 m cells are sampled directly and repeated where necessary;
the palette, river thresholds and physical terrain resolution are unchanged.
All 523 saved-world files and 157 production source files retain their hashes.
The four area renders remain 4096². See the local [16K receipt](features/2026-09-07-area-water-terrain-realism/verification/terrain-correction/render-16k/receipt.json).

### Previous terrain correction: candidate04

The completed frozen panel contains three default worlds and two MICRO worlds,
with 238 saved export commands and 38 selected areas. The default generations
produce 171 areas plus sampled tactical blocks, without PNG rendering.

| Current operation | Observed time | Scope |
|---|---:|---|
| Default seed 42 generation | 45 min 09 s | Driver elapsed; GNU child-process wall 43 min 52 s. |
| Default seed 7 generation | 47 min 02 s | Driver elapsed; GNU child-process wall 45 min 41 s. |
| Default seed 436342 generation | 49 min 43 s | Driver elapsed; GNU child-process wall 48 min 18 s. |
| Saved default world overview | 0.928–1.117 s | Adds roughly one second after generation. |
| Saved default area PNG 512 | 17–171 ms | Does not rerun terrain/water. |
| Saved default area PNG 4096 | 183–451 ms | Same saved terrain at higher raster resolution. |
| Saved default area JSON | 123–173 ms | CLI read, encode and write; browser/network costs excluded. |

The three default worlds ran concurrently with other host work. Peak RSS is
approximately 1.762 GiB per default process, versus historical candidate02's
300–643 MiB. Driver and GNU timings differ; the cause has not been established,
and both clocks are preserved in the evidence. Generation and memory cost
increased with the whole-domain correction. This is not an isolated performance
comparison, and no current per-stage profiler was enabled. The run does not
measure an all-images export for every area and tactical block. See
[full current comparison](features/2026-09-07-area-water-terrain-realism/reports/terrain-correction--candidate04-comparison.md).

The three retained tactical timing controls have also been rerun against current
candidate04 MICRO42 cells. Constraints plus the existing WFC took 6.72–9.23 ms;
including PNG encoding took 9.36–12.46 ms, excluding shared loading and file writes.
Saved PNG CLI export took 4.30–5.53 ms and JSON export 1.76–3.32 ms. All regenerated
blocks and CLI exports matched the saved current data exactly. These measurements
still concern the existing 24-tile system; future asset placement is outside this
timing scope. Evidence:
[current tactical timings](features/2026-09-07-area-water-terrain-realism/verification/terrain-correction/tactical-current/report.json).

The corrected MICRO seed 42 completed generation in **63.50 s** without PNG
rendering, with peak RSS **85,272 KiB** (about 83.3 MiB). Saved exports completed
successfully: overview **0.06884 s**, area (1,1) 512 PNG **0.03759 s**, 4096 detail
PNG **0.26460 s**, and JSON **0.12377 s**. These are one completed MICRO run, not
default-world timings or evidence that natural-realism acceptance is closed.
The full candidate04 run has completed; overall visual acceptance remains open.
Sources:
[generation measurement](features/2026-09-07-area-water-terrain-realism/verification/terrain-correction/micro42-c04-time.txt),
[export measurements](features/2026-09-07-area-water-terrain-realism/verification/terrain-correction/micro42-c04-exports.json).

### Historical pre-correction measurements: candidate02

The following table and frozen-panel results precede shared terrain evolution,
implicit incision, four-octave detail and the radial coast correction. They remain
useful historical evidence, but do not establish current default-world timings.
In particular, per-area erosion is no longer a production generation phase.

| Operation | Observed wall time | Scope |
|---|---:|---|
| Complete MICRO generation | 61–67 s | Eight areas plus sampled tactical data and publication; no PNG rendering. |
| Erosion of one prepared area | 1.8–3.75 s | Terrain erosion only; excludes shared hydrology and final composition. |
| Complete default generation | 33 min 50 s–35 min 7 s | Three 500×1000 km seeds; 171 areas plus sampled blocks, no PNG. |
| Default world overview | 0.98–1.10 s | Export after generation; not every area/tactical image. |
| Default seed 42 shared replay | 21 min 23 s | 50 million prepared cells; excludes original preparation, final index, final areas/blocks and publication. |
| Saved area PNG 512 | 10–130 ms | Existing-world export. |
| Saved area PNG 4096 | 160–520 ms | Existing-world export at detailed scale. |
| Saved area JSON | 110–630 ms | Existing-world export. |
| Saved tactical PNG / JSON CLI | 4–5 ms / 2–3 ms | Reads existing generated block; does not run WFC. |

For historical candidate02, all three independent default generations and the complete 238-export frozen panel passed their numerical/export checks. Their shared solves took 22 min 36 s–23 min 52 s, preparation 8 min 38 s–8 min 43 s, and peak process RSS 300–643 MiB. MICRO peaks were about 84 MiB. Those passes did not settle the subsequently reopened visual-realism issues. Full values and scope are in [performance evidence](features/2026-09-07-area-water-terrain-realism/reports/performance.md). Evidence:
[implementation progress](features/2026-09-07-area-water-terrain-realism/history/implementation-progress.md),
[candidate driver](features/2026-09-07-area-water-terrain-realism/verification/run_candidate.py),
[shared replay](features/2026-09-07-area-water-terrain-realism/verification/shared-replay/validation.json).

The historical three land-cell tactical probes at area (1,2), cells (64,64), (256,256) and
(384,384), measured actual constraints plus current WFC at **5.93–6.95 ms**, PNG
encoding at **2.08–2.23 ms**, and JSON serialization at **0.036–0.037 ms**.
Generation plus PNG was 8.03–9.04 ms, excluding file writes and shared loading.
World metadata load cost 0.027 ms, the one shared area load 10.99 ms, and the first
saved-block archive query 0.671 ms; later archive queries were cached. All three
regenerated blocks and their exports matched saved data. These figures concern
the current 24-tile WFC, with no timing claim for future assets or full tactical
coverage. Source:
[tactical timing report](features/2026-09-07-area-water-terrain-realism/verification/tactical-timing/report.json).
