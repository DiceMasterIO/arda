<p align="center">
  <img src="assets/logo.svg" alt="Arda" width="380">
</p>

<p align="center">
  <strong>Worlds from the ground up.</strong><br>
  Deterministic world generation in Rust.<br>
  Terrain, climate, and water, from a single seed.
</p>

<p align="center">
  <a href="https://github.com/DiceMasterIO/arda/actions/workflows/ci.yml"><img
    src="https://img.shields.io/github/actions/workflow/status/DiceMasterIO/arda/ci.yml?style=flat-square&amp;label=CI"
    alt="CI status"></a>
  <a href="#install"><img
    src="https://img.shields.io/badge/Rust-stable-80AD92?style=flat-square"
    alt="Built with stable Rust"></a>
  <a href="#map-and-data-exports"><img
    src="https://img.shields.io/badge/formats-BIN%20%2B%20PNG%20%2B%20JSON-444C56?style=flat-square"
    alt="BIN world data, PNG maps, and JSON exports"></a>
  <a href="LICENSE"><img
    src="https://img.shields.io/badge/license-Apache--2.0-444C56?style=flat-square"
    alt="Apache-2.0 licensed"></a>
</p>

<p align="center">
  Generate a continent, resolve its terrain and drainage into local areas,
  and save it as binary world data. Export PNG maps and JSON, or load the
  cells and water features through the Rust library. Run locally or in Docker.
</p>

<p align="center">
  <a href="#install">Install</a> ·
  <a href="#quick-start">Quick start</a> ·
  <a href="#what-you-get">What you get</a> ·
  <a href="#map-and-data-exports">Exports</a> ·
  <a href="#rust-library">Library</a> ·
  <a href="#docker-and-server-use">Docker and servers</a> ·
  <a href="#current-status">Status</a> ·
  <a href="#development">Development</a>
</p>

<p align="center">
  Read the <a href="docs/map-legend.md">map legend</a> or explore the
  <a href="docs/capstone/00-index.md">architecture reference</a>.
</p>

---

## Install

Build from source with stable Rust and a native C toolchain (compiler and linker):

```sh
git clone https://github.com/DiceMasterIO/arda.git
cd arda
cargo install --path crates/arda-cli --locked
```

This installs the `arda` command. To build inside the checkout instead:

```sh
cargo build --release -p arda-cli --locked
```

The binary is written to `target/release/arda` (`arda.exe` on Windows).
Use release builds for generation.

## Quick start

Start with the eight-area MICRO world:

```sh
# Generate the saved world data.
arda generate --seed 42 --micro --out worlds/w42

# Render an overview from that world.
arda export --world worlds/w42 --overview --out exports/w42

# Export one area at the default 8K resolution and as JSON.
arda export --world worlds/w42 --area 1,1 --out exports/w42
arda export --world worlds/w42 --area 1,1 --format json --out exports/w42
```

Generation writes `world.json` and the native binary layers under
`worlds/w42/`. The exports are `overview.png`, `area_01_01.png`, and
`area_01_01.json`. Area coordinates are zero-based `x,y` pairs.

Generation needs a new or empty destination. Exporting reads the saved
world; it does not rerun terrain or water generation.

For a single command that generates a new world and renders its overview:

```sh
arda preview --seed 42 --micro --quality 2k --out previews/w42
```

This writes `previews/w42/world/` and `previews/w42/overview.png`.
`preview` runs generation too, so use `export` for a world you already have.

---

## What you get

Arda currently concentrates on the physical world: relief, climate,
drainage, rivers, and lakes. It produces saved data as well as images.

| Layer | What it contains |
| --- | --- |
| Continent | Coarse relief shaped by tectonics, plus climate and drainage on a 1 km grid |
| Areas | 512 × 512 cells at 100 m spacing: 51.2 × 51.2 km per area, with terrain, rainfall, temperature, and water data |
| Water features | Shared river reaches and lakes with IDs, geometry, water surfaces, and annual flow information |
| Tactical blocks | Experimental 64 × 64 grids of five-foot tiles, sampled at selected land cells; this layer is unfinished |

The default requested extent is 500 × 1000 km. `--micro` selects a
102 × 204 km request with eight exported areas. Custom extents use
`--size WIDTHxHEIGHT`:

```sh
arda generate --seed 42 --size 500x1000 --out worlds/continent42
```

Requested kilometre extents and the exported area grid are not identical:
area counts currently use integer 51 km units, while each area spans
51.2 km. See [operations](docs/capstone/07-operations.md) for the exact
modeled bounds and resource limits.

### How the world takes shape

```text
Seed + configuration
        ↓
Continent relief and climate
        ↓
Shared terrain evolution across area boundaries
        ↓
Connected drainage and annual water balance
        ↓
Saved areas, water features, and sampled blocks
        ↓
PNG maps · JSON exports · Rust queries
```

**Terrain comes before water.** Fine relief follows local regional height
differences. Erosion evolves across internal area boundaries, so those
boundaries do not become fixed terrain walls.

**Lakes need a basin and a water supply.** The water model combines
physical depressions with rainfall, runoff, and evaporation. River and
lake records are shared across areas, rather than reconstructed separately
by each map export.

**The saved world is the source for exports.** PNG and JSON exports read
persisted terrain and water geometry. Increasing image resolution changes
the rendering, not the underlying terrain samples.

**Generation is reproducible.** A fixed seed, configuration, and generator
revision determine the output. CI checks a shared golden world on Linux,
macOS, and Windows. Keep the generator revision with a seed when you need
to reproduce an older world; changes to generation can change its output.

## Map and data exports

### Native world data: BIN

`arda generate` automatically writes the native saved world. Its `.bin`
files carry terrain, climate, cells, and water records; sampled tactical
blocks use `.tiles.zst` archives. No separate `--format bin` export is
needed.

```text
worlds/w42/
├── world.json                 # Manifest, seed, configuration, format version
├── continent/
│   ├── overview.bin           # Coarse terrain, climate, and drainage
│   └── objects.bin
├── areas/<ax>_<ay>/
│   ├── cells.bin              # Full saved area cell records
│   └── objects.bin            # Local objects and shared water context
├── hydrology/                 # Global .bin tables: lakes, reaches, basins, ...
└── blocks/<ax>_<ay>.tiles.zst  # Compressed sampled tactical blocks
```

`World::load` opens this directory through the Rust API. Preserve the
directory structure when moving a world between a desktop, container,
or backend server.

### Maps and interchange data: PNG and JSON

| Output | Command options | Result |
| --- | --- | --- |
| World overview | `--overview` | `overview.png`, 8K long edge by default |
| Area map | `--area 0,0` | `area_00_00.png`, 8192 × 8192 by default |
| Small area preview | `--area 0,0 --quality 512` | `area_00_00.png`, 512 × 512 |
| High-resolution area | `--area 0,0 --quality 32k` | `area_00_00.png`, 32768 × 32768 |
| Area data | `--area 0,0 --format json` | `area_00_00.json`, versioned cell and object data |
| Sampled block | `--block AX,AY,CX,CY --format png` or `json` | One saved experimental tactical block, if present |

Each row uses `arda export --world WORLD_DIR ... --out EXPORT_DIR`.
PNG and JSON are separate exports. `--quality` applies to area and overview
PNGs; it does not apply to JSON or tactical blocks.

Choose `--style atlas` for earthy elevation colours, directional relief shading
and sea-depth colours. Atlas interpolates terrain colour and lighting at every
output pixel while keeping the saved land/water masks and channel geometry.
Omitting `--style`, or choosing `--style classic`, retains the original render.
Explicit styles apply only to area/overview PNGs, not JSON or tactical blocks.

```sh
arda export --world worlds/w42 --overview --style atlas --quality 8k --out exports/w42-atlas
arda export --world worlds/w42 --area 1,1 --style atlas --quality 32k --out exports/w42-atlas
```

Quality accepts any whole-pixel size from **512 through 32768**, including
`512`, `1k`, `2k`, `4k`, `8k`, `16k`, and `32k`. A `k` is 1024 pixels.
The default is **8K**. Areas are square; overviews preserve the exported
area grid's aspect ratio and use the selected quality for the long edge.

Detailed area maps still use the same **100 m terrain grid**. Their
additional pixels make channel geometry easier to inspect and smooth Atlas
colour/lighting transitions; they do not add finer terrain measurements. Overview river strokes are symbolic and
should not be read as physical channel widths. The
[map legend](docs/map-legend.md) explains the terrain colours and water
presentation.

Area JSON includes selected terrain and water fields for every cell, plus
water objects. Temperature and rainfall are saved in the world and
available through the Rust API, but are not yet included in area JSON.
The saved world stores binary layers for loading and querying; JSON
provides an explicit interchange format. A full area JSON can be tens of
megabytes, so applications should choose the areas and fields they need. Browser
loading and network delivery are not implemented or benchmarked here.

<details>
<summary>Choose a resolution up to 32K</summary>

Use the same quality option for a saved world or a single area:

```sh
arda export --world worlds/w42 --overview --quality 32k --out exports/w42-32k
arda export --world worlds/w42 --area 1,1 --quality 16k --out exports/w42-16k
```

Both paths stream PNG output using rows or bands, instead of keeping the
whole uncompressed image in memory. Larger exports still require more
rendering time and output space. Completed images replace the previous
file only after rendering succeeds.

For compatibility, Classic `--detail` retains the older 4096 × 4096 area
export and its `_detail.png` filename. With `--style atlas`, `--detail` uses
the streamed 4096 × 4096 path and the standard `area_XX_YY.png` filename.
It cannot be combined with `--quality`. The preview command also retains
`--px` for pixels per area; it conflicts with `--quality` and any explicit
`--style`. Atlas preview uses `--style atlas --quality SIZE`.

</details>

<details>
<summary>Read an experimental tactical block</summary>

`--block` takes four coordinates: area X, area Y, cell X, cell Y.
Blocks are saved only at land cells whose local X and Y are multiples of
64, from 0 through 448. A coordinate such as `0,0,64,128` is a valid
request shape, but a block exists there only if that sampled cell is land.

The current tile vocabulary and adjacency rules are preliminary. These
exports do not yet provide finished maps with detailed assets, movement
rules, collision geometry, or NPCs.

</details>

## Rust library

The `arda` crate is the public entry point for generation, loading,
queries, and exports. Within a checkout, another workspace crate can use
`arda = { path = "../arda" }`.

```rust,no_run
use arda::{GenerateConfig, World};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("worlds/library42");
    arda::generate(42, GenerateConfig::MICRO, path)?;

    let world = World::load(path)?;
    let area = world.read_area(0, 0)?;
    let cell = area.cell(100, 100)?;
    println!("Elevation: {} mm", cell.height.raw());
    Ok(())
}
```

`World::load` reads the manifest without eagerly loading every area.
`area()` caches requested areas; `read_area()` returns an owned, uncached
read. `generate_with_limits()` accepts explicit resource limits and
rejects work that exceeds them, without changing the physical model to
fit.

Generate local API documentation with:

```sh
cargo doc --workspace --no-deps --open
```

## Docker and server use

Arda can generate and export worlds on a server using the same CLI or
Rust API. The Docker image packages that CLI. Build it locally, then
mount a writable world directory:

```sh
docker build -t arda-local .
docker run --rm -v /absolute/path/to/worlds:/worlds arda-local \
  generate --seed 42 --micro --out /worlds/w42

docker run --rm -v /absolute/path/to/worlds:/worlds arda-local \
  export --world /worlds/w42 --area 1,1 --format json --out /worlds/exports/w42
```

The container runs as UID 10001; the mounted directory must be writable by
that user. Native `.bin` layers, block archives, and exported files remain
on the mounted volume after the container exits.

A backend can load the saved world through the Rust API or deliver the
exported files through its own HTTP service. Arda's built-in read-only
HTTP server (`arda serve`) is [planned](docs/capstone/mockup/06-serve.md);
the current binary implements `generate`, `preview`, and `export` only.
The container does not currently start an HTTP listener.

---

## Current status

**Working physical-world generator, with further world content in development.**
Status updated on 2026-09-22 against source commit `342d03e55120`.

| Implemented | What is available now |
| --- | --- |
| Physical geography | Continental relief and climate; shared 100 m terrain evolution; connected drainage, rivers, lakes and representative annual water accounting |
| Area data | Elevation, temperature, rainfall, slope, drainage, channel properties and wetness, plus basic Bare/Grass/Marsh ground cover |
| Terrain corrections | Erosion across area boundaries and the regional-relief correction that removed the recorded repeated lake-district artifact |
| Persistence and loading | Format-4 saved worlds, manifest-first loading, requested area/archive caches and bounded owned area reads for exports |
| Maps and interchange | Classic and shaded Atlas PNGs; per-output-pixel Atlas colour/light interpolation; versioned JSON; area/world image quality 512–32K, default 8K, with streamed quality PNG publication |
| Developer tools | `generate`, `preview` and `export` CLI commands, a reusable Rust API, Docker execution and regression/CI infrastructure |
| Tactical prototype | Seeded, persisted 64×64 blocks with a preliminary 24-tile WFC vocabulary and symbolic exports |

| Remaining work or model boundary | Current boundary |
| --- | --- |
| Terrain realism | First Atlas comparison shows smoother relief; straight/parallel drainage shapes and 100 m shoreline steps remain visible. Wider terrain acceptance and historical rectangular-basin diagnosis remain open |
| Water model scope | Annual water is implemented; seasonal storage, groundwater, snowpack and dynamic floods are outside the current model |
| Rich vegetation and human geography | Full vegetation/forest density, soil moisture, settlements, roads, buildings, naming, realms and NPC generation remain deferred |
| Tactical maps | Sparse sampled blocks with a preliminary WFC tile set; detailed assets and gameplay geometry remain deferred |
| Serving and integration | No `arda serve` command or browser application; consumers use saved exports or the Rust API |

The Atlas first pass passed **648 workspace tests** on Linux (8 ignored),
formatting, strict Clippy, Rust 1.96.1 checks and the dependency audit. Repeated
32K area/overview exports were byte-identical; saved world files were unchanged.
Three independent comparisons against the pre-change executable preserved Classic
PNG bytes. The visual panel uses one retained seed-42 200 × 300 km world; it
does not complete the broader terrain-realism acceptance.

The earlier 32K export commit records 611 workspace tests and 74 targeted checks;
these are historical verification results, not a fresh test run. The last checked
[CI run for base commit `23dfb0999672`](https://github.com/DiceMasterIO/arda/actions/runs/34260968213)
passed Linux/macOS tests, lint, MSRV and dependency checks, but failed the
Windows golden-text comparison on LF versus CRLF line endings. Its 34 logged
fingerprint entries match after normalization; that test failure remains open.
The detailed [status audit](docs/capstone/open-items.md) separates implemented
work, verification evidence and unresolved visual acceptance.

Generation is an offline batch. Valid size inputs can still exceed
resource limits. Failures after generation begins can leave partial output
for diagnosis; admission failures leave the destination untouched.
There is no checkpoint resume; retry in a fresh directory. Incompatible
saved-world format versions require regeneration.

See [open items](docs/capstone/open-items.md) for the detailed status and
[operations](docs/capstone/07-operations.md) for capacity and persistence
behaviour.

## Development

The workspace separates the model, generation, rendering, public API, and
CLI:

| Crate | Responsibility |
| --- | --- |
| [`arda-core`](crates/arda-core) | Coordinates, domain types, deterministic RNG, and storage formats |
| [`arda-gen`](crates/arda-gen) | Continent, terrain, water, area, and block generation |
| [`arda-render`](crates/arda-render) | PNG rendering and JSON serialization |
| [`arda`](crates/arda) | Public generation, loading, query, and export API |
| [`arda-cli`](crates/arda-cli) | The `arda` command |

Run the main local checks:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-features
```

CI also checks the golden world across three operating systems, the
minimum supported Rust version, and dependency licenses/advisories.
Generation tests do real work and can take substantially longer than
ordinary unit tests.

Issues and pull requests are welcome. For generation or rendering bugs,
include the seed, configuration, generator revision, exact command, and a
map or data sample. Follow the [coding standards](docs/capstone/standards.md)
and add a regression test for fixes. Golden fingerprints should change
only when an intended generation change has been reviewed.

## Documentation

| Document | Use it for |
| --- | --- |
| [Map legend](docs/map-legend.md) | Reading terrain colours, hillshade, rivers, and lakes |
| [Reference index](docs/capstone/00-index.md) | Navigating the implementation |
| [Architecture](docs/capstone/01-architecture.md) | Crate boundaries and generation stages |
| [Data models](docs/capstone/02-models.md) | Cells, water features, and saved formats |
| [Operations](docs/capstone/07-operations.md) | CLI behaviour, resource limits, and exports |
| [Open items](docs/capstone/open-items.md) | Implemented work and remaining limitations |
| [Testing](docs/capstone/06-testing.md) | Determinism, physical checks, and verification scope |

---

## License

[Apache-2.0](LICENSE). See [NOTICE](NOTICE) for project attribution.
