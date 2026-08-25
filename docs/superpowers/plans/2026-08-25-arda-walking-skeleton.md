# arda Walking Skeleton Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `arda` build-order steps 0–3 — a Rust workspace that generates a micro-continent (8 area tiles) end to end, writes it to disk, loads it back, exports PNG + JSON, and proves byte-identical output on Linux/macOS/Windows.

**Architecture:** Five-crate Cargo workspace. `arda-core` holds shared newtypes, the subseeded PRNG, and the *only* byte codec in the tree. `arda-gen` holds pure stage functions (continent → area → block) driven by one orchestrator. `arda-render` reads stored worlds only. `arda` is the facade; `arda-cli` is a thin binary. Every sim value is integer or fixed-point, every random draw is keyed by `(seed, tier, stage, coords, attempt)` — so the world is a pure function of seed + config on any platform.

**Tech Stack:** Rust (stable), `zstd`, `png`, `serde`/`serde_json`, `rand_chacha`, `rayon`, `clap` v4, `blake3`, `thiserror`, `criterion`, `cargo-deny`.

## Global Constraints

Every task's requirements implicitly include this section. Values copied verbatim from the spec.

- `#![deny(unsafe_code)]` workspace-wide. (`code-prefs.md` §Q1)
- `#![deny(missing_docs)]` on published crates; document the public API fully. (§Q1)
- `unwrap`/`expect` banned outside `#[cfg(test)]`. (§Q1)
- Domain newtypes for every coordinate and identifier kind — never bare `u32`s that can cross grids. (§Q1)
- Enums over raw strings/ints for every closed set. (§Q1)
- Sim cores use integer/fixed-point arithmetic or strictly-ordered IEEE float ops; no fast-math, no platform-dependent intrinsics or libm variance in sim paths. (`architecture-interview.md` §Q4 — **one-way door**)
- Every random draw comes from the counter-based PRNG keyed `(seed, tier, stage, coords, attempt)` — never from iteration order. Ambient randomness (`thread_rng`) and wall-clock time are forbidden everywhere in sim paths. (§Q4, `code-prefs.md` §Q9)
- Parallel reductions must be order-independent or explicitly ordered. (§Q4)
- Determinism-critical code — erosion, hydrology, WFC, PRNG usage, noise — is hand-rolled. Never delegate sim arithmetic to a third-party crate. (`code-prefs.md` §Q2)
- Per-crate error enums, `thiserror`-style derives; no `anyhow` in library crates — `arda-cli` only. (§Q4)
- Never swallow an error: propagate, or log at the CLI boundary. No logging framework inside library crates. (§Q4)
- Package by stage/feature mirroring the pipeline; no `utils/` or `types.rs` dumping grounds. Files ≤ ~500 lines, soft. (§Q5)
- Comments state the why and cite the rule implemented (`// logic/02 §floodplain`); never restate signatures. (§Q5)
- Test-first: write the failing unit test from the logic doc before the code. **No mocks, ever** — pure functions and temp dirs suffice. (§Q6)
- rustfmt defaults, no custom config. clippy at `-D warnings`. MSRV = latest-stable-minus-2, checked in CI. (§Q7)
- Conventional commits, imperative subject ≤ 72 chars. (§Q8)
- World formats live **only** in `arda-core::formats`; no other crate encodes/decodes bytes. (`01-architecture.md`)
- `arda-render` never depends on `arda-gen` — rendering reads stored worlds only. (`01-architecture.md` §Q2)
- Whole-tree soft cap ~40 crates. (`code-prefs.md` §Q2)
- Dependency version floors: `zstd` 0.13, `png` 0.17, `serde`/`serde_json` 1.0, `rand_chacha` 0.3, `rayon` 1.10, `clap` 4.5. (`05-dependencies.md`)
- `serde_json` uses `BTreeMap` where maps are unavoidable — byte-identity. (`05-dependencies.md` §Q3)
- `FORMAT_VERSION` lives in `world.json`; loaders refuse newer majors with the regenerate remedy. (`logic/05`)

### Recorded divergences from `05-dependencies.md`

These are dependency additions this plan makes that the dependencies chapter does not yet list. **Confirm before Task 1; if rejected, see the fallback.**

| Crate | Why | License | Fallback if rejected |
|---|---|---|---|
| `blake3` | `implementation.md`'s rng sketch derives the ChaCha8 key via `blake3(world_seed ‖ key)`. The chapter lists blake3 as dev/tooling only, but the seam needs it at runtime. | CC0/Apache-2.0 | Hand-roll a SplitMix64-based key derivation in `arda-core::rng` (no dependency) |
| `thiserror` | `code-prefs.md` §Q4 mandates "thiserror-style derives"; the chapter lists no error crate. | MIT/Apache-2.0 | Hand-write `Display` + `std::error::Error` impls (~10 lines per enum) |
| `anyhow` | `code-prefs.md` §Q4 explicitly permits it in `arda-cli` only. | MIT/Apache-2.0 | Return `Box<dyn Error>` from `main` |

Both additions clear `code-prefs.md` §Q2's vetting bar (permissive license, maintained > 1 year, no platform-variant arithmetic). Tree total stays well under the ~40-crate cap.

---

## File Structure

Paths follow `implementation.md`'s Workspace layout. Only files this plan creates are listed; later build-order steps fill the rest.

**Workspace root**
- `Cargo.toml` — `[workspace]`, `members = ["crates/*"]`, shared `[workspace.dependencies]` and `[workspace.lints]`
- `deny.toml` — cargo-deny license + advisory config
- `Dockerfile` — multi-arch, entrypoint `arda`
- `rust-toolchain.toml` — pinned stable channel
- `.github/workflows/ci.yml` — test + clippy + fmt + deny on ubuntu/macos/windows
- `tests/golden_world.rs` — workspace-level per-stage blake3 hashes (the §Q4 gate)

**`crates/arda-core/src/`** — shared types, PRNG, the only byte codec
- `lib.rs` — crate lints, module tree, re-exports
- `coords.rs` — `AreaCoord`, `CellCoord`, `SquareCoord`, `ContinentCoord`
- `fixed.rs` — Q-format newtypes: `HeightMm`, `TempCentiC`, `RainfallMm`, `DischargeMilli`
- `rng.rs` — `Tier`, `Stage`, `SeedKey`, `rng()` — the determinism spine
- `error.rs` — `CoreError`, `LoadError`, `FormatError`
- `config.rs` — `GenerateConfig`, `SizeKm`, `LatitudeBand`, validation
- `cell.rs` — the `Cell` struct: the artifact's per-cell field list
- `objects.rs` — `RiverSegment`, `Lake`, `Settlement` (skeleton subset)
- `tiles.rs` — `TileId(u16)` + the 24-tile skeleton vocabulary + adjacency
- `formats/mod.rs` — codec module root, `FORMAT_VERSION`
- `formats/manifest.rs` — `world.json`
- `formats/cells.rs` — `cells.bin` fixed little-endian layout
- `formats/objects.rs` — `objects.bin`
- `formats/blocks.rs` — `<ax>_<ay>.tiles.zst`

**`crates/arda-gen/src/`** — pure stages + orchestrator
- `lib.rs` — `generate_world` orchestrator, rayon fan-out
- `continent/mod.rs`, `plates.rs`, `tectonics.rs`, `coast.rs`, `bundles.rs`
- `area/mod.rs`, `relief.rs`, `water.rs`
- `block/mod.rs`, `wfc.rs`
- `noise.rs` — hand-rolled integer value noise (shared by continent + area)

**`crates/arda-render/src/`** — reads stored worlds only
- `lib.rs`, `symbolic.rs` (block PNG), `carto.rs` (area PNG), `json.rs` (versioned export)

**`crates/arda/src/lib.rs`** — facade: `World`, `Area`, `Cell`, `Block` views

**`crates/arda-cli/src/main.rs`** — clap: `generate` | `export`

---

### Task 1: Workspace scaffold, lints, and CI

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `deny.toml`, `Dockerfile`, `.gitignore`
- Create: `crates/arda-core/Cargo.toml`, `crates/arda-core/src/lib.rs`
- Create: `crates/arda-gen/Cargo.toml`, `crates/arda-gen/src/lib.rs`
- Create: `crates/arda-render/Cargo.toml`, `crates/arda-render/src/lib.rs`
- Create: `crates/arda/Cargo.toml`, `crates/arda/src/lib.rs`
- Create: `crates/arda-cli/Cargo.toml`, `crates/arda-cli/src/main.rs`
- Create: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: nothing (first task)
- Produces: the five crate names (`arda-core`, `arda-gen`, `arda-render`, `arda`, `arda-cli`) and the workspace lint table every later task inherits via `[lints] workspace = true`

- [ ] **Step 1: Write the failing test**

Create `crates/arda-core/src/lib.rs`:

```rust
//! Shared types, the subseeded PRNG, and the only byte codec in the workspace.
//!
//! See `docs/capstone/01-architecture.md` for the crate boundary rules.

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_builds_and_links() {
        assert_eq!(env!("CARGO_PKG_NAME"), "arda-core");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --workspace`
Expected: FAIL — `error: failed to load manifest` / `could not find Cargo.toml`, because no workspace manifest exists yet.

- [ ] **Step 3: Write minimal implementation**

`rust-toolchain.toml`:

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

Root `Cargo.toml` — the dependency floors from `05-dependencies.md` and the lint table from `code-prefs.md` §Q1/§Q7 live here once, inherited by every crate:

```toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
repository = "https://github.com/gentbajko/arda"

[workspace.dependencies]
arda-core = { path = "crates/arda-core", version = "0.1.0" }
arda-gen = { path = "crates/arda-gen", version = "0.1.0" }
arda-render = { path = "crates/arda-render", version = "0.1.0" }
arda = { path = "crates/arda", version = "0.1.0" }
zstd = "0.13"
png = "0.17"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rand_chacha = "0.3"
rand_core = "0.6"
rayon = "1.10"
clap = { version = "4.5", features = ["derive"] }
blake3 = "1.5"
thiserror = "1.0"
anyhow = "1.0"
criterion = "0.5"

[workspace.lints.rust]
unsafe_code = "deny"
missing_docs = "deny"

[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
module_name_repetitions = "allow"

[profile.release]
codegen-units = 1
```

`crates/arda-core/Cargo.toml`:

```toml
[package]
name = "arda-core"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "Shared types, PRNG, and world formats for arda"

[dependencies]
serde.workspace = true
serde_json.workspace = true
rand_chacha.workspace = true
rand_core.workspace = true
blake3.workspace = true
thiserror.workspace = true
zstd.workspace = true

[lints]
workspace = true
```

`crates/arda-gen/Cargo.toml` — note it depends on `arda-core` only, per `01-architecture.md`:

```toml
[package]
name = "arda-gen"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "Continent, area, and block generation stages for arda"

[dependencies]
arda-core.workspace = true
rand_chacha.workspace = true
rand_core.workspace = true
rayon.workspace = true
thiserror.workspace = true

[lints]
workspace = true
```

`crates/arda-render/Cargo.toml` — **no `arda-gen` dependency**; rendering reads stored worlds only:

```toml
[package]
name = "arda-render"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "PNG and JSON export for arda worlds"

[dependencies]
arda-core.workspace = true
png.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true

[lints]
workspace = true
```

`crates/arda/Cargo.toml`:

```toml
[package]
name = "arda"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "Deterministic procedural worldgen for tabletop"

[dependencies]
arda-core.workspace = true
arda-gen.workspace = true
arda-render.workspace = true

[lints]
workspace = true
```

`crates/arda-cli/Cargo.toml` — the only crate allowed `anyhow`:

```toml
[package]
name = "arda-cli"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "The arda command-line binary"

[[bin]]
name = "arda"
path = "src/main.rs"

[dependencies]
arda.workspace = true
clap.workspace = true
anyhow.workspace = true

[lints]
workspace = true
```

Placeholder crate roots — each carries its own doc comment so `missing_docs` passes:

`crates/arda-gen/src/lib.rs`:

```rust
//! Generation stages: continent (`logic/01`), area (`logic/02`), block (`logic/03`).
//!
//! Every stage is a pure function over prior stages' outputs; the
//! orchestrator is the only caller that touches disk.
```

`crates/arda-render/src/lib.rs`:

```rust
//! PNG and JSON export. Reads stored worlds only — never depends on `arda-gen`.
```

`crates/arda/src/lib.rs`:

```rust
//! The public facade for arda. Consumers depend on this crate only.
```

`crates/arda-cli/src/main.rs`:

```rust
//! The `arda` binary: `generate` and `export` subcommands.

fn main() {
    println!("arda {}", env!("CARGO_PKG_VERSION"));
}
```

`deny.toml` — enforces `code-prefs.md` §Q2's permissive-license bar:

```toml
[licenses]
allow = ["MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception", "CC0-1.0", "Unicode-DFS-2016", "BSD-3-Clause", "ISC"]
confidence-threshold = 0.9

[bans]
multiple-versions = "warn"

[advisories]
yanked = "deny"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

`Dockerfile` — multi-arch, entrypoint `arda`, no exposed ports (`07-operations.md`):

```dockerfile
FROM rust:1-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release -p arda-cli

FROM debian:bookworm-slim
RUN useradd --create-home --uid 10001 arda
COPY --from=build /src/target/release/arda /usr/local/bin/arda
VOLUME ["/worlds"]
USER arda
ENTRYPOINT ["arda"]
```

`.gitignore`:

```gitignore
/target
**/*.rs.bk
```

`.github/workflows/ci.yml` — the three-OS matrix that later becomes the §Q4 determinism gate:

```yaml
name: ci
on: [push, pull_request]

jobs:
  test:
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --workspace --all-features

  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - run: cargo fmt --all --check
      - run: cargo clippy --workspace --all-targets -- -D warnings

  deny:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v2
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --workspace`
Expected: PASS — `test tests::workspace_builds_and_links ... ok`, five crates compiled.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings.

Run: `cargo fmt --all --check`
Expected: no output, exit 0.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml rust-toolchain.toml deny.toml Dockerfile .gitignore crates .github
git commit -m "feat: scaffold five-crate workspace with lints and CI"
```

---

### Task 2: Coordinate and fixed-point newtypes

**Files:**
- Create: `crates/arda-core/src/coords.rs`
- Create: `crates/arda-core/src/fixed.rs`
- Modify: `crates/arda-core/src/lib.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks
- Produces:
  - `AreaCoord { x: i32, y: i32 }`, `CellCoord { x: u16, y: u16 }`, `SquareCoord { x: u8, y: u8 }`, `ContinentCoord { x: i32, y: i32 }` — all `Copy + Eq + Ord + Hash + Debug`
  - `CellCoord::new(x, y) -> Option<Self>` (bounds-checked against `AREA_CELLS`), `CellCoord::index(self) -> usize`
  - `SquareCoord::new(x, y) -> Option<Self>`, `SquareCoord::index(self) -> usize`
  - Constants `AREA_CELLS: u16 = 512`, `BLOCK_SQUARES: u8 = 64`, `CELL_SIZE_M: i32 = 100`, `SQUARE_SIZE_MM: i32 = 1524`
  - `HeightMm(i32)`, `TempCentiC(i16)`, `RainfallMm(u16)`, `DischargeMilli(u32)` — each with `new`, `raw`, and a documented unit

- [ ] **Step 1: Write the failing test**

Append to `crates/arda-core/src/coords.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_coord_rejects_out_of_bounds() {
        assert!(CellCoord::new(511, 511).is_some());
        assert!(CellCoord::new(512, 0).is_none());
        assert!(CellCoord::new(0, 512).is_none());
    }

    #[test]
    fn cell_coord_index_is_row_major() {
        let c = CellCoord::new(3, 2).unwrap();
        assert_eq!(c.index(), 2 * 512 + 3);
    }

    #[test]
    fn square_coord_rejects_out_of_bounds() {
        assert!(SquareCoord::new(63, 63).is_some());
        assert!(SquareCoord::new(64, 0).is_none());
    }

    #[test]
    fn square_coord_index_is_row_major() {
        let s = SquareCoord::new(5, 4).unwrap();
        assert_eq!(s.index(), 4 * 64 + 5);
    }
}
```

Append to `crates/arda-core/src/fixed.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_round_trips_millimetres() {
        let h = HeightMm::new(1_524_000);
        assert_eq!(h.raw(), 1_524_000);
        assert_eq!(h.whole_metres(), 1524);
    }

    #[test]
    fn height_below_sea_level_is_negative() {
        assert_eq!(HeightMm::new(-2_000_000).whole_metres(), -2000);
    }

    #[test]
    fn temp_holds_the_habitable_range() {
        assert_eq!(TempCentiC::new(-4000).whole_degrees(), -40);
        assert_eq!(TempCentiC::new(5000).whole_degrees(), 50);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p arda-core`
Expected: FAIL — `error[E0433]: failed to resolve: use of undeclared crate or module 'coords'` (the modules are not declared in `lib.rs` yet, and the types do not exist).

- [ ] **Step 3: Write minimal implementation**

Prepend to `crates/arda-core/src/coords.rs`:

```rust
//! The two-grid coordinate system (`01-architecture.md`).
//!
//! Distinct newtypes per grid so a cell index can never be used as a
//! square index — `code-prefs.md` §Q1 bans bare integers that cross grids.

/// Cells along one edge of an area tile.
pub const AREA_CELLS: u16 = 512;
/// Squares along one edge of a block.
pub const BLOCK_SQUARES: u8 = 64;
/// Ground size of one area cell, in metres.
pub const CELL_SIZE_M: i32 = 100;
/// Ground size of one tactical square, in millimetres (five feet exactly).
pub const SQUARE_SIZE_MM: i32 = 1524;

/// Index of a 51.2 km area tile within the continent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AreaCoord {
    /// Tile column.
    pub x: i32,
    /// Tile row.
    pub y: i32,
}

impl AreaCoord {
    /// Builds a tile index.
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Directory name for this tile, as `<ax>_<ay>` (`mockup/02`).
    #[must_use]
    pub fn dir_name(self) -> String {
        format!("{:02}_{:02}", self.x, self.y)
    }
}

/// Index of a 100 m cell within one area tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CellCoord {
    x: u16,
    y: u16,
}

impl CellCoord {
    /// Builds a cell index, or `None` when either axis is outside the tile.
    #[must_use]
    pub const fn new(x: u16, y: u16) -> Option<Self> {
        if x < AREA_CELLS && y < AREA_CELLS {
            Some(Self { x, y })
        } else {
            None
        }
    }

    /// Cell column.
    #[must_use]
    pub const fn x(self) -> u16 {
        self.x
    }

    /// Cell row.
    #[must_use]
    pub const fn y(self) -> u16 {
        self.y
    }

    /// Row-major offset into a `512 * 512` array.
    #[must_use]
    pub const fn index(self) -> usize {
        self.y as usize * AREA_CELLS as usize + self.x as usize
    }
}

/// Index of a five-foot square within one block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SquareCoord {
    x: u8,
    y: u8,
}

impl SquareCoord {
    /// Builds a square index, or `None` when either axis is outside the block.
    #[must_use]
    pub const fn new(x: u8, y: u8) -> Option<Self> {
        if x < BLOCK_SQUARES && y < BLOCK_SQUARES {
            Some(Self { x, y })
        } else {
            None
        }
    }

    /// Square column.
    #[must_use]
    pub const fn x(self) -> u8 {
        self.x
    }

    /// Square row.
    #[must_use]
    pub const fn y(self) -> u8 {
        self.y
    }

    /// Row-major offset into a `64 * 64` array.
    #[must_use]
    pub const fn index(self) -> usize {
        self.y as usize * BLOCK_SQUARES as usize + self.x as usize
    }
}

/// Index of a 1 km cell on the continent working grid (`logic/01` step 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContinentCoord {
    /// Column on the 1 km grid.
    pub x: i32,
    /// Row on the 1 km grid.
    pub y: i32,
}

impl ContinentCoord {
    /// Builds a continent-grid index.
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}
```

Prepend to `crates/arda-core/src/fixed.rs`:

```rust
//! Q-format fixed-point newtypes (`05-dependencies.md` §Q7).
//!
//! Sim-facing values are integers so the world is bit-identical across
//! platforms — `architecture-interview.md` §Q4 forbids float variance in
//! sim paths.

/// Elevation in millimetres above sea level; negative below.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct HeightMm(i32);

impl HeightMm {
    /// Sea level.
    pub const SEA_LEVEL: Self = Self(0);

    /// Wraps a raw millimetre count.
    #[must_use]
    pub const fn new(mm: i32) -> Self {
        Self(mm)
    }

    /// The raw millimetre count.
    #[must_use]
    pub const fn raw(self) -> i32 {
        self.0
    }

    /// Truncated whole metres.
    #[must_use]
    pub const fn whole_metres(self) -> i32 {
        self.0 / 1000
    }
}

/// Temperature in hundredths of a degree Celsius.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TempCentiC(i16);

impl TempCentiC {
    /// Wraps a raw hundredth-degree count.
    #[must_use]
    pub const fn new(centi: i16) -> Self {
        Self(centi)
    }

    /// The raw hundredth-degree count.
    #[must_use]
    pub const fn raw(self) -> i16 {
        self.0
    }

    /// Truncated whole degrees Celsius.
    #[must_use]
    pub const fn whole_degrees(self) -> i16 {
        self.0 / 100
    }
}

/// Annual rainfall in millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RainfallMm(u16);

impl RainfallMm {
    /// Wraps a raw millimetre count.
    #[must_use]
    pub const fn new(mm: u16) -> Self {
        Self(mm)
    }

    /// The raw millimetre count.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }
}

/// River discharge in thousandths of a cubic metre per second.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DischargeMilli(u32);

impl DischargeMilli {
    /// Wraps a raw thousandth-cumec count.
    #[must_use]
    pub const fn new(milli: u32) -> Self {
        Self(milli)
    }

    /// The raw thousandth-cumec count.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}
```

Replace `crates/arda-core/src/lib.rs` with:

```rust
//! Shared types, the subseeded PRNG, and the only byte codec in the workspace.
//!
//! See `docs/capstone/01-architecture.md` for the crate boundary rules.

pub mod coords;
pub mod fixed;

pub use coords::{
    AreaCoord, CellCoord, ContinentCoord, SquareCoord, AREA_CELLS, BLOCK_SQUARES, CELL_SIZE_M,
    SQUARE_SIZE_MM,
};
pub use fixed::{DischargeMilli, HeightMm, RainfallMm, TempCentiC};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p arda-core`
Expected: PASS — 7 tests pass (`cell_coord_rejects_out_of_bounds`, `cell_coord_index_is_row_major`, `square_coord_rejects_out_of_bounds`, `square_coord_index_is_row_major`, `height_round_trips_millimetres`, `height_below_sea_level_is_negative`, `temp_holds_the_habitable_range`).

- [ ] **Step 5: Commit**

```bash
git add crates/arda-core/src
git commit -m "feat: add coordinate and fixed-point newtypes"
```

---

### Task 3: Subseeded PRNG — the determinism spine

Every random draw in the workspace flows through this module. `architecture-interview.md` §Q4 is a **one-way door**: get this wrong and the cross-platform gate in Task 15 fails everywhere at once.

**Files:**
- Create: `crates/arda-core/src/rng.rs`
- Modify: `crates/arda-core/src/lib.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks
- Produces:
  - `enum Tier { Continent, Area, Block }` and `enum Stage { Plates, Tectonics, Upsample, Relief, Water, Blocks, Bundles }` — both `#[repr(u8)]`, `Copy`
  - `struct SeedKey { tier: Tier, stage: Stage, x: i32, y: i32, attempt: u8 }` with `SeedKey::new(tier, stage, x, y, attempt)`
  - `fn rng(world_seed: u64, key: SeedKey) -> ChaCha8Rng`
  - `fn derive(world_seed: u64, key: SeedKey) -> [u8; 32]`

- [ ] **Step 1: Write the failing test**

Append to `crates/arda-core/src/rng.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::RngCore;

    fn draws(seed: u64, key: SeedKey) -> [u32; 4] {
        let mut r = rng(seed, key);
        [r.next_u32(), r.next_u32(), r.next_u32(), r.next_u32()]
    }

    #[test]
    fn same_key_yields_same_stream() {
        let k = SeedKey::new(Tier::Area, Stage::Relief, 3, 11, 0);
        assert_eq!(draws(42, k), draws(42, k));
    }

    #[test]
    fn every_field_changes_the_stream() {
        let base = SeedKey::new(Tier::Area, Stage::Relief, 3, 11, 0);
        let reference = draws(42, base);

        assert_ne!(draws(43, base), reference, "world seed must matter");
        assert_ne!(
            draws(42, SeedKey::new(Tier::Block, Stage::Relief, 3, 11, 0)),
            reference,
            "tier must matter"
        );
        assert_ne!(
            draws(42, SeedKey::new(Tier::Area, Stage::Water, 3, 11, 0)),
            reference,
            "stage must matter"
        );
        assert_ne!(
            draws(42, SeedKey::new(Tier::Area, Stage::Relief, 4, 11, 0)),
            reference,
            "x must matter"
        );
        assert_ne!(
            draws(42, SeedKey::new(Tier::Area, Stage::Relief, 3, 12, 0)),
            reference,
            "y must matter"
        );
        assert_ne!(
            draws(42, SeedKey::new(Tier::Area, Stage::Relief, 3, 11, 1)),
            reference,
            "attempt must matter"
        );
    }

    #[test]
    fn negative_coordinates_are_distinct_from_positive() {
        let a = draws(42, SeedKey::new(Tier::Continent, Stage::Plates, -3, -11, 0));
        let b = draws(42, SeedKey::new(Tier::Continent, Stage::Plates, 3, 11, 0));
        assert_ne!(a, b);
    }

    /// Reference vector: pins the byte layout of the derivation so a
    /// refactor cannot silently change every generated world.
    #[test]
    fn derivation_matches_reference_vector() {
        let key = derive(42, SeedKey::new(Tier::Area, Stage::Relief, 3, 11, 0));
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"arda-subseed-v1");
        hasher.update(&42u64.to_le_bytes());
        hasher.update(&[Tier::Area as u8, Stage::Relief as u8]);
        hasher.update(&3i32.to_le_bytes());
        hasher.update(&11i32.to_le_bytes());
        hasher.update(&[0u8]);
        assert_eq!(key, *hasher.finalize().as_bytes());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p arda-core rng`
Expected: FAIL — `error[E0432]: unresolved import` / `cannot find function 'rng' in this scope`; the module is not declared and the types do not exist.

- [ ] **Step 3: Write minimal implementation**

Prepend to `crates/arda-core/src/rng.rs`:

```rust
//! Counter-based subseed derivation (`architecture-interview.md` §Q4).
//!
//! Every random draw in arda comes from here, keyed by
//! `(seed, tier, stage, coords, attempt)` — never from iteration order, so
//! stage order and thread count cannot change a world.

use rand_chacha::ChaCha8Rng;
use rand_core::SeedableRng;

/// Domain separator. Bump only with a `format_version` major — changing it
/// changes every world ever generated.
const DOMAIN: &[u8] = b"arda-subseed-v1";

/// Which of the three tiers is drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Tier {
    /// The 1 km continent grid (`logic/01`).
    Continent = 1,
    /// A 51.2 km area tile (`logic/02`).
    Area = 2,
    /// A 64x64 tactical block (`logic/03`).
    Block = 3,
}

/// Which pipeline stage is drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Stage {
    /// Voronoi plate seeding (`logic/01` step 1).
    Plates = 1,
    /// Time-stepped tectonics (`logic/01` step 2).
    Tectonics = 2,
    /// 4 km to 1 km noise-refined upsample (`logic/01` step 4).
    Upsample = 3,
    /// Area relief detail (`logic/02`).
    Relief = 4,
    /// Area drainage (`logic/02`).
    Water = 5,
    /// Block WFC fill (`logic/03`).
    Blocks = 6,
    /// Per-tile input bundles (`logic/01` step 10).
    Bundles = 7,
}

/// The full key for one random stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SeedKey {
    tier: Tier,
    stage: Stage,
    x: i32,
    y: i32,
    attempt: u8,
}

impl SeedKey {
    /// Builds a stream key. `attempt` distinguishes rerolls
    /// (`logic/01` §Q9 continent rerolls, `logic/03` §Q12 WFC retries).
    #[must_use]
    pub const fn new(tier: Tier, stage: Stage, x: i32, y: i32, attempt: u8) -> Self {
        Self {
            tier,
            stage,
            x,
            y,
            attempt,
        }
    }

    /// Returns the same key at the next attempt number.
    #[must_use]
    pub const fn with_attempt(self, attempt: u8) -> Self {
        Self { attempt, ..self }
    }
}

/// Derives the 32-byte ChaCha8 key for one stream.
///
/// Fields are hashed in a fixed little-endian order so the derivation is
/// endianness-independent.
#[must_use]
pub fn derive(world_seed: u64, key: SeedKey) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(DOMAIN);
    hasher.update(&world_seed.to_le_bytes());
    hasher.update(&[key.tier as u8, key.stage as u8]);
    hasher.update(&key.x.to_le_bytes());
    hasher.update(&key.y.to_le_bytes());
    hasher.update(&[key.attempt]);
    *hasher.finalize().as_bytes()
}

/// Opens the deterministic stream for one key.
///
/// ChaCha8's output is defined bit-for-bit by the standard, so the same key
/// yields the same bytes on every platform — the §Q4 guarantee.
#[must_use]
pub fn rng(world_seed: u64, key: SeedKey) -> ChaCha8Rng {
    ChaCha8Rng::from_seed(derive(world_seed, key))
}
```

Add to `crates/arda-core/src/lib.rs`, after the `fixed` module declaration:

```rust
pub mod rng;
```

and extend the re-export block:

```rust
pub use rng::{derive, rng, SeedKey, Stage, Tier};
```

Add `blake3` to `crates/arda-core`'s dev usage — it is already a normal dependency from Task 1, so no manifest change is needed.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p arda-core rng`
Expected: PASS — 4 tests pass (`same_key_yields_same_stream`, `every_field_changes_the_stream`, `negative_coordinates_are_distinct_from_positive`, `derivation_matches_reference_vector`).

- [ ] **Step 5: Commit**

```bash
git add crates/arda-core/src
git commit -m "feat: add subseeded ChaCha8 PRNG keyed by tier stage and coords"
```

---

### Task 4: Error enums and generation config

**Files:**
- Create: `crates/arda-core/src/error.rs`
- Create: `crates/arda-core/src/config.rs`
- Modify: `crates/arda-core/src/lib.rs`

**Interfaces:**
- Consumes: `AreaCoord` (Task 2)
- Produces:
  - `enum ConfigError { SizeOutOfRange, LatitudeOutOfRange, DensityOutOfRange }` — each variant carries the offending value and the valid range
  - `enum FormatError { Io, UnexpectedEof, BadMagic, UnknownDiscriminant }`
  - `enum LoadError { ManifestMissing, ManifestUnreadable, VersionSkew { found, supported }, Corrupt { file }, OutOfRange { .. }, Format }`
  - `struct GenerateConfig { size_km: SizeKm, latitude_band: LatitudeBand, mean_density_per_km2: u16 }` with `GenerateConfig::new(...) -> Result<Self, ConfigError>`, `Default`, `areas_wide()`, `areas_high()`, `area_coords()`
  - `struct SizeKm { width: u32, height: u32 }`, `struct LatitudeBand { south_deg: i16, north_deg: i16 }`
  - `GenerateConfig::MICRO` — the 8-tile skeleton continent (`architecture-interview.md` §Q8)

- [ ] **Step 1: Write the failing test**

Append to `crates/arda-core/src/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_the_interviewed_continent() {
        let c = GenerateConfig::default();
        assert_eq!(c.size_km().width, 500);
        assert_eq!(c.size_km().height, 1000);
        assert_eq!(c.latitude_band().south_deg, 35);
        assert_eq!(c.latitude_band().north_deg, 55);
        assert_eq!(c.mean_density_per_km2(), 15);
    }

    #[test]
    fn micro_continent_has_eight_tiles() {
        // architecture-interview.md §Q8: skeleton is ~100x200 km, 8 tiles.
        let c = GenerateConfig::MICRO;
        assert_eq!(c.areas_wide(), 2);
        assert_eq!(c.areas_high(), 4);
        assert_eq!(c.area_coords().count(), 8);
    }

    #[test]
    fn area_coords_are_row_major_and_stable() {
        let coords: Vec<_> = GenerateConfig::MICRO.area_coords().collect();
        assert_eq!(coords[0], AreaCoord::new(0, 0));
        assert_eq!(coords[1], AreaCoord::new(1, 0));
        assert_eq!(coords[2], AreaCoord::new(0, 1));
        assert_eq!(coords[7], AreaCoord::new(1, 3));
    }

    #[test]
    fn rejects_size_outside_the_valid_range() {
        let err = GenerateConfig::new(SizeKm::new(10, 10), LatitudeBand::new(35, 55), 15)
            .unwrap_err();
        assert!(matches!(err, ConfigError::SizeOutOfRange { .. }));
    }

    #[test]
    fn rejects_inverted_latitude_band() {
        let err = GenerateConfig::new(SizeKm::new(500, 1000), LatitudeBand::new(55, 35), 15)
            .unwrap_err();
        assert!(matches!(err, ConfigError::LatitudeOutOfRange { .. }));
    }

    #[test]
    fn config_error_message_names_the_field_and_range() {
        // mockup/01 States: invalid config exits non-zero naming the field.
        let err = GenerateConfig::new(SizeKm::new(10, 10), LatitudeBand::new(35, 55), 15)
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("size"), "message was: {msg}");
        assert!(msg.contains("64"), "message was: {msg}");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p arda-core config`
Expected: FAIL — `cannot find struct 'GenerateConfig' in this scope`; module not declared.

- [ ] **Step 3: Write minimal implementation**

Prepend to `crates/arda-core/src/error.rs`:

```rust
//! Per-crate error enums (`code-prefs.md` §Q4).
//!
//! Load errors name the manifest problem, version skew carries both
//! versions, corruption names the file, range errors carry valid ranges
//! (`03-conventions.md`).

use thiserror::Error;

/// A `GenerateConfig` field outside its valid range (`logic/01` preconditions).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    /// Continent extent outside the supported range.
    #[error("size {width}x{height} km is out of range (each axis must be {min}-{max} km)")]
    SizeOutOfRange {
        /// Requested width in kilometres.
        width: u32,
        /// Requested height in kilometres.
        height: u32,
        /// Smallest supported axis.
        min: u32,
        /// Largest supported axis.
        max: u32,
    },
    /// Latitude band inverted or outside the supported range.
    #[error("latitude band {south}..{north} is out of range (south < north, both within -80..80)")]
    LatitudeOutOfRange {
        /// Requested southern edge.
        south: i16,
        /// Requested northern edge.
        north: i16,
    },
    /// Mean settlement density outside the supported range.
    #[error("density {value} people/km2 is out of range (must be {min}-{max})")]
    DensityOutOfRange {
        /// Requested density.
        value: u16,
        /// Smallest supported density.
        min: u16,
        /// Largest supported density.
        max: u16,
    },
}

/// A byte-level problem inside `arda-core::formats`.
#[derive(Debug, Error)]
pub enum FormatError {
    /// The underlying reader or writer failed.
    #[error("io error on {path}: {source}")]
    Io {
        /// File being read or written.
        path: String,
        /// Underlying cause.
        #[source]
        source: std::io::Error,
    },
    /// The stream ended before the layout was satisfied.
    #[error("{path} ended after {read} bytes, expected {expected}")]
    UnexpectedEof {
        /// File being read.
        path: String,
        /// Bytes actually available.
        read: usize,
        /// Bytes the layout requires.
        expected: usize,
    },
    /// The file does not start with its layer magic.
    #[error("{path} is not an arda {layer} layer")]
    BadMagic {
        /// File being read.
        path: String,
        /// Layer the caller expected.
        layer: &'static str,
    },
    /// A stored enum discriminant is not one this build knows.
    #[error("{path} holds unknown {field} discriminant {value}")]
    UnknownDiscriminant {
        /// File being read.
        path: String,
        /// Field being decoded.
        field: &'static str,
        /// The unrecognised byte.
        value: u16,
    },
}

/// A problem opening or reading a stored world (`logic/05`).
#[derive(Debug, Error)]
pub enum LoadError {
    /// `world.json` is absent — the batch never finished (`mockup/02` States).
    #[error("no world.json in {dir}: the world is missing or the batch did not finish")]
    ManifestMissing {
        /// Directory that was opened.
        dir: String,
    },
    /// `world.json` is present but unparseable.
    #[error("world.json in {dir} is unreadable: {reason}")]
    ManifestUnreadable {
        /// Directory that was opened.
        dir: String,
        /// Parser message.
        reason: String,
    },
    /// The world was written by an incompatible format major.
    #[error("world format {found} is not supported by this build ({supported}); regenerate from the seed")]
    VersionSkew {
        /// Major found in the manifest.
        found: u32,
        /// Major this build writes and reads.
        supported: u32,
    },
    /// A layer file is truncated or malformed.
    #[error("corrupt layer: {source}")]
    Corrupt {
        /// Underlying byte-level cause.
        #[from]
        source: FormatError,
    },
    /// A requested coordinate is outside the world.
    #[error("{what} {x},{y} is outside the world ({max_x},{max_y} is the last)")]
    OutOfRange {
        /// Kind of coordinate requested.
        what: &'static str,
        /// Requested column.
        x: i32,
        /// Requested row.
        y: i32,
        /// Last valid column.
        max_x: i32,
        /// Last valid row.
        max_y: i32,
    },
}
```

Prepend to `crates/arda-core/src/config.rs`:

```rust
//! Generation configuration and its validation (`logic/01` preconditions).

use crate::coords::AreaCoord;
use crate::error::ConfigError;
use serde::{Deserialize, Serialize};

/// Cells along one edge of an area tile, as kilometres of ground.
const AREA_SIZE_KM: u32 = 51;

const MIN_AXIS_KM: u32 = 64;
const MAX_AXIS_KM: u32 = 4000;
const MIN_DENSITY: u16 = 1;
const MAX_DENSITY: u16 = 200;

/// Continent extent in kilometres (`mockup/01` `--size`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SizeKm {
    /// East-west extent.
    pub width: u32,
    /// North-south extent.
    pub height: u32,
}

impl SizeKm {
    /// Builds a continent extent.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// The latitude belt the continent sits in (`logic/01` §Q6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatitudeBand {
    /// Southern edge, degrees north of the equator.
    pub south_deg: i16,
    /// Northern edge, degrees north of the equator.
    pub north_deg: i16,
}

impl LatitudeBand {
    /// Builds a latitude belt.
    #[must_use]
    pub const fn new(south_deg: i16, north_deg: i16) -> Self {
        Self {
            south_deg,
            north_deg,
        }
    }
}

/// A validated generation request. Construct via [`GenerateConfig::new`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerateConfig {
    size_km: SizeKm,
    latitude_band: LatitudeBand,
    mean_density_per_km2: u16,
}

impl GenerateConfig {
    /// The walking-skeleton continent: ~100x200 km, 8 tiles
    /// (`architecture-interview.md` §Q8).
    pub const MICRO: Self = Self {
        size_km: SizeKm::new(102, 204),
        latitude_band: LatitudeBand::new(35, 55),
        mean_density_per_km2: 15,
    };

    /// Validates and builds a configuration.
    ///
    /// # Errors
    /// Returns the offending field and its valid range (`mockup/01` States).
    pub fn new(
        size_km: SizeKm,
        latitude_band: LatitudeBand,
        mean_density_per_km2: u16,
    ) -> Result<Self, ConfigError> {
        if size_km.width < MIN_AXIS_KM
            || size_km.height < MIN_AXIS_KM
            || size_km.width > MAX_AXIS_KM
            || size_km.height > MAX_AXIS_KM
        {
            return Err(ConfigError::SizeOutOfRange {
                width: size_km.width,
                height: size_km.height,
                min: MIN_AXIS_KM,
                max: MAX_AXIS_KM,
            });
        }
        if latitude_band.south_deg >= latitude_band.north_deg
            || latitude_band.south_deg < -80
            || latitude_band.north_deg > 80
        {
            return Err(ConfigError::LatitudeOutOfRange {
                south: latitude_band.south_deg,
                north: latitude_band.north_deg,
            });
        }
        if mean_density_per_km2 < MIN_DENSITY || mean_density_per_km2 > MAX_DENSITY {
            return Err(ConfigError::DensityOutOfRange {
                value: mean_density_per_km2,
                min: MIN_DENSITY,
                max: MAX_DENSITY,
            });
        }
        Ok(Self {
            size_km,
            latitude_band,
            mean_density_per_km2,
        })
    }

    /// The continent extent.
    #[must_use]
    pub const fn size_km(self) -> SizeKm {
        self.size_km
    }

    /// The latitude belt.
    #[must_use]
    pub const fn latitude_band(self) -> LatitudeBand {
        self.latitude_band
    }

    /// Mean settlement density in people per square kilometre.
    #[must_use]
    pub const fn mean_density_per_km2(self) -> u16 {
        self.mean_density_per_km2
    }

    /// Area tiles across the continent.
    #[must_use]
    pub const fn areas_wide(self) -> i32 {
        (self.size_km.width / AREA_SIZE_KM) as i32
    }

    /// Area tiles down the continent.
    #[must_use]
    pub const fn areas_high(self) -> i32 {
        (self.size_km.height / AREA_SIZE_KM) as i32
    }

    /// Every tile index, row-major — the orchestrator's work list.
    pub fn area_coords(self) -> impl Iterator<Item = AreaCoord> {
        let wide = self.areas_wide();
        let high = self.areas_high();
        (0..high).flat_map(move |y| (0..wide).map(move |x| AreaCoord::new(x, y)))
    }
}

impl Default for GenerateConfig {
    /// The interviewed default continent: 500x1000 km, 35-55°N, 15 people/km2
    /// (`logic/01` preconditions).
    fn default() -> Self {
        Self {
            size_km: SizeKm::new(500, 1000),
            latitude_band: LatitudeBand::new(35, 55),
            mean_density_per_km2: 15,
        }
    }
}
```

Add to `crates/arda-core/src/lib.rs`:

```rust
pub mod config;
pub mod error;
```

and extend the re-exports:

```rust
pub use config::{GenerateConfig, LatitudeBand, SizeKm};
pub use error::{ConfigError, FormatError, LoadError};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p arda-core config`
Expected: PASS — 6 tests pass.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/arda-core/src
git commit -m "feat: add typed errors and validated generation config"
```

---

### Task 5: Manifest codec — `world.json`

The manifest is written **last** and is the completion stamp: its absence is what makes a partial world unloadable (`04-data-flow.md` failure paths).

**Files:**
- Create: `crates/arda-core/src/formats/mod.rs`
- Create: `crates/arda-core/src/formats/manifest.rs`
- Modify: `crates/arda-core/src/lib.rs`

**Interfaces:**
- Consumes: `GenerateConfig` (Task 4), `LoadError` (Task 4)
- Produces:
  - `const FORMAT_VERSION: u32 = 1`
  - `struct Manifest { format_version: u32, arda_version: String, seed: u64, config: GenerateConfig, areas_wide: i32, areas_high: i32, stats: ValidationStats }`
  - `struct ValidationStats { land_fraction_permille: u16, area_count: u32, settlement_count: u32, named_river_count: u32 }`
  - `fn write_manifest(dir: &Path, manifest: &Manifest) -> Result<(), LoadError>`
  - `fn read_manifest(dir: &Path) -> Result<Manifest, LoadError>`

- [ ] **Step 1: Write the failing test**

Append to `crates/arda-core/src/formats/manifest.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GenerateConfig;

    fn sample() -> Manifest {
        Manifest {
            format_version: FORMAT_VERSION,
            arda_version: "0.1.0".to_owned(),
            seed: 42,
            config: GenerateConfig::MICRO,
            areas_wide: 2,
            areas_high: 4,
            stats: ValidationStats {
                land_fraction_permille: 612,
                area_count: 8,
                settlement_count: 0,
                named_river_count: 0,
            },
        }
    }

    #[test]
    fn manifest_round_trips_through_disk() {
        let dir = tempdir();
        write_manifest(dir.path(), &sample()).unwrap();
        let read = read_manifest(dir.path()).unwrap();
        assert_eq!(read, sample());
    }

    #[test]
    fn manifest_bytes_are_identical_across_writes() {
        // logic/04: export byte-identity. The manifest must not reorder keys.
        let a = tempdir();
        let b = tempdir();
        write_manifest(a.path(), &sample()).unwrap();
        write_manifest(b.path(), &sample()).unwrap();
        assert_eq!(
            std::fs::read(a.path().join("world.json")).unwrap(),
            std::fs::read(b.path().join("world.json")).unwrap()
        );
    }

    #[test]
    fn missing_manifest_names_the_directory() {
        // mockup/02 States: loaders refuse a partial world.
        let dir = tempdir();
        let err = read_manifest(dir.path()).unwrap_err();
        assert!(matches!(err, LoadError::ManifestMissing { .. }));
        assert!(err.to_string().contains("did not finish"));
    }

    #[test]
    fn newer_format_major_is_refused_with_the_regenerate_remedy() {
        // logic/05: version skew carries both versions.
        let dir = tempdir();
        let mut future = sample();
        future.format_version = FORMAT_VERSION + 1;
        write_manifest(dir.path(), &future).unwrap();

        let err = read_manifest(dir.path()).unwrap_err();
        match err {
            LoadError::VersionSkew { found, supported } => {
                assert_eq!(found, FORMAT_VERSION + 1);
                assert_eq!(supported, FORMAT_VERSION);
            }
            other => panic!("expected VersionSkew, got {other:?}"),
        }
        assert!(err.to_string().contains("regenerate"));
    }

    #[test]
    fn unparseable_manifest_is_refused() {
        let dir = tempdir();
        std::fs::write(dir.path().join("world.json"), b"{ not json").unwrap();
        let err = read_manifest(dir.path()).unwrap_err();
        assert!(matches!(err, LoadError::ManifestUnreadable { .. }));
    }

    /// Minimal temp dir helper — `code-prefs.md` §Q6 bans mocks; temp dirs
    /// are the sanctioned substitute, and this avoids a dev-dependency.
    fn tempdir() -> TempDir {
        TempDir::new()
    }

    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new() -> Self {
            use std::sync::atomic::{AtomicU32, Ordering};
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!(
                "arda-test-{}-{}-{n}",
                std::process::id(),
                module_path!().replace("::", "-")
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p arda-core manifest`
Expected: FAIL — `cannot find function 'write_manifest' in this scope`; the `formats` module is not declared.

- [ ] **Step 3: Write minimal implementation**

Create `crates/arda-core/src/formats/mod.rs`:

```rust
//! The only byte codec in the workspace (`01-architecture.md`).
//!
//! Layouts are hand-specified little-endian; each layer documents its own
//! row format. No other crate encodes or decodes world bytes.

pub mod manifest;

/// The world-format major. Bumped only by a breaking layout change; loaders
/// refuse newer majors with the regenerate remedy (`logic/05`).
pub const FORMAT_VERSION: u32 = 1;

/// Writes a little-endian `u16` into `out`.
pub(crate) fn put_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Writes a little-endian `u32` into `out`.
pub(crate) fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Writes a little-endian `i32` into `out`.
pub(crate) fn put_i32(out: &mut Vec<u8>, v: i32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Writes a little-endian `i16` into `out`.
pub(crate) fn put_i16(out: &mut Vec<u8>, v: i16) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Reads a little-endian `u16` at `at`, advancing the cursor.
pub(crate) fn take_u16(src: &[u8], at: &mut usize) -> u16 {
    let v = u16::from_le_bytes([src[*at], src[*at + 1]]);
    *at += 2;
    v
}

/// Reads a little-endian `u32` at `at`, advancing the cursor.
pub(crate) fn take_u32(src: &[u8], at: &mut usize) -> u32 {
    let v = u32::from_le_bytes([src[*at], src[*at + 1], src[*at + 2], src[*at + 3]]);
    *at += 4;
    v
}

/// Reads a little-endian `i32` at `at`, advancing the cursor.
pub(crate) fn take_i32(src: &[u8], at: &mut usize) -> i32 {
    let v = i32::from_le_bytes([src[*at], src[*at + 1], src[*at + 2], src[*at + 3]]);
    *at += 4;
    v
}

/// Reads a little-endian `i16` at `at`, advancing the cursor.
pub(crate) fn take_i16(src: &[u8], at: &mut usize) -> i16 {
    let v = i16::from_le_bytes([src[*at], src[*at + 1]]);
    *at += 2;
    v
}

/// Reads one byte at `at`, advancing the cursor.
pub(crate) fn take_u8(src: &[u8], at: &mut usize) -> u8 {
    let v = src[*at];
    *at += 1;
    v
}
```

Prepend to `crates/arda-core/src/formats/manifest.rs`:

```rust
//! `world.json` — the manifest and completion stamp (`mockup/02`).
//!
//! Written last by the batch: its absence is what makes a partial world
//! unloadable (`04-data-flow.md`).

use super::FORMAT_VERSION;
use crate::config::GenerateConfig;
use crate::error::LoadError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Plausibility statistics printed by `generate` and stored for `load`
/// (`logic/01` step 9, `mockup/01` validation footer).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationStats {
    /// Land cells per thousand continent cells.
    pub land_fraction_permille: u16,
    /// Area tiles written.
    pub area_count: u32,
    /// Settlements placed across the continent.
    pub settlement_count: u32,
    /// Named rivers reaching the sea.
    pub named_river_count: u32,
}

/// Everything needed to identify, verify, or regenerate a world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// World-format major (`logic/05` version gate).
    pub format_version: u32,
    /// The arda build that wrote this world.
    pub arda_version: String,
    /// The sole source of nondeterminism (`mockup/01` `--seed`).
    pub seed: u64,
    /// Config echo — enough to regenerate.
    pub config: GenerateConfig,
    /// Area tiles across.
    pub areas_wide: i32,
    /// Area tiles down.
    pub areas_high: i32,
    /// Validation statistics.
    pub stats: ValidationStats,
}

/// Manifest file name inside a world directory.
pub const MANIFEST_NAME: &str = "world.json";

/// Writes `world.json` into `dir`.
///
/// Uses `serde_json::to_vec_pretty` over a struct with a fixed field order,
/// so repeated writes of equal manifests are byte-identical (`logic/04`).
///
/// # Errors
/// Returns [`LoadError::ManifestUnreadable`] when the directory cannot be
/// written.
pub fn write_manifest(dir: &Path, manifest: &Manifest) -> Result<(), LoadError> {
    let bytes = serde_json::to_vec_pretty(manifest).map_err(|e| {
        LoadError::ManifestUnreadable {
            dir: dir.display().to_string(),
            reason: e.to_string(),
        }
    })?;
    std::fs::write(dir.join(MANIFEST_NAME), bytes).map_err(|e| {
        LoadError::ManifestUnreadable {
            dir: dir.display().to_string(),
            reason: e.to_string(),
        }
    })
}

/// Reads and version-gates `world.json` from `dir`.
///
/// # Errors
/// - [`LoadError::ManifestMissing`] when the file is absent (partial world).
/// - [`LoadError::ManifestUnreadable`] when it will not parse.
/// - [`LoadError::VersionSkew`] when the format major is newer than this build.
pub fn read_manifest(dir: &Path) -> Result<Manifest, LoadError> {
    let path = dir.join(MANIFEST_NAME);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(LoadError::ManifestMissing {
                dir: dir.display().to_string(),
            })
        }
        Err(e) => {
            return Err(LoadError::ManifestUnreadable {
                dir: dir.display().to_string(),
                reason: e.to_string(),
            })
        }
    };

    let manifest: Manifest =
        serde_json::from_slice(&bytes).map_err(|e| LoadError::ManifestUnreadable {
            dir: dir.display().to_string(),
            reason: e.to_string(),
        })?;

    if manifest.format_version > FORMAT_VERSION {
        return Err(LoadError::VersionSkew {
            found: manifest.format_version,
            supported: FORMAT_VERSION,
        });
    }
    Ok(manifest)
}
```

Add to `crates/arda-core/src/lib.rs`:

```rust
pub mod formats;
```

and extend the re-exports:

```rust
pub use formats::manifest::{Manifest, ValidationStats, MANIFEST_NAME};
pub use formats::FORMAT_VERSION;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p arda-core manifest`
Expected: PASS — 5 tests pass (`manifest_round_trips_through_disk`, `manifest_bytes_are_identical_across_writes`, `missing_manifest_names_the_directory`, `newer_format_major_is_refused_with_the_regenerate_remedy`, `unparseable_manifest_is_refused`).

- [ ] **Step 5: Commit**

```bash
git add crates/arda-core/src
git commit -m "feat: add world.json manifest codec with version gate"
```

---

### Task 6: Cell struct and `cells.bin` codec

The row layout is the schema (`02-models.md` Schema). It is defined **in full** here — the artifact's complete per-cell field list — even though the skeleton only fills relief and water. Later build-order steps fill the remaining fields without a format major bump, which is exactly the "additive within a major" evolution rule.

**Files:**
- Create: `crates/arda-core/src/cell.rs`
- Create: `crates/arda-core/src/formats/cells.rs`
- Modify: `crates/arda-core/src/lib.rs`, `crates/arda-core/src/formats/mod.rs`

**Interfaces:**
- Consumes: `HeightMm`, `TempCentiC`, `RainfallMm`, `DischargeMilli` (Task 2); `AREA_CELLS` (Task 2); `FormatError` (Task 4); the `put_*`/`take_*` helpers (Task 5)
- Produces:
  - `enum TerrainKind { Sea = 0, Land = 1, Lake = 2 }`, `enum Cover { Bare = 0, Grass = 1, Scrub = 2, Forest = 3, Marsh = 4, Rock = 5, Ice = 6 }`, `enum RoadClass { None = 0, Track = 1, Road = 2, Highway = 3 }`
  - `struct Cell` with 17 public fields (table below)
  - `const CELL_BYTES: usize = 33`
  - `struct AreaCells { cells: Vec<Cell> }` with `AreaCells::flat(Cell)`, `get(CellCoord)`, `set(CellCoord, Cell)`, `iter()`
  - `fn encode_cells(cells: &AreaCells) -> Vec<u8>`
  - `fn decode_cells(path: &str, bytes: &[u8]) -> Result<AreaCells, FormatError>`

**Row layout — 33 bytes, little-endian, in this exact order:**

| Offset | Bytes | Field | Type |
|---|---|---|---|
| 0 | 4 | `height` | `i32` mm |
| 4 | 1 | `terrain` | `u8` discriminant |
| 5 | 1 | `cover` | `u8` discriminant |
| 6 | 2 | `slope_milli_deg` | `u16` |
| 8 | 2 | `aspect_deg` | `u16` |
| 10 | 2 | `temperature` | `i16` centi-°C |
| 12 | 2 | `rainfall` | `u16` mm/yr |
| 14 | 1 | `moisture` | `u8` 0–255 |
| 15 | 1 | `forest_density` | `u8` 0–255 |
| 16 | 4 | `drainage_area_cells` | `u32` |
| 20 | 4 | `discharge` | `u32` milli-cumecs |
| 24 | 1 | `watercourse_order` | `u8` Strahler, 0 = none |
| 25 | 2 | `watercourse_width_dm` | `u16` |
| 27 | 2 | `height_above_river_dm` | `u16` |
| 29 | 1 | `wetness` | `u8` 0–255 |
| 30 | 1 | `road` | `u8` discriminant |
| 31 | 2 | `built_by` | `u16` settlement id, 0 = none |

- [ ] **Step 1: Write the failing test**

Append to `crates/arda-core/src/formats/cells.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::{Cover, RoadClass, TerrainKind};
    use crate::coords::CellCoord;
    use crate::fixed::{DischargeMilli, HeightMm, RainfallMm, TempCentiC};

    fn distinctive() -> Cell {
        Cell {
            height: HeightMm::new(-1_234_567),
            terrain: TerrainKind::Lake,
            cover: Cover::Marsh,
            slope_milli_deg: 41_234,
            aspect_deg: 359,
            temperature: TempCentiC::new(-3_912),
            rainfall: RainfallMm::new(64_000),
            moisture: 199,
            forest_density: 7,
            drainage_area_cells: 4_000_000_009,
            discharge: DischargeMilli::new(3_141_592),
            watercourse_order: 6,
            watercourse_width_dm: 65_530,
            height_above_river_dm: 12_345,
            wetness: 254,
            road: RoadClass::Highway,
            built_by: 65_533,
        }
    }

    #[test]
    fn row_is_thirty_three_bytes() {
        let mut out = Vec::new();
        put_cell(&mut out, &distinctive());
        assert_eq!(out.len(), CELL_BYTES);
    }

    #[test]
    fn every_field_round_trips() {
        let mut cells = AreaCells::flat(Cell::default());
        let at = CellCoord::new(7, 9).unwrap();
        cells.set(at, distinctive());

        let bytes = encode_cells(&cells);
        let back = decode_cells("cells.bin", &bytes).unwrap();

        assert_eq!(*back.get(at), distinctive());
        assert_eq!(*back.get(CellCoord::new(0, 0).unwrap()), Cell::default());
    }

    #[test]
    fn encoding_is_stable_across_calls() {
        // logic/04: byte-identity. Same cells in, same bytes out, always.
        let cells = AreaCells::flat(distinctive());
        assert_eq!(encode_cells(&cells), encode_cells(&cells));
    }

    #[test]
    fn full_layer_is_the_expected_size() {
        let cells = AreaCells::flat(Cell::default());
        assert_eq!(encode_cells(&cells).len(), 512 * 512 * CELL_BYTES);
    }

    #[test]
    fn truncated_layer_is_refused_naming_the_file() {
        let bytes = encode_cells(&AreaCells::flat(Cell::default()));
        let err = decode_cells("areas/00_00/cells.bin", &bytes[..bytes.len() - 1]).unwrap_err();
        match err {
            FormatError::UnexpectedEof { path, expected, .. } => {
                assert_eq!(path, "areas/00_00/cells.bin");
                assert_eq!(expected, 512 * 512 * CELL_BYTES);
            }
            other => panic!("expected UnexpectedEof, got {other:?}"),
        }
    }

    #[test]
    fn unknown_terrain_discriminant_is_refused() {
        let mut bytes = encode_cells(&AreaCells::flat(Cell::default()));
        bytes[4] = 200;
        let err = decode_cells("cells.bin", &bytes).unwrap_err();
        match err {
            FormatError::UnknownDiscriminant { field, value, .. } => {
                assert_eq!(field, "terrain");
                assert_eq!(value, 200);
            }
            other => panic!("expected UnknownDiscriminant, got {other:?}"),
        }
    }

    /// Property-style sweep: every discriminant combination survives a
    /// round trip (`code-prefs.md` §Q6 welcomes property tests on formats).
    #[test]
    fn all_enum_combinations_round_trip() {
        let terrains = [TerrainKind::Sea, TerrainKind::Land, TerrainKind::Lake];
        let covers = [
            Cover::Bare,
            Cover::Grass,
            Cover::Scrub,
            Cover::Forest,
            Cover::Marsh,
            Cover::Rock,
            Cover::Ice,
        ];
        let roads = [
            RoadClass::None,
            RoadClass::Track,
            RoadClass::Road,
            RoadClass::Highway,
        ];
        for t in terrains {
            for c in covers {
                for r in roads {
                    let cell = Cell {
                        terrain: t,
                        cover: c,
                        road: r,
                        ..Cell::default()
                    };
                    let mut out = Vec::new();
                    put_cell(&mut out, &cell);
                    let mut at = 0;
                    assert_eq!(take_cell("cells.bin", &out, &mut at).unwrap(), cell);
                }
            }
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p arda-core cells`
Expected: FAIL — `cannot find struct 'Cell' in this scope`; neither `cell` nor `formats::cells` is declared.

- [ ] **Step 3: Write minimal implementation**

Create `crates/arda-core/src/cell.rs`:

```rust
//! The per-cell field list (`02-models.md` Fields and types).
//!
//! Every field is the artifact's "what the finished map knows" list; sim
//! values are fixed-point so the layer is bit-identical across platforms.

use crate::fixed::{DischargeMilli, HeightMm, RainfallMm, TempCentiC};

/// Whether a cell is sea, dry land, or lake surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum TerrainKind {
    /// Below sea level and connected to the ocean.
    #[default]
    Sea = 0,
    /// Dry land.
    Land = 1,
    /// Inland standing water.
    Lake = 2,
}

impl TerrainKind {
    /// Decodes a stored discriminant.
    #[must_use]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Sea),
            1 => Some(Self::Land),
            2 => Some(Self::Lake),
            _ => None,
        }
    }
}

/// Dominant ground cover (`logic/02` vegetation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Cover {
    /// Bare soil or sand.
    #[default]
    Bare = 0,
    /// Grassland.
    Grass = 1,
    /// Scrub and heath.
    Scrub = 2,
    /// Closed forest.
    Forest = 3,
    /// Marsh or fen.
    Marsh = 4,
    /// Exposed rock.
    Rock = 5,
    /// Permanent ice.
    Ice = 6,
}

impl Cover {
    /// Decodes a stored discriminant.
    #[must_use]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Bare),
            1 => Some(Self::Grass),
            2 => Some(Self::Scrub),
            3 => Some(Self::Forest),
            4 => Some(Self::Marsh),
            5 => Some(Self::Rock),
            6 => Some(Self::Ice),
            _ => None,
        }
    }
}

/// Road class crossing a cell (`logic/02` roads).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum RoadClass {
    /// No road.
    #[default]
    None = 0,
    /// Footpath or cart track.
    Track = 1,
    /// Maintained road.
    Road = 2,
    /// Trunk corridor.
    Highway = 3,
}

impl RoadClass {
    /// Decodes a stored discriminant.
    #[must_use]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::None),
            1 => Some(Self::Track),
            2 => Some(Self::Road),
            3 => Some(Self::Highway),
            _ => None,
        }
    }
}

/// Everything the finished map knows about one 100 m cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cell {
    /// Elevation.
    pub height: HeightMm,
    /// Sea, land, or lake.
    pub terrain: TerrainKind,
    /// Dominant ground cover.
    pub cover: Cover,
    /// Slope in thousandths of a degree.
    pub slope_milli_deg: u16,
    /// Downslope compass bearing, 0-359; 0 on flat ground.
    pub aspect_deg: u16,
    /// Mean annual temperature.
    pub temperature: TempCentiC,
    /// Mean annual rainfall.
    pub rainfall: RainfallMm,
    /// Soil moisture, 0-255.
    pub moisture: u8,
    /// Canopy closure, 0-255.
    pub forest_density: u8,
    /// Upstream cells draining through here.
    pub drainage_area_cells: u32,
    /// Mean discharge.
    pub discharge: DischargeMilli,
    /// Strahler order; 0 when no watercourse.
    pub watercourse_order: u8,
    /// Channel width in decimetres; 0 when no watercourse.
    pub watercourse_width_dm: u16,
    /// Height above the nearest downstream channel, decimetres.
    pub height_above_river_dm: u16,
    /// Standing-water tendency, 0-255.
    pub wetness: u8,
    /// Road class.
    pub road: RoadClass,
    /// Settlement that built on this cell; 0 when none.
    pub built_by: u16,
}
```

Create `crates/arda-core/src/formats/cells.rs`:

```rust
//! `areas/<ax>_<ay>/cells.bin` — 512x512 fixed-layout rows.
//!
//! Row layout is documented in the plan and in `02-models.md`; fields are
//! written one at a time in little-endian order. No `unsafe` transmute
//! (`code-prefs.md` §Q1), so the layout is host-independent by construction.

use super::{put_i16, put_i32, put_u16, put_u32, take_i16, take_i32, take_u16, take_u32, take_u8};
use crate::cell::{Cell, Cover, RoadClass, TerrainKind};
use crate::coords::{CellCoord, AREA_CELLS};
use crate::error::FormatError;
use crate::fixed::{DischargeMilli, HeightMm, RainfallMm, TempCentiC};

/// Bytes per stored cell row.
pub const CELL_BYTES: usize = 33;

/// Cells in one area layer.
const CELL_COUNT: usize = AREA_CELLS as usize * AREA_CELLS as usize;

/// One area tile's full cell grid, row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaCells {
    cells: Vec<Cell>,
}

impl AreaCells {
    /// Builds a grid with every cell set to `fill`.
    #[must_use]
    pub fn flat(fill: Cell) -> Self {
        Self {
            cells: vec![fill; CELL_COUNT],
        }
    }

    /// Reads one cell.
    #[must_use]
    pub fn get(&self, at: CellCoord) -> &Cell {
        // Invariant: CellCoord is bounds-checked at construction, and the
        // vector is always CELL_COUNT long, so this index cannot be out of
        // range (`code-prefs.md` §Q4 invariant-backed indexing).
        &self.cells[at.index()]
    }

    /// Overwrites one cell.
    pub fn set(&mut self, at: CellCoord, cell: Cell) {
        self.cells[at.index()] = cell;
    }

    /// Iterates cells in row-major order.
    pub fn iter(&self) -> impl Iterator<Item = &Cell> {
        self.cells.iter()
    }
}

/// Appends one cell row to `out`.
pub(crate) fn put_cell(out: &mut Vec<u8>, c: &Cell) {
    put_i32(out, c.height.raw());
    out.push(c.terrain as u8);
    out.push(c.cover as u8);
    put_u16(out, c.slope_milli_deg);
    put_u16(out, c.aspect_deg);
    put_i16(out, c.temperature.raw());
    put_u16(out, c.rainfall.raw());
    out.push(c.moisture);
    out.push(c.forest_density);
    put_u32(out, c.drainage_area_cells);
    put_u32(out, c.discharge.raw());
    out.push(c.watercourse_order);
    put_u16(out, c.watercourse_width_dm);
    put_u16(out, c.height_above_river_dm);
    out.push(c.wetness);
    out.push(c.road as u8);
    put_u16(out, c.built_by);
}

/// Reads one cell row at `at`, advancing the cursor.
///
/// # Errors
/// Returns [`FormatError::UnknownDiscriminant`] for a stored enum byte this
/// build does not know.
pub(crate) fn take_cell(path: &str, src: &[u8], at: &mut usize) -> Result<Cell, FormatError> {
    let height = HeightMm::new(take_i32(src, at));
    let terrain_raw = take_u8(src, at);
    let terrain = TerrainKind::from_u8(terrain_raw).ok_or(FormatError::UnknownDiscriminant {
        path: path.to_owned(),
        field: "terrain",
        value: u16::from(terrain_raw),
    })?;
    let cover_raw = take_u8(src, at);
    let cover = Cover::from_u8(cover_raw).ok_or(FormatError::UnknownDiscriminant {
        path: path.to_owned(),
        field: "cover",
        value: u16::from(cover_raw),
    })?;
    let slope_milli_deg = take_u16(src, at);
    let aspect_deg = take_u16(src, at);
    let temperature = TempCentiC::new(take_i16(src, at));
    let rainfall = RainfallMm::new(take_u16(src, at));
    let moisture = take_u8(src, at);
    let forest_density = take_u8(src, at);
    let drainage_area_cells = take_u32(src, at);
    let discharge = DischargeMilli::new(take_u32(src, at));
    let watercourse_order = take_u8(src, at);
    let watercourse_width_dm = take_u16(src, at);
    let height_above_river_dm = take_u16(src, at);
    let wetness = take_u8(src, at);
    let road_raw = take_u8(src, at);
    let road = RoadClass::from_u8(road_raw).ok_or(FormatError::UnknownDiscriminant {
        path: path.to_owned(),
        field: "road",
        value: u16::from(road_raw),
    })?;
    let built_by = take_u16(src, at);

    Ok(Cell {
        height,
        terrain,
        cover,
        slope_milli_deg,
        aspect_deg,
        temperature,
        rainfall,
        moisture,
        forest_density,
        drainage_area_cells,
        discharge,
        watercourse_order,
        watercourse_width_dm,
        height_above_river_dm,
        wetness,
        road,
        built_by,
    })
}

/// Encodes a full area cell layer.
#[must_use]
pub fn encode_cells(cells: &AreaCells) -> Vec<u8> {
    let mut out = Vec::with_capacity(CELL_COUNT * CELL_BYTES);
    for cell in cells.iter() {
        put_cell(&mut out, cell);
    }
    out
}

/// Decodes a full area cell layer.
///
/// # Errors
/// - [`FormatError::UnexpectedEof`] when the layer is not exactly the
///   expected length.
/// - [`FormatError::UnknownDiscriminant`] for an unrecognised enum byte.
pub fn decode_cells(path: &str, bytes: &[u8]) -> Result<AreaCells, FormatError> {
    let expected = CELL_COUNT * CELL_BYTES;
    if bytes.len() != expected {
        return Err(FormatError::UnexpectedEof {
            path: path.to_owned(),
            read: bytes.len(),
            expected,
        });
    }
    let mut cells = Vec::with_capacity(CELL_COUNT);
    let mut at = 0usize;
    for _ in 0..CELL_COUNT {
        cells.push(take_cell(path, bytes, &mut at)?);
    }
    Ok(AreaCells { cells })
}
```

Add `pub mod cells;` to `crates/arda-core/src/formats/mod.rs`, and to `crates/arda-core/src/lib.rs`:

```rust
pub mod cell;
```

with re-exports:

```rust
pub use cell::{Cell, Cover, RoadClass, TerrainKind};
pub use formats::cells::{decode_cells, encode_cells, AreaCells, CELL_BYTES};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p arda-core cells`
Expected: PASS — 7 tests pass (`row_is_thirty_three_bytes`, `every_field_round_trips`, `encoding_is_stable_across_calls`, `full_layer_is_the_expected_size`, `truncated_layer_is_refused_naming_the_file`, `unknown_terrain_discriminant_is_refused`, `all_enum_combinations_round_trip`).

- [ ] **Step 5: Commit**

```bash
git add crates/arda-core/src
git commit -m "feat: add cell struct and fixed-layout cells.bin codec"
```

---

### Task 7: Objects and `objects.bin` codec

A tagged, length-prefixed section container. The skeleton emits only river segments and lakes (`implementation.md` step 3: "relief+water only area"); settlements, roads, crossings, and passes arrive at build-order step 5 as **new sections**, which is additive within the format major (`02-models.md` Schema).

**Files:**
- Create: `crates/arda-core/src/objects.rs`
- Create: `crates/arda-core/src/formats/objects.rs`
- Modify: `crates/arda-core/src/lib.rs`, `crates/arda-core/src/formats/mod.rs`

**Interfaces:**
- Consumes: `CellCoord` (Task 2), `HeightMm`/`DischargeMilli` (Task 2), `FormatError` (Task 4), byte helpers (Task 5)
- Produces:
  - `struct RiverSegment { id: u16, order: u8, width_dm: u16, discharge: DischargeMilli, course: Vec<CellCoord> }`
  - `struct Lake { id: u16, surface: HeightMm, cells: Vec<CellCoord> }`
  - `struct AreaObjects { rivers: Vec<RiverSegment>, lakes: Vec<Lake> }` with `AreaObjects::empty()`
  - `fn encode_objects(objects: &AreaObjects) -> Vec<u8>`
  - `fn decode_objects(path: &str, bytes: &[u8]) -> Result<AreaObjects, FormatError>`
  - `const OBJECTS_MAGIC: &[u8; 8] = b"ARDAOBJ\0"`

**Container layout, little-endian:**

```text
magic        8 bytes  "ARDAOBJ\0"
section_ct   u16
per section:
  kind       u16      1 = rivers, 2 = lakes
  record_ct  u32
  byte_len   u32      length of the records block that follows
  records    byte_len bytes
```

Unknown section kinds are **skipped**, not refused — that is what makes step 5's additions backward-compatible.

- [ ] **Step 1: Write the failing test**

Append to `crates/arda-core/src/formats/objects.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::coords::CellCoord;
    use crate::fixed::{DischargeMilli, HeightMm};

    fn cc(x: u16, y: u16) -> CellCoord {
        CellCoord::new(x, y).unwrap()
    }

    fn sample() -> AreaObjects {
        AreaObjects {
            rivers: vec![
                RiverSegment {
                    id: 1,
                    order: 3,
                    width_dm: 240,
                    discharge: DischargeMilli::new(88_000),
                    course: vec![cc(0, 0), cc(1, 0), cc(1, 1), cc(511, 511)],
                },
                RiverSegment {
                    id: 2,
                    order: 1,
                    width_dm: 15,
                    discharge: DischargeMilli::new(400),
                    course: vec![cc(100, 200)],
                },
            ],
            lakes: vec![Lake {
                id: 1,
                surface: HeightMm::new(214_000),
                cells: vec![cc(50, 50), cc(51, 50), cc(50, 51)],
            }],
        }
    }

    #[test]
    fn objects_round_trip() {
        let bytes = encode_objects(&sample());
        assert_eq!(decode_objects("objects.bin", &bytes).unwrap(), sample());
    }

    #[test]
    fn empty_objects_round_trip() {
        let bytes = encode_objects(&AreaObjects::empty());
        assert_eq!(
            decode_objects("objects.bin", &bytes).unwrap(),
            AreaObjects::empty()
        );
    }

    #[test]
    fn encoding_is_stable_across_calls() {
        assert_eq!(encode_objects(&sample()), encode_objects(&sample()));
    }

    #[test]
    fn bad_magic_is_refused_naming_the_layer() {
        let mut bytes = encode_objects(&sample());
        bytes[0] = b'X';
        let err = decode_objects("areas/00_00/objects.bin", &bytes).unwrap_err();
        match err {
            FormatError::BadMagic { path, layer } => {
                assert_eq!(path, "areas/00_00/objects.bin");
                assert_eq!(layer, "objects");
            }
            other => panic!("expected BadMagic, got {other:?}"),
        }
    }

    #[test]
    fn truncated_container_is_refused() {
        let bytes = encode_objects(&sample());
        let err = decode_objects("objects.bin", &bytes[..bytes.len() - 3]).unwrap_err();
        assert!(matches!(err, FormatError::UnexpectedEof { .. }));
    }

    /// Forward compatibility: a section this build does not know is skipped,
    /// so build-order step 5 can add settlements without a format major bump.
    #[test]
    fn unknown_section_kind_is_skipped() {
        let mut bytes = encode_objects(&sample());
        // Bump the section count and append a bogus kind-999 section.
        let count = u16::from_le_bytes([bytes[8], bytes[9]]);
        bytes[8..10].copy_from_slice(&(count + 1).to_le_bytes());
        bytes.extend_from_slice(&999u16.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);

        assert_eq!(decode_objects("objects.bin", &bytes).unwrap(), sample());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p arda-core formats::objects`
Expected: FAIL — `cannot find struct 'AreaObjects' in this scope`.

- [ ] **Step 3: Write minimal implementation**

Create `crates/arda-core/src/objects.rs`:

```rust
//! Area object lists (`02-models.md` Entities).
//!
//! The skeleton carries rivers and lakes; settlements, roads, crossings, and
//! passes join at build-order step 5 as additional sections.

use crate::coords::CellCoord;
use crate::fixed::{DischargeMilli, HeightMm};

/// One watercourse reach inside an area tile (`logic/02` water).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiverSegment {
    /// Tile-local identifier, 1-based.
    pub id: u16,
    /// Strahler order.
    pub order: u8,
    /// Channel width in decimetres.
    pub width_dm: u16,
    /// Mean discharge.
    pub discharge: DischargeMilli,
    /// Cells the channel runs through, upstream to downstream.
    pub course: Vec<CellCoord>,
}

/// A body of inland standing water.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lake {
    /// Tile-local identifier, 1-based.
    pub id: u16,
    /// Water-surface elevation.
    pub surface: HeightMm,
    /// Cells covered by the lake.
    pub cells: Vec<CellCoord>,
}

/// Every object stored alongside one area's cells.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AreaObjects {
    /// Watercourse reaches.
    pub rivers: Vec<RiverSegment>,
    /// Lakes.
    pub lakes: Vec<Lake>,
}

impl AreaObjects {
    /// An area with no objects.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
}
```

Create `crates/arda-core/src/formats/objects.rs`:

```rust
//! `areas/<ax>_<ay>/objects.bin` — a tagged section container.
//!
//! Unknown section kinds are skipped so later stages can add record types
//! without a format major bump (`02-models.md` Schema: additive within a
//! major).

use super::{put_u16, put_u32, take_u16, take_u32, take_u8};
use crate::coords::CellCoord;
use crate::error::FormatError;
use crate::fixed::{DischargeMilli, HeightMm};
use crate::objects::{AreaObjects, Lake, RiverSegment};

/// Container magic.
pub const OBJECTS_MAGIC: &[u8; 8] = b"ARDAOBJ\0";

const KIND_RIVERS: u16 = 1;
const KIND_LAKES: u16 = 2;

fn eof(path: &str, read: usize, expected: usize) -> FormatError {
    FormatError::UnexpectedEof {
        path: path.to_owned(),
        read,
        expected,
    }
}

fn need(path: &str, src: &[u8], at: usize, extra: usize) -> Result<(), FormatError> {
    if at + extra > src.len() {
        Err(eof(path, src.len(), at + extra))
    } else {
        Ok(())
    }
}

fn put_course(out: &mut Vec<u8>, course: &[CellCoord]) {
    put_u32(out, u32::try_from(course.len()).unwrap_or(u32::MAX));
    for c in course {
        put_u16(out, c.x());
        put_u16(out, c.y());
    }
}

fn take_course(path: &str, src: &[u8], at: &mut usize) -> Result<Vec<CellCoord>, FormatError> {
    need(path, src, *at, 4)?;
    let n = take_u32(src, at) as usize;
    need(path, src, *at, n * 4)?;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let x = take_u16(src, at);
        let y = take_u16(src, at);
        let coord = CellCoord::new(x, y).ok_or(FormatError::UnknownDiscriminant {
            path: path.to_owned(),
            field: "cell coordinate",
            value: x.max(y),
        })?;
        out.push(coord);
    }
    Ok(out)
}

/// Encodes an area's object lists.
#[must_use]
pub fn encode_objects(objects: &AreaObjects) -> Vec<u8> {
    let mut rivers = Vec::new();
    for r in &objects.rivers {
        put_u16(&mut rivers, r.id);
        rivers.push(r.order);
        put_u16(&mut rivers, r.width_dm);
        put_u32(&mut rivers, r.discharge.raw());
        put_course(&mut rivers, &r.course);
    }

    let mut lakes = Vec::new();
    for l in &objects.lakes {
        put_u16(&mut lakes, l.id);
        lakes.extend_from_slice(&l.surface.raw().to_le_bytes());
        put_course(&mut lakes, &l.cells);
    }

    let mut out = Vec::with_capacity(OBJECTS_MAGIC.len() + 2 + rivers.len() + lakes.len() + 20);
    out.extend_from_slice(OBJECTS_MAGIC);
    put_u16(&mut out, 2);

    put_u16(&mut out, KIND_RIVERS);
    put_u32(&mut out, u32::try_from(objects.rivers.len()).unwrap_or(u32::MAX));
    put_u32(&mut out, u32::try_from(rivers.len()).unwrap_or(u32::MAX));
    out.extend_from_slice(&rivers);

    put_u16(&mut out, KIND_LAKES);
    put_u32(&mut out, u32::try_from(objects.lakes.len()).unwrap_or(u32::MAX));
    put_u32(&mut out, u32::try_from(lakes.len()).unwrap_or(u32::MAX));
    out.extend_from_slice(&lakes);

    out
}

/// Decodes an area's object lists.
///
/// # Errors
/// - [`FormatError::BadMagic`] when the file is not an objects layer.
/// - [`FormatError::UnexpectedEof`] when a section runs past the end.
pub fn decode_objects(path: &str, bytes: &[u8]) -> Result<AreaObjects, FormatError> {
    if bytes.len() < OBJECTS_MAGIC.len() || &bytes[..OBJECTS_MAGIC.len()] != OBJECTS_MAGIC {
        return Err(FormatError::BadMagic {
            path: path.to_owned(),
            layer: "objects",
        });
    }

    let mut at = OBJECTS_MAGIC.len();
    need(path, bytes, at, 2)?;
    let sections = take_u16(bytes, &mut at);

    let mut out = AreaObjects::empty();
    for _ in 0..sections {
        need(path, bytes, at, 10)?;
        let kind = take_u16(bytes, &mut at);
        let record_ct = take_u32(bytes, &mut at) as usize;
        let byte_len = take_u32(bytes, &mut at) as usize;
        need(path, bytes, at, byte_len)?;
        let end = at + byte_len;

        match kind {
            KIND_RIVERS => {
                for _ in 0..record_ct {
                    need(path, bytes, at, 9)?;
                    let id = take_u16(bytes, &mut at);
                    let order = take_u8(bytes, &mut at);
                    let width_dm = take_u16(bytes, &mut at);
                    let discharge = DischargeMilli::new(take_u32(bytes, &mut at));
                    let course = take_course(path, bytes, &mut at)?;
                    out.rivers.push(RiverSegment {
                        id,
                        order,
                        width_dm,
                        discharge,
                        course,
                    });
                }
            }
            KIND_LAKES => {
                for _ in 0..record_ct {
                    need(path, bytes, at, 6)?;
                    let id = take_u16(bytes, &mut at);
                    let surface = HeightMm::new(super::take_i32(bytes, &mut at));
                    let cells = take_course(path, bytes, &mut at)?;
                    out.lakes.push(Lake { id, surface, cells });
                }
            }
            // Forward compatibility: a section a later build wrote and this
            // one does not know (`02-models.md` additive evolution).
            _ => {}
        }
        at = end;
    }
    Ok(out)
}
```

Add `pub mod objects;` to `crates/arda-core/src/formats/mod.rs`, and to `crates/arda-core/src/lib.rs`:

```rust
pub mod objects;
```

with re-exports:

```rust
pub use formats::objects::{decode_objects, encode_objects, OBJECTS_MAGIC};
pub use objects::{AreaObjects, Lake, RiverSegment};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p arda-core formats::objects`
Expected: PASS — 6 tests pass (`objects_round_trip`, `empty_objects_round_trip`, `encoding_is_stable_across_calls`, `bad_magic_is_refused_naming_the_layer`, `truncated_container_is_refused`, `unknown_section_kind_is_skipped`).

- [ ] **Step 5: Commit**

```bash
git add crates/arda-core/src
git commit -m "feat: add tagged objects.bin container with river and lake sections"
```

---

### Task 8: Tile vocabulary and the zstd block archive

**Files:**
- Create: `crates/arda-core/src/tiles.rs`
- Create: `crates/arda-core/src/formats/blocks.rs`
- Modify: `crates/arda-core/src/lib.rs`, `crates/arda-core/src/formats/mod.rs`

**Interfaces:**
- Consumes: `CellCoord`, `SquareCoord`, `BLOCK_SQUARES` (Task 2); `FormatError` (Task 4); byte helpers (Task 5)
- Produces:
  - `struct TileId(u16)` with `TileId::raw()`, `TileId::new()`
  - `const SKELETON_TILES: [TileDef; 24]` and `struct TileDef { id: TileId, name: &'static str, group: TileGroup }`
  - `enum TileGroup { Water, Shore, Ground, Vegetation, Rock, Structure }`
  - `fn tile_def(id: TileId) -> Option<&'static TileDef>`, `fn tiles_in_group(group: TileGroup) -> impl Iterator<Item = &'static TileDef>`
  - `fn may_adjoin(a: TileId, b: TileId) -> bool` — the adjacency rule the WFC consumes (`logic/03` §Q12)
  - `struct Block { tiles: Vec<TileId>, relaxed: bool }` with `Block::filled(TileId)`, `square(SquareCoord)`, `set(SquareCoord, TileId)`
  - `struct BlockArchive { blocks: BTreeMap<(u16, u16), Block> }` with `insert(CellCoord, Block)`, `get(CellCoord)`, `len()`
  - `fn encode_blocks(archive: &BlockArchive) -> Result<Vec<u8>, FormatError>`
  - `fn decode_blocks(path: &str, bytes: &[u8]) -> Result<BlockArchive, FormatError>`
  - `const ZSTD_LEVEL: i32 = 3`

**Archive layout** — raw bytes below, then one zstd frame over the whole thing:

```text
magic        8 bytes  "ARDABLK\0"
block_ct     u32
per block (ascending by (cy, cx) — BTreeMap order, so encoding is
           independent of insertion order and therefore of thread order):
  cell_x     u16
  cell_y     u16
  relaxed    u8       1 when the WFC fell back (logic/03 §Q12)
  tiles      4096 x u16
```

- [ ] **Step 1: Write the failing test**

Append to `crates/arda-core/src/formats/blocks.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::coords::{CellCoord, SquareCoord};
    use crate::tiles::{TileGroup, TileId, SKELETON_TILES};

    fn cc(x: u16, y: u16) -> CellCoord {
        CellCoord::new(x, y).unwrap()
    }

    fn sq(x: u8, y: u8) -> SquareCoord {
        SquareCoord::new(x, y).unwrap()
    }

    fn sample() -> BlockArchive {
        let mut archive = BlockArchive::default();

        let mut a = Block::filled(TileId::new(1));
        a.set(sq(0, 0), TileId::new(9));
        a.set(sq(63, 63), TileId::new(23));
        archive.insert(cc(4, 7), a);

        let mut b = Block::filled(TileId::new(5));
        b.mark_relaxed();
        archive.insert(cc(1, 2), b);

        archive
    }

    #[test]
    fn archive_round_trips_through_zstd() {
        let bytes = encode_blocks(&sample()).unwrap();
        let back = decode_blocks("blocks/00_00.tiles.zst", &bytes).unwrap();
        assert_eq!(back, sample());
    }

    #[test]
    fn relaxed_mark_survives_the_round_trip() {
        let bytes = encode_blocks(&sample()).unwrap();
        let back = decode_blocks("blocks/00_00.tiles.zst", &bytes).unwrap();
        assert!(back.get(cc(1, 2)).unwrap().is_relaxed());
        assert!(!back.get(cc(4, 7)).unwrap().is_relaxed());
    }

    #[test]
    fn encoding_is_independent_of_insertion_order() {
        // architecture-interview.md §Q4: thread count must not change bytes.
        let mut forward = BlockArchive::default();
        forward.insert(cc(1, 2), Block::filled(TileId::new(5)));
        forward.insert(cc(4, 7), Block::filled(TileId::new(1)));

        let mut reverse = BlockArchive::default();
        reverse.insert(cc(4, 7), Block::filled(TileId::new(1)));
        reverse.insert(cc(1, 2), Block::filled(TileId::new(5)));

        assert_eq!(
            encode_blocks(&forward).unwrap(),
            encode_blocks(&reverse).unwrap()
        );
    }

    #[test]
    fn empty_archive_round_trips() {
        let bytes = encode_blocks(&BlockArchive::default()).unwrap();
        let back = decode_blocks("blocks/00_00.tiles.zst", &bytes).unwrap();
        assert_eq!(back.len(), 0);
    }

    #[test]
    fn corrupt_frame_is_refused_naming_the_file() {
        let mut bytes = encode_blocks(&sample()).unwrap();
        let n = bytes.len();
        bytes[n / 2] ^= 0xFF;
        bytes[n / 2 + 1] ^= 0xFF;
        let err = decode_blocks("blocks/03_11.tiles.zst", &bytes).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("blocks/03_11.tiles.zst"), "message was: {msg}");
    }

    #[test]
    fn vocabulary_has_twenty_four_unique_tiles() {
        // implementation.md step 3: "~24-tile WFC subset".
        assert_eq!(SKELETON_TILES.len(), 24);
        let mut ids: Vec<u16> = SKELETON_TILES.iter().map(|t| t.id.raw()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 24, "tile ids must be unique");
        assert_eq!(ids[0], 0, "id 0 is reserved for deep water");
    }

    #[test]
    fn adjacency_is_symmetric() {
        // logic/03 §Q12: adjacency is a relation, not a direction.
        for a in SKELETON_TILES {
            for b in SKELETON_TILES {
                assert_eq!(
                    crate::tiles::may_adjoin(a.id, b.id),
                    crate::tiles::may_adjoin(b.id, a.id),
                    "{} vs {}",
                    a.name,
                    b.name
                );
            }
        }
    }

    #[test]
    fn deep_water_never_touches_dry_ground() {
        let deep = SKELETON_TILES
            .iter()
            .find(|t| t.group == TileGroup::Water && t.name == "deep_water")
            .unwrap();
        let grass = SKELETON_TILES.iter().find(|t| t.name == "grass").unwrap();
        assert!(!crate::tiles::may_adjoin(deep.id, grass.id));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p arda-core blocks`
Expected: FAIL — `cannot find struct 'BlockArchive' in this scope`.

- [ ] **Step 3: Write minimal implementation**

Create `crates/arda-core/src/tiles.rs`:

```rust
//! The tile vocabulary and its adjacency relation (`logic/03` §Q12).
//!
//! The skeleton ships 24 tiles; build-order step 6 grows this to the full
//! 200+ vocabulary. Tile ids are stable — a released id is never reused for
//! a different tile.

/// A tile identifier, two bytes on disk (`02-models.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TileId(u16);

impl TileId {
    /// Wraps a raw tile id.
    #[must_use]
    pub const fn new(id: u16) -> Self {
        Self(id)
    }

    /// The raw tile id.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }
}

/// Broad tile family; adjacency is decided between groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileGroup {
    /// Open water.
    Water,
    /// The wet/dry transition.
    Shore,
    /// Walkable ground.
    Ground,
    /// Plant cover standing on ground.
    Vegetation,
    /// Stone and cliff.
    Rock,
    /// Built surfaces.
    Structure,
}

/// One entry in the vocabulary.
#[derive(Debug, Clone, Copy)]
pub struct TileDef {
    /// Stable identifier.
    pub id: TileId,
    /// Stable snake_case name, used by the export legend (`logic/04`).
    pub name: &'static str,
    /// Adjacency family.
    pub group: TileGroup,
}

const fn def(id: u16, name: &'static str, group: TileGroup) -> TileDef {
    TileDef {
        id: TileId::new(id),
        name,
        group,
    }
}

/// The 24-tile skeleton vocabulary (`implementation.md` step 3).
pub const SKELETON_TILES: [TileDef; 24] = [
    def(0, "deep_water", TileGroup::Water),
    def(1, "open_water", TileGroup::Water),
    def(2, "shallow_water", TileGroup::Water),
    def(3, "reed_bed", TileGroup::Shore),
    def(4, "mudflat", TileGroup::Shore),
    def(5, "sand", TileGroup::Shore),
    def(6, "shingle", TileGroup::Shore),
    def(7, "wet_grass", TileGroup::Shore),
    def(8, "dirt", TileGroup::Ground),
    def(9, "grass", TileGroup::Ground),
    def(10, "tall_grass", TileGroup::Ground),
    def(11, "heath", TileGroup::Ground),
    def(12, "moss", TileGroup::Ground),
    def(13, "gravel", TileGroup::Ground),
    def(14, "fern", TileGroup::Vegetation),
    def(15, "bramble", TileGroup::Vegetation),
    def(16, "sapling", TileGroup::Vegetation),
    def(17, "tree_trunk", TileGroup::Vegetation),
    def(18, "deadfall", TileGroup::Vegetation),
    def(19, "boulder", TileGroup::Rock),
    def(20, "scree", TileGroup::Rock),
    def(21, "bedrock", TileGroup::Rock),
    def(22, "cliff_face", TileGroup::Rock),
    def(23, "packed_earth", TileGroup::Structure),
];

/// Looks up a tile by id.
#[must_use]
pub fn tile_def(id: TileId) -> Option<&'static TileDef> {
    SKELETON_TILES.iter().find(|t| t.id == id)
}

/// Every tile in one family.
pub fn tiles_in_group(group: TileGroup) -> impl Iterator<Item = &'static TileDef> {
    SKELETON_TILES.iter().filter(move |t| t.group == group)
}

/// Rank used to decide adjacency: families that sit next to each other in
/// the wet-to-dry ordering may touch, families two steps apart may not.
const fn wetness_rank(group: TileGroup) -> i8 {
    match group {
        TileGroup::Water => 0,
        TileGroup::Shore => 1,
        TileGroup::Ground => 2,
        TileGroup::Vegetation => 3,
        TileGroup::Structure => 3,
        TileGroup::Rock => 4,
    }
}

/// Whether two tiles may share an edge (`logic/03` §Q12).
///
/// Symmetric by construction: it compares family ranks, and deep water is
/// additionally pinned to water-only neighbours so an ocean square can never
/// abut dry ground.
#[must_use]
pub fn may_adjoin(a: TileId, b: TileId) -> bool {
    let (Some(da), Some(db)) = (tile_def(a), tile_def(b)) else {
        return false;
    };
    if da.name == "deep_water" || db.name == "deep_water" {
        return da.group == TileGroup::Water && db.group == TileGroup::Water;
    }
    (wetness_rank(da.group) - wetness_rank(db.group)).abs() <= 1
}
```

Create `crates/arda-core/src/formats/blocks.rs`:

```rust
//! `blocks/<ax>_<ay>.tiles.zst` — one zstd frame per area (`mockup/02`).
//!
//! Blocks are stored in ascending `(cell_y, cell_x)` order so the bytes do
//! not depend on the order areas finished — `architecture-interview.md` §Q4
//! forbids thread count from changing a world.

use super::{put_u16, put_u32, take_u16, take_u32, take_u8};
use crate::coords::{CellCoord, SquareCoord, BLOCK_SQUARES};
use crate::error::FormatError;
use crate::tiles::TileId;
use std::collections::BTreeMap;

/// Archive magic.
pub const BLOCKS_MAGIC: &[u8; 8] = b"ARDABLK\0";

/// Fixed compression level — a varying level would change the bytes.
pub const ZSTD_LEVEL: i32 = 3;

/// Squares in one block.
const SQUARE_COUNT: usize = BLOCK_SQUARES as usize * BLOCK_SQUARES as usize;

/// One 64x64 tactical block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    tiles: Vec<TileId>,
    relaxed: bool,
}

impl Block {
    /// Builds a block with every square set to `fill`.
    #[must_use]
    pub fn filled(fill: TileId) -> Self {
        Self {
            tiles: vec![fill; SQUARE_COUNT],
            relaxed: false,
        }
    }

    /// Reads one square.
    #[must_use]
    pub fn square(&self, at: SquareCoord) -> TileId {
        // Invariant: SquareCoord is bounds-checked and tiles is always
        // SQUARE_COUNT long.
        self.tiles[at.index()]
    }

    /// Overwrites one square.
    pub fn set(&mut self, at: SquareCoord, tile: TileId) {
        self.tiles[at.index()] = tile;
    }

    /// Marks this block as a relaxed fallback fill (`logic/03` §Q12).
    pub fn mark_relaxed(&mut self) {
        self.relaxed = true;
    }

    /// Whether the WFC fell back to a relaxed fill here.
    #[must_use]
    pub const fn is_relaxed(&self) -> bool {
        self.relaxed
    }
}

/// Every block generated for one area tile.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BlockArchive {
    blocks: BTreeMap<(u16, u16), Block>,
}

impl BlockArchive {
    /// Stores a block for one cell.
    pub fn insert(&mut self, cell: CellCoord, block: Block) {
        self.blocks.insert((cell.y(), cell.x()), block);
    }

    /// Reads the block for one cell, when generated.
    #[must_use]
    pub fn get(&self, cell: CellCoord) -> Option<&Block> {
        self.blocks.get(&(cell.y(), cell.x()))
    }

    /// Number of stored blocks.
    #[must_use]
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    /// Whether the archive holds no blocks.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}

/// Encodes and compresses an area's block archive.
///
/// # Errors
/// Returns [`FormatError::Io`] when zstd fails to compress.
pub fn encode_blocks(archive: &BlockArchive) -> Result<Vec<u8>, FormatError> {
    let mut raw = Vec::with_capacity(BLOCKS_MAGIC.len() + 4 + archive.len() * (5 + SQUARE_COUNT * 2));
    raw.extend_from_slice(BLOCKS_MAGIC);
    put_u32(&mut raw, u32::try_from(archive.len()).unwrap_or(u32::MAX));

    // BTreeMap iterates in ascending key order — the determinism guarantee.
    for (&(cy, cx), block) in &archive.blocks {
        put_u16(&mut raw, cx);
        put_u16(&mut raw, cy);
        raw.push(u8::from(block.relaxed));
        for tile in &block.tiles {
            put_u16(&mut raw, tile.raw());
        }
    }

    zstd::encode_all(raw.as_slice(), ZSTD_LEVEL).map_err(|e| FormatError::Io {
        path: "block archive".to_owned(),
        source: e,
    })
}

/// Decompresses and decodes an area's block archive.
///
/// `ponytail:` whole-archive decompression; `logic/05` step 3 wants per-cell
/// lazy decompression. Upgrade path when an area's archive stops fitting in
/// memory: write one zstd frame per block plus an offset index in the header,
/// and seek to the frame. The on-disk magic already gates that change.
///
/// # Errors
/// - [`FormatError::Io`] when the zstd frame will not decompress.
/// - [`FormatError::BadMagic`] when the payload is not a block archive.
/// - [`FormatError::UnexpectedEof`] when a block runs past the end.
pub fn decode_blocks(path: &str, bytes: &[u8]) -> Result<BlockArchive, FormatError> {
    let raw = zstd::decode_all(bytes).map_err(|e| FormatError::Io {
        path: path.to_owned(),
        source: e,
    })?;

    if raw.len() < BLOCKS_MAGIC.len() || &raw[..BLOCKS_MAGIC.len()] != BLOCKS_MAGIC {
        return Err(FormatError::BadMagic {
            path: path.to_owned(),
            layer: "blocks",
        });
    }

    let mut at = BLOCKS_MAGIC.len();
    if at + 4 > raw.len() {
        return Err(FormatError::UnexpectedEof {
            path: path.to_owned(),
            read: raw.len(),
            expected: at + 4,
        });
    }
    let count = take_u32(&raw, &mut at) as usize;

    let mut archive = BlockArchive::default();
    let per_block = 5 + SQUARE_COUNT * 2;
    for _ in 0..count {
        if at + per_block > raw.len() {
            return Err(FormatError::UnexpectedEof {
                path: path.to_owned(),
                read: raw.len(),
                expected: at + per_block,
            });
        }
        let cx = take_u16(&raw, &mut at);
        let cy = take_u16(&raw, &mut at);
        let relaxed = take_u8(&raw, &mut at) == 1;
        let mut tiles = Vec::with_capacity(SQUARE_COUNT);
        for _ in 0..SQUARE_COUNT {
            tiles.push(TileId::new(take_u16(&raw, &mut at)));
        }
        let cell = CellCoord::new(cx, cy).ok_or(FormatError::UnknownDiscriminant {
            path: path.to_owned(),
            field: "block cell coordinate",
            value: cx.max(cy),
        })?;
        archive.insert(cell, Block { tiles, relaxed });
    }
    Ok(archive)
}
```

Add `pub mod blocks;` to `crates/arda-core/src/formats/mod.rs`, and to `crates/arda-core/src/lib.rs`:

```rust
pub mod tiles;
```

with re-exports:

```rust
pub use formats::blocks::{decode_blocks, encode_blocks, Block, BlockArchive, ZSTD_LEVEL};
pub use tiles::{may_adjoin, tile_def, tiles_in_group, TileDef, TileGroup, TileId, SKELETON_TILES};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p arda-core blocks`
Expected: PASS — 8 tests pass (`archive_round_trips_through_zstd`, `relaxed_mark_survives_the_round_trip`, `encoding_is_independent_of_insertion_order`, `empty_archive_round_trips`, `corrupt_frame_is_refused_naming_the_file`, `vocabulary_has_twenty_four_unique_tiles`, `adjacency_is_symmetric`, `deep_water_never_touches_dry_ground`).

Run: `cargo test -p arda-core`
Expected: PASS — all 37 `arda-core` tests green. **Build-order steps 0–2 are now complete.**

- [ ] **Step 5: Commit**

```bash
git add crates/arda-core/src
git commit -m "feat: add tile vocabulary and zstd block archive codec"
```

---

### Task 9: Integer value noise

Hand-rolled per `code-prefs.md` §Q2 — noise is determinism-critical, so it is never delegated to a crate. Pure integer arithmetic, so it cannot drift across platforms (§Q4).

**Files:**
- Create: `crates/arda-gen/src/noise.rs`
- Modify: `crates/arda-gen/src/lib.rs`

**Interfaces:**
- Consumes: nothing (self-contained integer maths)
- Produces:
  - `fn hash_2d(seed: u64, x: i32, y: i32) -> u32` — a stateless lattice hash
  - `fn value_noise(seed: u64, x: i32, y: i32, period: i32) -> i32` — smooth noise in `-32768..=32767`, bilinear over a lattice of spacing `period`
  - `fn fbm(seed: u64, x: i32, y: i32, period: i32, octaves: u8) -> i32` — summed octaves, halving period and amplitude, result in `-32768..=32767`

- [ ] **Step 1: Write the failing test**

Append to `crates/arda-gen/src/noise.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable_for_the_same_input() {
        assert_eq!(hash_2d(42, 3, 11), hash_2d(42, 3, 11));
    }

    #[test]
    fn hash_separates_transposed_coordinates() {
        assert_ne!(hash_2d(42, 3, 11), hash_2d(42, 11, 3));
    }

    #[test]
    fn hash_separates_sign_flips() {
        assert_ne!(hash_2d(42, -3, 11), hash_2d(42, 3, 11));
        assert_ne!(hash_2d(42, 3, -11), hash_2d(42, 3, 11));
    }

    #[test]
    fn value_noise_stays_in_range() {
        for y in -40..40 {
            for x in -40..40 {
                let v = value_noise(42, x, y, 8);
                assert!((-32768..=32767).contains(&v), "out of range at {x},{y}: {v}");
            }
        }
    }

    #[test]
    fn value_noise_is_continuous_between_lattice_points() {
        // Adjacent samples inside one lattice cell must not jump by more than
        // the lattice delta divided by the period.
        let period = 16;
        let mut worst = 0i32;
        for x in 0..64 {
            let a = value_noise(42, x, 0, period);
            let b = value_noise(42, x + 1, 0, period);
            worst = worst.max((a - b).abs());
        }
        assert!(worst < 65536 / period, "discontinuity of {worst}");
    }

    #[test]
    fn value_noise_hits_lattice_values_exactly() {
        // At a lattice point the interpolation weight is zero, so the sample
        // is the lattice value itself — this pins the interpolation maths.
        let period = 8;
        let lattice = i64::from(hash_2d(42, 0, 0) % 65536) - 32768;
        assert_eq!(i64::from(value_noise(42, 0, 0, period)), lattice);
    }

    #[test]
    fn fbm_stays_in_range() {
        for y in -20..20 {
            for x in -20..20 {
                let v = fbm(42, x, y, 32, 4);
                assert!((-32768..=32767).contains(&v), "out of range at {x},{y}: {v}");
            }
        }
    }

    #[test]
    fn fbm_differs_from_single_octave() {
        assert_ne!(fbm(42, 5, 7, 32, 4), value_noise(42, 5, 7, 32));
    }

    #[test]
    fn different_seeds_give_different_fields() {
        let a: Vec<i32> = (0..32).map(|x| fbm(42, x, 0, 16, 3)).collect();
        let b: Vec<i32> = (0..32).map(|x| fbm(43, x, 0, 16, 3)).collect();
        assert_ne!(a, b);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p arda-gen noise`
Expected: FAIL — `cannot find function 'hash_2d' in this scope`; module not declared.

- [ ] **Step 3: Write minimal implementation**

Prepend to `crates/arda-gen/src/noise.rs`:

```rust
//! Hand-rolled integer value noise (`code-prefs.md` §Q2).
//!
//! Noise feeds relief, so it is determinism-critical: every operation here is
//! integer arithmetic with explicit wrapping, and nothing depends on float
//! rounding (`architecture-interview.md` §Q4).

/// Stateless lattice hash. Splitmix-style avalanche over the packed inputs.
#[must_use]
pub fn hash_2d(seed: u64, x: i32, y: i32) -> u32 {
    // Cast through u32 first so negative coordinates keep their bit pattern
    // and do not collide with their positive counterparts.
    let mut v = seed
        ^ (u64::from(x as u32) << 32)
        ^ u64::from(y as u32).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    v ^= v >> 30;
    v = v.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    v ^= v >> 27;
    v = v.wrapping_mul(0x94D0_49BB_1331_11EB);
    v ^= v >> 31;
    #[allow(clippy::cast_possible_truncation)]
    {
        v as u32
    }
}

/// The lattice value at a lattice index, in `-32768..=32767`.
fn lattice(seed: u64, lx: i32, ly: i32) -> i64 {
    i64::from(hash_2d(seed, lx, ly) % 65536) - 32768
}

/// Smoothstep weight on a `0..period` offset, scaled to `0..=65536`.
fn weight(offset: i32, period: i32) -> i64 {
    // t in 0..=65536, then 3t^2 - 2t^3 evaluated in fixed point.
    let t = i64::from(offset) * 65536 / i64::from(period);
    let t2 = t * t >> 16;
    let t3 = t2 * t >> 16;
    (3 * t2 - 2 * t3).clamp(0, 65536)
}

/// Euclidean-style floor division, so negative coordinates land on the
/// lattice cell below rather than toward zero.
fn floor_div(a: i32, b: i32) -> i32 {
    if a >= 0 {
        a / b
    } else {
        -((-a + b - 1) / b)
    }
}

/// Bilinear value noise with smoothstep weights.
///
/// `period` is the lattice spacing in cells and must be positive.
#[must_use]
pub fn value_noise(seed: u64, x: i32, y: i32, period: i32) -> i32 {
    let period = period.max(1);
    let lx = floor_div(x, period);
    let ly = floor_div(y, period);
    let fx = x - lx * period;
    let fy = y - ly * period;

    let v00 = lattice(seed, lx, ly);
    let v10 = lattice(seed, lx + 1, ly);
    let v01 = lattice(seed, lx, ly + 1);
    let v11 = lattice(seed, lx + 1, ly + 1);

    let wx = weight(fx, period);
    let wy = weight(fy, period);

    let top = v00 + ((v10 - v00) * wx >> 16);
    let bottom = v01 + ((v11 - v01) * wx >> 16);
    let value = top + ((bottom - top) * wy >> 16);

    #[allow(clippy::cast_possible_truncation)]
    {
        value.clamp(-32768, 32767) as i32
    }
}

/// Fractional Brownian motion: `octaves` octaves, each half the period and
/// half the amplitude of the last.
#[must_use]
pub fn fbm(seed: u64, x: i32, y: i32, period: i32, octaves: u8) -> i32 {
    let mut total: i64 = 0;
    let mut amplitude: i64 = 65536;
    let mut normaliser: i64 = 0;
    let mut p = period.max(1);

    for octave in 0..octaves.max(1) {
        let sample = i64::from(value_noise(seed ^ u64::from(octave) << 40, x, y, p));
        total += sample * amplitude >> 16;
        normaliser += amplitude;
        amplitude /= 2;
        p = (p / 2).max(1);
        if amplitude == 0 {
            break;
        }
    }

    if normaliser == 0 {
        return 0;
    }
    #[allow(clippy::cast_possible_truncation)]
    {
        (total * 65536 / normaliser).clamp(-32768, 32767) as i32
    }
}
```

Add to `crates/arda-gen/src/lib.rs`:

```rust
pub mod noise;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p arda-gen noise`
Expected: PASS — 8 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/arda-gen/src
git commit -m "feat: add hand-rolled integer value noise"
```

---

### Task 10: Continent stage — plates, tectonics, coast, upsample

The skeleton runs a **kinematic-lite** tectonics pass (`implementation.md` step 3): real plates, real boundary typing, real coupled response, but 20 steps on a small domain instead of the full 100. Build-order step 4 replaces the loop body after spike S1 without changing this module's signature.

**Files:**
- Create: `crates/arda-gen/src/continent/mod.rs`
- Create: `crates/arda-gen/src/continent/plates.rs`
- Create: `crates/arda-gen/src/continent/tectonics.rs`
- Create: `crates/arda-gen/src/continent/coast.rs`
- Modify: `crates/arda-gen/src/lib.rs`

**Interfaces:**
- Consumes: `GenerateConfig`, `HeightMm`, `SeedKey`/`Tier`/`Stage`/`rng` (Tasks 2–4); `fbm` (Task 9)
- Produces:
  - `enum CrustType { Continental, Oceanic }`
  - `struct Plate { id: u8, centre_x: i32, centre_y: i32, crust: CrustType, drift_x: i32, drift_y: i32 }`
  - `fn seed_plates(seed: u64, sim: SimExtent) -> Vec<Plate>` — 8–14 plates, rim forced oceanic
  - `struct SimExtent { width: i32, height: i32 }` — the 4 km domain, 2x the visible continent
  - `fn plate_of(plates: &[Plate], x: i32, y: i32) -> u8` — Voronoi assignment
  - `fn run_tectonics(seed: u64, plates: &[Plate], sim: SimExtent, steps: u16) -> Vec<i32>` — uplift in mm per 4 km cell
  - `struct ContinentGrid { width: i32, height: i32, height_mm: Vec<i32> }` with `get(x, y) -> HeightMm`, `land_fraction_permille()`
  - `fn generate_continent(seed: u64, config: GenerateConfig) -> ContinentGrid` — the stage entry point, 1 km working grid

- [ ] **Step 1: Write the failing test**

Append to `crates/arda-gen/src/continent/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::GenerateConfig;

    #[test]
    fn micro_continent_grid_is_the_configured_size() {
        let grid = generate_continent(42, GenerateConfig::MICRO);
        assert_eq!(grid.width(), 102);
        assert_eq!(grid.height(), 204);
    }

    #[test]
    fn generation_is_deterministic() {
        let a = generate_continent(42, GenerateConfig::MICRO);
        let b = generate_continent(42, GenerateConfig::MICRO);
        assert_eq!(a, b);
    }

    #[test]
    fn different_seeds_give_different_continents() {
        let a = generate_continent(42, GenerateConfig::MICRO);
        let b = generate_continent(43, GenerateConfig::MICRO);
        assert_ne!(a, b);
    }

    #[test]
    fn every_edge_cell_is_ocean() {
        // logic/01 invariant: every map-edge cell is ocean (mockup Q22).
        let grid = generate_continent(42, GenerateConfig::MICRO);
        let (w, h) = (grid.width(), grid.height());
        for x in 0..w {
            assert!(grid.get(x, 0).raw() < 0, "top edge at {x} is land");
            assert!(grid.get(x, h - 1).raw() < 0, "bottom edge at {x} is land");
        }
        for y in 0..h {
            assert!(grid.get(0, y).raw() < 0, "left edge at {y} is land");
            assert!(grid.get(w - 1, y).raw() < 0, "right edge at {y} is land");
        }
    }

    #[test]
    fn land_fraction_is_within_the_validation_gate() {
        // logic/01 step 9: land fraction within 25-90%.
        let grid = generate_continent(42, GenerateConfig::MICRO);
        let permille = grid.land_fraction_permille();
        assert!(
            (250..=900).contains(&permille),
            "land fraction {permille} per mille is outside 250..=900"
        );
    }

    #[test]
    fn relief_has_real_range() {
        // A flat plate would pass the other tests; assert the continent
        // actually has mountains and sea floor.
        let grid = generate_continent(42, GenerateConfig::MICRO);
        let max = (0..grid.height())
            .flat_map(|y| (0..grid.width()).map(move |x| (x, y)))
            .map(|(x, y)| grid.get(x, y).raw())
            .max()
            .unwrap_or(0);
        assert!(max > 400_000, "highest point is only {max} mm");
    }
}
```

Append to `crates/arda-gen/src/continent/plates.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn sim() -> SimExtent {
        SimExtent {
            width: 51,
            height: 102,
        }
    }

    #[test]
    fn plate_count_is_within_the_interviewed_range() {
        // logic/01 step 1: seed 8-14 plates.
        for seed in 0..20u64 {
            let n = seed_plates(seed, sim()).len();
            assert!((8..=14).contains(&n), "seed {seed} produced {n} plates");
        }
    }

    #[test]
    fn plate_seeding_is_deterministic() {
        assert_eq!(seed_plates(42, sim()), seed_plates(42, sim()));
    }

    #[test]
    fn rim_plates_are_oceanic() {
        // logic/01 step 1: the domain rim is forced oceanic.
        let plates = seed_plates(42, sim());
        let s = sim();
        for p in &plates {
            let near_rim = p.centre_x < s.width / 5
                || p.centre_x > s.width * 4 / 5
                || p.centre_y < s.height / 5
                || p.centre_y > s.height * 4 / 5;
            if near_rim {
                assert_eq!(p.crust, CrustType::Oceanic, "rim plate {} is continental", p.id);
            }
        }
    }

    #[test]
    fn at_least_one_plate_is_continental() {
        let plates = seed_plates(42, sim());
        assert!(plates.iter().any(|p| p.crust == CrustType::Continental));
    }

    #[test]
    fn voronoi_assignment_is_total_and_stable() {
        let plates = seed_plates(42, sim());
        let s = sim();
        for y in 0..s.height {
            for x in 0..s.width {
                let id = plate_of(&plates, x, y);
                assert!(plates.iter().any(|p| p.id == id));
                assert_eq!(id, plate_of(&plates, x, y));
            }
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p arda-gen continent`
Expected: FAIL — `cannot find function 'generate_continent' in this scope`.

- [ ] **Step 3: Write minimal implementation**

Create `crates/arda-gen/src/continent/plates.rs` (prepend above the test module):

```rust
//! Voronoi plate seeding (`logic/01` step 1).

use arda_core::{rng, SeedKey, Stage, Tier};
use rand_core::RngCore;

/// The 4 km simulation domain, roughly twice the visible continent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimExtent {
    /// Domain width in 4 km cells.
    pub width: i32,
    /// Domain height in 4 km cells.
    pub height: i32,
}

/// Whether a plate carries continental or oceanic crust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrustType {
    /// Thick, buoyant crust — sits above sea level once isostasy applies.
    Continental,
    /// Thin, dense crust — sits well below sea level.
    Oceanic,
}

/// One tectonic plate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plate {
    /// Plate identifier, used as the Voronoi label.
    pub id: u8,
    /// Voronoi site column on the 4 km grid.
    pub centre_x: i32,
    /// Voronoi site row on the 4 km grid.
    pub centre_y: i32,
    /// Crust type.
    pub crust: CrustType,
    /// Drift per step, columns.
    pub drift_x: i32,
    /// Drift per step, rows.
    pub drift_y: i32,
}

/// Seeds 8-14 plates as Voronoi sites, forcing the domain rim oceanic so map
/// edges are guaranteed ocean (`logic/01` step 1, mockup Q22).
#[must_use]
pub fn seed_plates(seed: u64, sim: SimExtent) -> Vec<Plate> {
    let mut r = rng(seed, SeedKey::new(Tier::Continent, Stage::Plates, 0, 0, 0));
    let count = 8 + (r.next_u32() % 7) as u8; // 8..=14

    (0..count)
        .map(|id| {
            let centre_x = (r.next_u32() % sim.width.unsigned_abs()) as i32;
            let centre_y = (r.next_u32() % sim.height.unsigned_abs()) as i32;

            let near_rim = centre_x < sim.width / 5
                || centre_x > sim.width * 4 / 5
                || centre_y < sim.height / 5
                || centre_y > sim.height * 4 / 5;

            // Rim plates are always oceanic; interior plates are continental
            // about two times in three, which keeps land fraction inside the
            // step-9 gate without tuning.
            let crust = if near_rim || r.next_u32() % 3 == 0 {
                CrustType::Oceanic
            } else {
                CrustType::Continental
            };

            Plate {
                id,
                centre_x,
                centre_y,
                crust,
                drift_x: (r.next_u32() % 5) as i32 - 2,
                drift_y: (r.next_u32() % 5) as i32 - 2,
            }
        })
        .collect()
}

/// Nearest-site Voronoi assignment. Ties break toward the lower plate id, so
/// the result never depends on iteration order (§Q4).
#[must_use]
pub fn plate_of(plates: &[Plate], x: i32, y: i32) -> u8 {
    let mut best_id = 0u8;
    let mut best_d2 = i64::MAX;
    for p in plates {
        let dx = i64::from(x - p.centre_x);
        let dy = i64::from(y - p.centre_y);
        let d2 = dx * dx + dy * dy;
        if d2 < best_d2 {
            best_d2 = d2;
            best_id = p.id;
        }
    }
    best_id
}
```

Create `crates/arda-gen/src/continent/tectonics.rs`:

```rust
//! Time-stepped tectonics, coupled with erosion (`logic/01` steps 2-3).
//!
//! Skeleton scope: kinematic-lite — real boundary typing and a real coupled
//! response, 20 steps instead of the full 100. Build-order step 4 replaces
//! the loop body after spike S1; the signature stays.

use super::plates::{plate_of, CrustType, Plate, SimExtent};

/// Uplift added per step at a convergent continental collision, millimetres.
const COLLISION_UPLIFT_MM: i32 = 62_000;
/// Uplift added per step at a subduction arc, millimetres.
const ARC_UPLIFT_MM: i32 = 41_000;
/// Subsidence per step at a rift, millimetres.
const RIFT_SUBSIDENCE_MM: i32 = 24_000;

/// Accumulated uplift in millimetres per 4 km cell.
///
/// Boundary type follows the two plates' crust types and relative motion
/// (`logic/01` §Q5); after each step a coarse diffusion pass lets relief
/// respond, which is the artifact's causality rule at continental scale.
#[must_use]
pub fn run_tectonics(plates: &[Plate], sim: SimExtent, steps: u16) -> Vec<i32> {
    let w = sim.width;
    let h = sim.height;
    let mut uplift = vec![0i32; (w * h) as usize];

    let owner: Vec<u8> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| plate_of(plates, x, y))
        .collect();

    for _ in 0..steps {
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let i = (y * w + x) as usize;
                let mine = owner[i];
                let Some(a) = plates.iter().find(|p| p.id == mine) else {
                    continue;
                };

                // Only the four orthogonal neighbours, in a fixed order.
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let j = ((y + dy) * w + (x + dx)) as usize;
                    let theirs = owner[j];
                    if theirs == mine {
                        continue;
                    }
                    let Some(b) = plates.iter().find(|p| p.id == theirs) else {
                        continue;
                    };

                    // Relative motion projected onto the boundary normal:
                    // negative means the plates are approaching.
                    let closing = (b.drift_x - a.drift_x) * dx + (b.drift_y - a.drift_y) * dy;

                    uplift[i] += match (a.crust, b.crust, closing) {
                        (CrustType::Continental, CrustType::Continental, c) if c < 0 => {
                            COLLISION_UPLIFT_MM
                        }
                        (CrustType::Continental, CrustType::Oceanic, c)
                        | (CrustType::Oceanic, CrustType::Continental, c)
                            if c < 0 =>
                        {
                            ARC_UPLIFT_MM
                        }
                        (_, _, c) if c > 0 => -RIFT_SUBSIDENCE_MM,
                        // Transform: no vertical component.
                        _ => 0,
                    };
                }
            }
        }
        diffuse(&mut uplift, w, h);
    }
    uplift
}

/// One coarse erosion/isostasy pass: a fixed-weight five-point stencil.
///
/// Integer arithmetic with a fixed divisor, so it is bit-identical anywhere.
fn diffuse(field: &mut [i32], w: i32, h: i32) {
    let source = field.to_vec();
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let i = (y * w + x) as usize;
            let sum = i64::from(source[i]) * 4
                + i64::from(source[i - 1])
                + i64::from(source[i + 1])
                + i64::from(source[i - w as usize])
                + i64::from(source[i + w as usize]);
            #[allow(clippy::cast_possible_truncation)]
            {
                field[i] = (sum / 8) as i32;
            }
        }
    }
}
```

Create `crates/arda-gen/src/continent/coast.rs`:

```rust
//! Isostatic base elevation and the coastline cut (`logic/01` step 3).

use super::plates::{CrustType, Plate};

/// Base elevation of continental crust before tectonics, millimetres.
pub const CONTINENTAL_BASE_MM: i32 = 320_000;
/// Base elevation of oceanic crust before tectonics, millimetres.
pub const OCEANIC_BASE_MM: i32 = -2_100_000;

/// Base elevation for a cell owned by `plate`.
#[must_use]
pub fn base_elevation_mm(plate: &Plate) -> i32 {
    match plate.crust {
        CrustType::Continental => CONTINENTAL_BASE_MM,
        CrustType::Oceanic => OCEANIC_BASE_MM,
    }
}

/// Forces a cell to ocean when it lies within `margin` cells of the visible
/// map edge (`logic/01` invariant: every map-edge cell is ocean).
#[must_use]
pub fn rim_forced_ocean(x: i32, y: i32, width: i32, height: i32, margin: i32) -> bool {
    x < margin || y < margin || x >= width - margin || y >= height - margin
}
```

Create `crates/arda-gen/src/continent/mod.rs` (prepend above the test module):

```rust
//! Continent generation (`logic/01`).
//!
//! Skeleton scope: plates, kinematic-lite tectonics, coast, and the 4 km to
//! 1 km upsample. Climate, hydrology, human geography, and naming arrive at
//! build-order step 4.

pub mod coast;
pub mod plates;
pub mod tectonics;

use crate::noise::fbm;
use arda_core::{GenerateConfig, HeightMm};
use coast::{base_elevation_mm, rim_forced_ocean};
use plates::{plate_of, SimExtent};

/// Simulation cell size in kilometres (`logic/01` step 2).
const SIM_CELL_KM: i32 = 4;
/// Tectonic steps in the skeleton pass; the full stage runs 100.
const SKELETON_STEPS: u16 = 20;
/// Cells of guaranteed ocean at the visible map edge.
const RIM_MARGIN: i32 = 2;

/// The 1 km continent working grid (`logic/01` step 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinentGrid {
    width: i32,
    height: i32,
    height_mm: Vec<i32>,
}

impl ContinentGrid {
    /// Grid width in 1 km cells.
    #[must_use]
    pub const fn width(&self) -> i32 {
        self.width
    }

    /// Grid height in 1 km cells.
    #[must_use]
    pub const fn height(&self) -> i32 {
        self.height
    }

    /// Elevation at a cell; out-of-range coordinates clamp to the edge, which
    /// is always ocean.
    #[must_use]
    pub fn get(&self, x: i32, y: i32) -> HeightMm {
        let cx = x.clamp(0, self.width - 1);
        let cy = y.clamp(0, self.height - 1);
        HeightMm::new(self.height_mm[(cy * self.width + cx) as usize])
    }

    /// Land cells per thousand (`logic/01` step 9 gate).
    #[must_use]
    pub fn land_fraction_permille(&self) -> u16 {
        let land = self.height_mm.iter().filter(|&&h| h > 0).count();
        let total = self.height_mm.len().max(1);
        #[allow(clippy::cast_possible_truncation)]
        {
            (land * 1000 / total) as u16
        }
    }
}

/// Runs the skeleton continent stage.
///
/// Steps, in the `logic/01` order: seed plates on a 2x domain, run the
/// coupled tectonics loop, apply isostatic base elevation, cut the coastline
/// at sea level, then upsample 4 km to 1 km with noise refinement.
#[must_use]
pub fn generate_continent(seed: u64, config: GenerateConfig) -> ContinentGrid {
    #[allow(clippy::cast_possible_wrap)]
    let vis_w = config.size_km().width as i32;
    #[allow(clippy::cast_possible_wrap)]
    let vis_h = config.size_km().height as i32;

    // Step 1: plates on a domain twice the visible continent.
    let sim = SimExtent {
        width: (vis_w * 2 / SIM_CELL_KM).max(8),
        height: (vis_h * 2 / SIM_CELL_KM).max(8),
    };
    let plates = plates::seed_plates(seed, sim);

    // Step 2: coupled tectonics.
    let uplift = tectonics::run_tectonics(&plates, sim, SKELETON_STEPS);

    // Step 3: isostasy plus accumulated uplift, on the 4 km grid.
    let coarse: Vec<i32> = (0..sim.height)
        .flat_map(|y| (0..sim.width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let id = plate_of(&plates, x, y);
            let base = plates
                .iter()
                .find(|p| p.id == id)
                .map_or(coast::OCEANIC_BASE_MM, base_elevation_mm);
            base.saturating_add(uplift[(y * sim.width + x) as usize])
        })
        .collect();

    // Step 4: upsample to the 1 km working grid, noise-refined. The visible
    // continent is the centre of the domain.
    let off_x = (sim.width * SIM_CELL_KM - vis_w) / 2;
    let off_y = (sim.height * SIM_CELL_KM - vis_h) / 2;

    let mut height_mm = Vec::with_capacity((vis_w * vis_h) as usize);
    for y in 0..vis_h {
        for x in 0..vis_w {
            let dx = x + off_x;
            let dy = y + off_y;
            let mut h = sample_coarse(&coarse, sim, dx, dy);

            // Noise refinement: +/- 180 m of detail, only on land, so the
            // coastline stays where tectonics put it.
            if h > 0 {
                let detail = fbm(seed ^ 0x00DE_7A11, x, y, 24, 4);
                h = h.saturating_add(detail * 180_000 / 32_768);
            }

            if rim_forced_ocean(x, y, vis_w, vis_h, RIM_MARGIN) {
                h = h.min(coast::OCEANIC_BASE_MM / 2);
            }
            height_mm.push(h);
        }
    }

    ContinentGrid {
        width: vis_w,
        height: vis_h,
        height_mm,
    }
}

/// Bilinear sample of the 4 km grid at 1 km resolution, in fixed point.
fn sample_coarse(coarse: &[i32], sim: SimExtent, km_x: i32, km_y: i32) -> i32 {
    let gx = (km_x / SIM_CELL_KM).clamp(0, sim.width - 1);
    let gy = (km_y / SIM_CELL_KM).clamp(0, sim.height - 1);
    let gx1 = (gx + 1).min(sim.width - 1);
    let gy1 = (gy + 1).min(sim.height - 1);

    let fx = i64::from(km_x % SIM_CELL_KM) * 65536 / i64::from(SIM_CELL_KM);
    let fy = i64::from(km_y % SIM_CELL_KM) * 65536 / i64::from(SIM_CELL_KM);

    let at = |x: i32, y: i32| i64::from(coarse[(y * sim.width + x) as usize]);
    let top = at(gx, gy) + ((at(gx1, gy) - at(gx, gy)) * fx >> 16);
    let bottom = at(gx, gy1) + ((at(gx1, gy1) - at(gx, gy1)) * fx >> 16);

    #[allow(clippy::cast_possible_truncation)]
    {
        (top + ((bottom - top) * fy >> 16)) as i32
    }
}
```

Add to `crates/arda-gen/src/lib.rs`:

```rust
pub mod continent;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p arda-gen continent`
Expected: PASS — 11 tests pass (6 in `continent::tests`, 5 in `plates::tests`).

If `land_fraction_is_within_the_validation_gate` fails, tune `CONTINENTAL_BASE_MM` in `coast.rs` first — it is the single knob that moves land fraction. Do **not** weaken the assertion: it is the `logic/01` step 9 gate.

- [ ] **Step 5: Commit**

```bash
git add crates/arda-gen/src
git commit -m "feat: add skeleton continent stage with plates and tectonics"
```

---

### Task 11: Tile bundles and the pinned-edge function

The load-bearing seam. `logic/01`'s invariant is that a bundle depends on **coarse data + seed only, never on any area's fine output** — that is what lets areas generate in any order. This task makes edge agreement structural rather than something a later stage has to repair: `boundary_height` is a pure function of *absolute* cell coordinates, so two neighbours computing their shared edge call the same function with the same arguments and cannot disagree.

**Files:**
- Create: `crates/arda-gen/src/continent/bundles.rs`
- Modify: `crates/arda-gen/src/continent/mod.rs`

**Interfaces:**
- Consumes: `ContinentGrid` (Task 10), `AreaCoord`/`AREA_CELLS` (Task 2), `fbm` (Task 9)
- Produces:
  - `fn boundary_height(seed: u64, continent: &ContinentGrid, abs_x: i32, abs_y: i32) -> i32` — millimetres, the single source of every area's relief
  - `struct TileBundle { area: AreaCoord, north: Vec<i32>, south: Vec<i32>, east: Vec<i32>, west: Vec<i32>, mean_height_mm: i32 }`
  - `fn bundle_for(seed: u64, continent: &ContinentGrid, area: AreaCoord) -> TileBundle`
  - `fn abs_cell(area: AreaCoord, local_x: u16, local_y: u16) -> (i32, i32)`

- [ ] **Step 1: Write the failing test**

Append to `crates/arda-gen/src/continent/bundles.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::{AreaCoord, GenerateConfig, AREA_CELLS};

    fn fixture() -> ContinentGrid {
        crate::continent::generate_continent(42, GenerateConfig::MICRO)
    }

    #[test]
    fn abs_cell_maps_tiles_without_overlap() {
        assert_eq!(abs_cell(AreaCoord::new(0, 0), 0, 0), (0, 0));
        assert_eq!(abs_cell(AreaCoord::new(0, 0), 511, 511), (511, 511));
        assert_eq!(abs_cell(AreaCoord::new(1, 0), 0, 0), (512, 0));
        assert_eq!(abs_cell(AreaCoord::new(1, 3), 0, 0), (512, 1536));
    }

    #[test]
    fn boundary_height_is_deterministic() {
        let c = fixture();
        assert_eq!(
            boundary_height(42, &c, 700, 1200),
            boundary_height(42, &c, 700, 1200)
        );
    }

    /// The invariant this whole task exists for: the east edge of tile (0, y)
    /// and the west edge of tile (1, y) are the same cells, so they must be
    /// the same numbers — `implementation.md` "Pinned edges".
    #[test]
    fn neighbouring_tiles_agree_on_their_shared_vertical_edge() {
        let c = fixture();
        let left = bundle_for(42, &c, AreaCoord::new(0, 1));
        let right = bundle_for(42, &c, AreaCoord::new(1, 1));
        assert_eq!(left.east, right.west);
        assert_eq!(left.east.len(), AREA_CELLS as usize);
    }

    #[test]
    fn neighbouring_tiles_agree_on_their_shared_horizontal_edge() {
        let c = fixture();
        let top = bundle_for(42, &c, AreaCoord::new(1, 1));
        let bottom = bundle_for(42, &c, AreaCoord::new(1, 2));
        assert_eq!(top.south, bottom.north);
    }

    #[test]
    fn bundles_do_not_depend_on_the_order_they_are_built() {
        let c = fixture();
        let forward: Vec<_> = GenerateConfig::MICRO
            .area_coords()
            .map(|a| bundle_for(42, &c, a))
            .collect();
        let mut coords: Vec<_> = GenerateConfig::MICRO.area_coords().collect();
        coords.reverse();
        let mut backward: Vec<_> = coords.iter().map(|&a| bundle_for(42, &c, a)).collect();
        backward.reverse();
        assert_eq!(forward, backward);
    }

    #[test]
    fn every_micro_tile_gets_a_bundle() {
        let c = fixture();
        let bundles: Vec<_> = GenerateConfig::MICRO
            .area_coords()
            .map(|a| bundle_for(42, &c, a))
            .collect();
        assert_eq!(bundles.len(), 8);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p arda-gen bundles`
Expected: FAIL — `cannot find function 'bundle_for' in this scope`.

- [ ] **Step 3: Write minimal implementation**

Prepend to `crates/arda-gen/src/continent/bundles.rs`:

```rust
//! Per-tile input bundles (`logic/01` step 10).
//!
//! A bundle is a pure function of coarse continent data plus the seed — never
//! of any area's fine output. Adjacent tiles are therefore mirror-consistent
//! by construction, and areas may generate in any order (`logic/01`
//! invariant).

use super::ContinentGrid;
use crate::noise::fbm;
use arda_core::{AreaCoord, AREA_CELLS};

/// Area cells per continent kilometre: cells are 100 m, so ten.
const CELLS_PER_KM: i32 = 10;

/// Absolute cell coordinates of a tile-local cell.
#[must_use]
pub fn abs_cell(area: AreaCoord, local_x: u16, local_y: u16) -> (i32, i32) {
    (
        area.x * i32::from(AREA_CELLS) + i32::from(local_x),
        area.y * i32::from(AREA_CELLS) + i32::from(local_y),
    )
}

/// Terrain elevation at an absolute cell, in millimetres.
///
/// This is the single source of area relief. Because it takes absolute
/// coordinates, two tiles sharing an edge call it with identical arguments
/// and get identical answers — the pinned-edge guarantee
/// (`implementation.md` "Pinned edges", `logic/02` amendment 3).
#[must_use]
pub fn boundary_height(seed: u64, continent: &ContinentGrid, abs_x: i32, abs_y: i32) -> i32 {
    // Bilinear sample of the 1 km continent grid at 100 m resolution.
    let km_x = abs_x.div_euclid(CELLS_PER_KM);
    let km_y = abs_y.div_euclid(CELLS_PER_KM);
    let fx = i64::from(abs_x.rem_euclid(CELLS_PER_KM)) * 65536 / i64::from(CELLS_PER_KM);
    let fy = i64::from(abs_y.rem_euclid(CELLS_PER_KM)) * 65536 / i64::from(CELLS_PER_KM);

    let at = |x: i32, y: i32| i64::from(continent.get(x, y).raw());
    let top = at(km_x, km_y) + ((at(km_x + 1, km_y) - at(km_x, km_y)) * fx >> 16);
    let bottom = at(km_x, km_y + 1) + ((at(km_x + 1, km_y + 1) - at(km_x, km_y + 1)) * fx >> 16);
    let coarse = top + ((bottom - top) * fy >> 16);

    // Area-scale detail, keyed by absolute position so it is edge-safe.
    // Amplitude scales down toward sea level, so detail never manufactures
    // coastline the continent stage did not put there (§Q8: area detail
    // refines coarse features but never relocates them).
    let amplitude = if coarse > 0 {
        (coarse / 12).clamp(2_000, 90_000)
    } else {
        1_500
    };
    let detail = i64::from(fbm(seed ^ 0x00A1_2EA5, abs_x, abs_y, 40, 5));

    #[allow(clippy::cast_possible_truncation)]
    {
        (coarse + (detail * i64::from(amplitude) >> 15)) as i32
    }
}

/// One area tile's inputs, computed from coarse data and the seed only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileBundle {
    /// Which tile this bundle feeds.
    pub area: AreaCoord,
    /// Heights along the tile's northern edge row, west to east.
    pub north: Vec<i32>,
    /// Heights along the tile's southern edge row, west to east.
    pub south: Vec<i32>,
    /// Heights along the tile's eastern edge column, north to south.
    pub east: Vec<i32>,
    /// Heights along the tile's western edge column, north to south.
    pub west: Vec<i32>,
    /// Mean elevation across the tile, for cheap summaries.
    pub mean_height_mm: i32,
}

/// Builds one tile's bundle.
#[must_use]
pub fn bundle_for(seed: u64, continent: &ContinentGrid, area: AreaCoord) -> TileBundle {
    let n = AREA_CELLS;
    let last = n - 1;

    let row = |local_y: u16| -> Vec<i32> {
        (0..n)
            .map(|local_x| {
                let (ax, ay) = abs_cell(area, local_x, local_y);
                boundary_height(seed, continent, ax, ay)
            })
            .collect()
    };
    let column = |local_x: u16| -> Vec<i32> {
        (0..n)
            .map(|local_y| {
                let (ax, ay) = abs_cell(area, local_x, local_y);
                boundary_height(seed, continent, ax, ay)
            })
            .collect()
    };

    let north = row(0);
    let south = row(last);
    let west = column(0);
    let east = column(last);

    let sum: i64 = north
        .iter()
        .chain(south.iter())
        .chain(east.iter())
        .chain(west.iter())
        .map(|&h| i64::from(h))
        .sum();
    #[allow(clippy::cast_possible_truncation)]
    let mean_height_mm = (sum / (4 * i64::from(n))) as i32;

    TileBundle {
        area,
        north,
        south,
        east,
        west,
        mean_height_mm,
    }
}
```

Add `pub mod bundles;` to `crates/arda-gen/src/continent/mod.rs`.

> **Note on the shared edge.** `bundle_for` gives tile `(0, y)` its east column at local x = 511 and tile `(1, y)` its west column at local x = 0, which are absolute columns 511 and 512 — adjacent, not identical. The test asserts they are *equal*, which holds because `boundary_height` is smooth across that seam only if both tiles sample the same column. Make `east` sample local x = `AREA_CELLS` (absolute 512, i.e. the neighbour's first column) so the two are literally the same cell. Adjust `column(last)` to `column_abs(i32::from(n))` taking an absolute local offset that may equal `AREA_CELLS`; `abs_cell` already accepts any `u16`, so pass `n` rather than `last` for the east/south edges.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p arda-gen bundles`
Expected: PASS — 6 tests pass (`abs_cell_maps_tiles_without_overlap`, `boundary_height_is_deterministic`, `neighbouring_tiles_agree_on_their_shared_vertical_edge`, `neighbouring_tiles_agree_on_their_shared_horizontal_edge`, `bundles_do_not_depend_on_the_order_they_are_built`, `every_micro_tile_gets_a_bundle`).

- [ ] **Step 5: Commit**

```bash
git add crates/arda-gen/src
git commit -m "feat: add tile bundles with structurally pinned edges"
```

---

### Task 12: Area stage — relief and water

Two of the seven area stages, at skeleton fidelity (`implementation.md` step 3: "relief+water only area"). Stage purity is enforced by the signatures: `water` takes `relief`'s output and nothing else, so the causal rule from `logic/02` is checked by the compiler.

**Files:**
- Create: `crates/arda-gen/src/area/mod.rs`
- Create: `crates/arda-gen/src/area/relief.rs`
- Create: `crates/arda-gen/src/area/water.rs`
- Modify: `crates/arda-gen/src/lib.rs`

**Interfaces:**
- Consumes: `TileBundle`/`boundary_height`/`abs_cell` (Task 11), `ContinentGrid` (Task 10), `AreaCells`/`Cell`/`TerrainKind`/`Cover` (Task 6), `AreaObjects`/`RiverSegment` (Task 7)
- Produces:
  - `struct ReliefGrid { heights: Vec<i32> }` with `get(CellCoord) -> i32`
  - `fn relief(seed: u64, continent: &ContinentGrid, bundle: &TileBundle) -> ReliefGrid`
  - `struct WaterGrid { drainage: Vec<u32>, order: Vec<u8>, downstream: Vec<Option<u32>> }`
  - `fn water(relief: &ReliefGrid) -> WaterGrid`
  - `fn compose(relief: &ReliefGrid, water: &WaterGrid) -> (AreaCells, AreaObjects)`
  - `fn generate_area(seed: u64, continent: &ContinentGrid, bundle: &TileBundle) -> (AreaCells, AreaObjects)`
  - `const CHANNEL_THRESHOLD_CELLS: u32 = 240`

- [ ] **Step 1: Write the failing test**

Append to `crates/arda-gen/src/area/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::continent::bundles::bundle_for;
    use crate::continent::generate_continent;
    use arda_core::{AreaCoord, CellCoord, GenerateConfig, TerrainKind, AREA_CELLS};

    fn setup(area: AreaCoord) -> (ContinentGrid, TileBundle) {
        let c = generate_continent(42, GenerateConfig::MICRO);
        let b = bundle_for(42, &c, area);
        (c, b)
    }

    fn cc(x: u16, y: u16) -> CellCoord {
        CellCoord::new(x, y).unwrap()
    }

    #[test]
    fn area_generation_is_deterministic() {
        let (c, b) = setup(AreaCoord::new(1, 1));
        assert_eq!(generate_area(42, &c, &b), generate_area(42, &c, &b));
    }

    #[test]
    fn relief_matches_the_bundle_on_the_northern_edge() {
        // logic/02 amendment 3: edge heights are pinned to the bundle.
        let (c, b) = setup(AreaCoord::new(1, 1));
        let r = relief(42, &c, &b);
        for x in 0..AREA_CELLS {
            assert_eq!(
                r.get(cc(x, 0)),
                b.north[x as usize],
                "north edge mismatch at x={x}"
            );
        }
    }

    #[test]
    fn relief_matches_the_bundle_on_the_western_edge() {
        let (c, b) = setup(AreaCoord::new(1, 1));
        let r = relief(42, &c, &b);
        for y in 0..AREA_CELLS {
            assert_eq!(r.get(cc(0, y)), b.west[y as usize], "west edge mismatch at y={y}");
        }
    }

    /// The cross-tile agreement gate (`implementation.md` step 5
    /// verification, brought forward because the skeleton must not bake in a
    /// seam it cannot later remove).
    #[test]
    fn adjacent_tiles_agree_on_their_shared_edge_cells() {
        let c = generate_continent(42, GenerateConfig::MICRO);
        let left = bundle_for(42, &c, AreaCoord::new(0, 1));
        let right = bundle_for(42, &c, AreaCoord::new(1, 1));
        let lr = relief(42, &c, &left);
        let rr = relief(42, &c, &right);
        for y in 0..AREA_CELLS {
            assert_eq!(
                lr.get(cc(AREA_CELLS - 1, y)),
                rr.get(cc(0, y)),
                "tiles disagree at shared row y={y}"
            );
        }
    }

    #[test]
    fn every_land_cell_has_a_downstream_or_reaches_the_edge() {
        // logic/01 invariant, at area scale: water always has somewhere to go.
        let (c, b) = setup(AreaCoord::new(1, 1));
        let r = relief(42, &c, &b);
        let w = water(&r);
        for y in 0..AREA_CELLS {
            for x in 0..AREA_CELLS {
                let at = cc(x, y);
                let on_edge = x == 0 || y == 0 || x == AREA_CELLS - 1 || y == AREA_CELLS - 1;
                if r.get(at) > 0 && !on_edge {
                    assert!(
                        w.downstream_of(at).is_some(),
                        "landlocked cell at {x},{y} has no downstream"
                    );
                }
            }
        }
    }

    #[test]
    fn drainage_area_is_at_least_one_everywhere() {
        let (c, b) = setup(AreaCoord::new(1, 1));
        let w = water(&relief(42, &c, &b));
        for y in 0..AREA_CELLS {
            for x in 0..AREA_CELLS {
                assert!(w.drainage_at(cc(x, y)) >= 1);
            }
        }
    }

    #[test]
    fn drainage_increases_downstream() {
        // The defining property of flow accumulation.
        let (c, b) = setup(AreaCoord::new(1, 1));
        let r = relief(42, &c, &b);
        let w = water(&r);
        for y in 1..AREA_CELLS - 1 {
            for x in 1..AREA_CELLS - 1 {
                let at = cc(x, y);
                if let Some(down) = w.downstream_of(at) {
                    assert!(
                        w.drainage_at(down) >= w.drainage_at(at),
                        "drainage shrank downstream of {x},{y}"
                    );
                }
            }
        }
    }

    #[test]
    fn channels_appear_and_carry_strahler_order() {
        let (c, b) = setup(AreaCoord::new(1, 1));
        let (cells, objects) = generate_area(42, &c, &b);
        let channel_cells = (0..AREA_CELLS)
            .flat_map(|y| (0..AREA_CELLS).map(move |x| cc(x, y)))
            .filter(|&at| cells.get(at).watercourse_order > 0)
            .count();
        assert!(channel_cells > 0, "no channels were cut");
        assert!(!objects.rivers.is_empty(), "no river segments were emitted");
        assert!(objects.rivers.iter().all(|r| r.order >= 1));
    }

    #[test]
    fn sea_cells_are_marked_and_carry_no_channel() {
        let (c, b) = setup(AreaCoord::new(0, 0));
        let (cells, _) = generate_area(42, &c, &b);
        let mut saw_sea = false;
        for y in 0..AREA_CELLS {
            for x in 0..AREA_CELLS {
                let cell = cells.get(cc(x, y));
                if cell.terrain == TerrainKind::Sea {
                    saw_sea = true;
                    assert_eq!(cell.watercourse_order, 0);
                }
            }
        }
        assert!(saw_sea, "the corner tile should contain ocean");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p arda-gen area`
Expected: FAIL — `cannot find function 'generate_area' in this scope`.

- [ ] **Step 3: Write minimal implementation**

Create `crates/arda-gen/src/area/relief.rs`:

```rust
//! Area relief (`logic/02`), conditioned on the tile bundle.
//!
//! Every cell — edges included — comes from `boundary_height`, so the pinned
//! edges are not a repair step but the definition (`logic/02` amendment 3).

use crate::continent::bundles::{abs_cell, boundary_height, TileBundle};
use crate::continent::ContinentGrid;
use arda_core::{CellCoord, AREA_CELLS};

/// One tile's elevation field, in millimetres.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReliefGrid {
    heights: Vec<i32>,
}

impl ReliefGrid {
    /// Elevation at a cell.
    #[must_use]
    pub fn get(&self, at: CellCoord) -> i32 {
        self.heights[at.index()]
    }
}

/// Builds the tile's relief.
#[must_use]
pub fn relief(seed: u64, continent: &ContinentGrid, bundle: &TileBundle) -> ReliefGrid {
    let n = AREA_CELLS;
    let mut heights = Vec::with_capacity(n as usize * n as usize);
    for y in 0..n {
        for x in 0..n {
            let (ax, ay) = abs_cell(bundle.area, x, y);
            heights.push(boundary_height(seed, continent, ax, ay));
        }
    }
    ReliefGrid { heights }
}
```

Create `crates/arda-gen/src/area/water.rs`:

```rust
//! Area drainage (`logic/02` water).
//!
//! D8 flow with a fixed neighbour order, so ties break identically on every
//! platform and the result never depends on iteration order (§Q4).

use super::relief::ReliefGrid;
use arda_core::{CellCoord, AREA_CELLS};

/// Upstream cells needed before a channel is cut.
pub const CHANNEL_THRESHOLD_CELLS: u32 = 240;

/// The eight neighbour offsets, in a fixed order. Ties in the steepest-descent
/// search resolve to the first entry, which makes D8 deterministic.
const NEIGHBOURS: [(i32, i32); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

/// Drainage directions, accumulation, and Strahler order for one tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaterGrid {
    drainage: Vec<u32>,
    order: Vec<u8>,
    downstream: Vec<Option<u32>>,
}

impl WaterGrid {
    /// Upstream cell count draining through a cell.
    #[must_use]
    pub fn drainage_at(&self, at: CellCoord) -> u32 {
        self.drainage[at.index()]
    }

    /// Strahler order at a cell; 0 when below the channel threshold.
    #[must_use]
    pub fn order_at(&self, at: CellCoord) -> u8 {
        self.order[at.index()]
    }

    /// The cell this one drains into, when it has one.
    #[must_use]
    pub fn downstream_of(&self, at: CellCoord) -> Option<CellCoord> {
        let idx = self.downstream[at.index()]?;
        let n = u32::from(AREA_CELLS);
        #[allow(clippy::cast_possible_truncation)]
        CellCoord::new((idx % n) as u16, (idx / n) as u16)
    }
}

/// Computes drainage from relief alone — the causal rule made a signature
/// (`logic/02`: a stage reads only prior stages' outputs).
#[must_use]
pub fn water(relief: &ReliefGrid) -> WaterGrid {
    let n = i32::from(AREA_CELLS);
    let count = (n * n) as usize;

    // Step 1: steepest-descent direction per cell.
    let mut downstream: Vec<Option<u32>> = vec![None; count];
    for y in 0..n {
        for x in 0..n {
            let Some(at) = CellCoord::new(x as u16, y as u16) else {
                continue;
            };
            let here = relief.get(at);
            let mut best_drop = 0i32;
            let mut best: Option<u32> = None;

            for (dx, dy) in NEIGHBOURS {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= n || ny >= n {
                    // Off-tile: water leaves the area, which counts as an
                    // outlet rather than a sink.
                    continue;
                }
                let Some(nb) = CellCoord::new(nx as u16, ny as u16) else {
                    continue;
                };
                let drop = here - relief.get(nb);
                if drop > best_drop {
                    best_drop = drop;
                    #[allow(clippy::cast_sign_loss)]
                    {
                        best = Some((ny * n + nx) as u32);
                    }
                }
            }
            downstream[at.index()] = best;
        }
    }

    // Step 2: flow accumulation, processing cells from high to low so every
    // upstream contribution is already counted. Sorting by (height, index)
    // keeps the order total and therefore deterministic.
    let mut by_height: Vec<(i32, u32)> = (0..count)
        .map(|i| {
            #[allow(clippy::cast_possible_truncation)]
            let idx = i as u32;
            let x = idx % u32::from(AREA_CELLS);
            let y = idx / u32::from(AREA_CELLS);
            #[allow(clippy::cast_possible_truncation)]
            let at = CellCoord::new(x as u16, y as u16);
            (at.map_or(i32::MIN, |c| relief.get(c)), idx)
        })
        .collect();
    by_height.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));

    let mut drainage = vec![1u32; count];
    for &(_, idx) in &by_height {
        if let Some(down) = downstream[idx as usize] {
            drainage[down as usize] = drainage[down as usize].saturating_add(drainage[idx as usize]);
        }
    }

    // Step 3: Strahler order from the accumulation, one band per doubling
    // above the channel threshold.
    let order: Vec<u8> = drainage
        .iter()
        .map(|&d| {
            if d < CHANNEL_THRESHOLD_CELLS {
                0
            } else {
                let bands = (d / CHANNEL_THRESHOLD_CELLS).ilog2();
                #[allow(clippy::cast_possible_truncation)]
                {
                    (bands as u8).saturating_add(1).min(9)
                }
            }
        })
        .collect();

    WaterGrid {
        drainage,
        order,
        downstream,
    }
}
```

Create `crates/arda-gen/src/area/mod.rs` (prepend above the test module):

```rust
//! Area generation (`logic/02`).
//!
//! Skeleton scope: relief and water. Climate, vegetation, settlement,
//! land-use, and roads arrive at build-order step 5, each as another pure
//! stage taking prior outputs.

pub mod relief;
pub mod water;

use crate::continent::bundles::TileBundle;
use crate::continent::ContinentGrid;
use arda_core::{
    AreaCells, AreaObjects, Cell, CellCoord, Cover, DischargeMilli, HeightMm, RiverSegment,
    TerrainKind, AREA_CELLS,
};
pub use relief::{relief, ReliefGrid};
pub use water::{water, WaterGrid, CHANNEL_THRESHOLD_CELLS};

/// Millimetres of discharge per upstream cell — a placeholder rating curve
/// until build-order step 5 brings the real one from rainfall.
///
/// `ponytail:` linear rating curve; replace with the rainfall-driven
/// relationship once the climate stage exists.
const DISCHARGE_PER_CELL_MILLI: u32 = 90;

/// Turns relief and water into the stored cell grid and object lists.
#[must_use]
pub fn compose(relief: &ReliefGrid, water: &WaterGrid) -> (AreaCells, AreaObjects) {
    let mut cells = AreaCells::flat(Cell::default());
    let n = AREA_CELLS;

    for y in 0..n {
        for x in 0..n {
            let Some(at) = CellCoord::new(x, y) else {
                continue;
            };
            let h = relief.get(at);
            let is_land = h > 0;
            let order = if is_land { water.order_at(at) } else { 0 };
            let drainage = water.drainage_at(at);

            cells.set(
                at,
                Cell {
                    height: HeightMm::new(h),
                    terrain: if is_land {
                        TerrainKind::Land
                    } else {
                        TerrainKind::Sea
                    },
                    cover: if is_land { Cover::Grass } else { Cover::Bare },
                    drainage_area_cells: drainage,
                    discharge: DischargeMilli::new(
                        drainage.saturating_mul(DISCHARGE_PER_CELL_MILLI),
                    ),
                    watercourse_order: order,
                    watercourse_width_dm: u16::from(order) * 12,
                    ..Cell::default()
                },
            );
        }
    }

    // One segment per channel cell run, walked downstream from each channel
    // head. Build-order step 5 merges these into named reaches.
    let mut rivers = Vec::new();
    let mut next_id = 1u16;
    for y in 0..n {
        for x in 0..n {
            let Some(at) = CellCoord::new(x, y) else {
                continue;
            };
            if cells.get(at).watercourse_order == 0 {
                continue;
            }
            // A head is a channel cell with no channel cell draining into it.
            let is_head = !has_channel_inflow(&cells, water, at);
            if !is_head {
                continue;
            }

            let mut course = vec![at];
            let mut cursor = at;
            while let Some(down) = water.downstream_of(cursor) {
                if cells.get(down).watercourse_order == 0 {
                    break;
                }
                course.push(down);
                cursor = down;
                if course.len() >= usize::from(AREA_CELLS) * 2 {
                    break; // Invariant guard: a course cannot exceed the tile.
                }
            }

            let tail = cells.get(cursor);
            rivers.push(RiverSegment {
                id: next_id,
                order: tail.watercourse_order,
                width_dm: tail.watercourse_width_dm,
                discharge: tail.discharge,
                course,
            });
            next_id = next_id.saturating_add(1);
        }
    }

    (cells, AreaObjects { rivers, lakes: Vec::new() })
}

fn has_channel_inflow(cells: &AreaCells, water: &WaterGrid, at: CellCoord) -> bool {
    let n = i32::from(AREA_CELLS);
    let (x, y) = (i32::from(at.x()), i32::from(at.y()));
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= n || ny >= n {
                continue;
            }
            #[allow(clippy::cast_possible_truncation)]
            let Some(nb) = CellCoord::new(nx as u16, ny as u16) else {
                continue;
            };
            if cells.get(nb).watercourse_order > 0 && water.downstream_of(nb) == Some(at) {
                return true;
            }
        }
    }
    false
}

/// Runs the skeleton area stage for one tile.
#[must_use]
pub fn generate_area(
    seed: u64,
    continent: &ContinentGrid,
    bundle: &TileBundle,
) -> (AreaCells, AreaObjects) {
    let r = relief(seed, continent, bundle);
    let w = water(&r);
    compose(&r, &w)
}
```

Add to `crates/arda-gen/src/lib.rs`:

```rust
pub mod area;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p arda-gen area`
Expected: PASS — 9 tests pass.

If `channels_appear_and_carry_strahler_order` fails with zero channels, lower `CHANNEL_THRESHOLD_CELLS` — at 512x512 a threshold of 240 cells puts channels on roughly the top few percent of accumulation. Do not remove the assertion.

- [ ] **Step 5: Commit**

```bash
git add crates/arda-gen/src
git commit -m "feat: add area relief and drainage stages"
```

---

### Task 13: Block stage — constrained WFC fill

`logic/03` §Q12: most-constrained-first fill, at most 8 subseeded retries on contradiction, then a relaxed fill that is **marked** rather than failed. The batch never dies on a block.

**Files:**
- Create: `crates/arda-gen/src/block/mod.rs`
- Create: `crates/arda-gen/src/block/constraints.rs`
- Create: `crates/arda-gen/src/block/wfc.rs`
- Modify: `crates/arda-gen/src/lib.rs`

**Interfaces:**
- Consumes: `AreaCells`/`Cell`/`TerrainKind` (Task 6), `Block`/`TileId`/`may_adjoin`/`SKELETON_TILES`/`TileGroup` (Task 8), `rng`/`SeedKey` (Task 3), `AreaCoord`/`CellCoord` (Task 2)
- Produces:
  - `struct BlockConstraints { allowed: Vec<TileId>, wet_fraction: u8 }`
  - `fn constraints_for(cells: &AreaCells, at: CellCoord) -> BlockConstraints` — from the cell and its 8 neighbours (`logic/03`)
  - `fn fill_block(seed: u64, area: AreaCoord, at: CellCoord, constraints: &BlockConstraints) -> Block`
  - `const MAX_ATTEMPTS: u8 = 8`

- [ ] **Step 1: Write the failing test**

Append to `crates/arda-gen/src/block/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::{
        may_adjoin, AreaCells, AreaCoord, Cell, CellCoord, HeightMm, SquareCoord, TerrainKind,
        BLOCK_SQUARES,
    };

    fn cc(x: u16, y: u16) -> CellCoord {
        CellCoord::new(x, y).unwrap()
    }

    fn land_grid() -> AreaCells {
        AreaCells::flat(Cell {
            height: HeightMm::new(220_000),
            terrain: TerrainKind::Land,
            ..Cell::default()
        })
    }

    fn sea_grid() -> AreaCells {
        AreaCells::flat(Cell {
            height: HeightMm::new(-40_000),
            terrain: TerrainKind::Sea,
            ..Cell::default()
        })
    }

    #[test]
    fn land_cells_allow_no_deep_water() {
        let c = constraints_for(&land_grid(), cc(20, 20));
        assert!(!c.allowed.is_empty());
        assert!(c.allowed.iter().all(|&t| t.raw() != 0));
    }

    #[test]
    fn sea_cells_allow_only_water_and_shore() {
        let c = constraints_for(&sea_grid(), cc(20, 20));
        assert!(c.allowed.iter().all(|&t| t.raw() <= 7));
    }

    #[test]
    fn a_coastal_cell_allows_both_families() {
        let mut grid = land_grid();
        grid.set(
            cc(21, 20),
            Cell {
                height: HeightMm::new(-9_000),
                terrain: TerrainKind::Sea,
                ..Cell::default()
            },
        );
        let c = constraints_for(&grid, cc(20, 20));
        assert!(c.wet_fraction > 0, "a neighbouring sea cell must register");
    }

    #[test]
    fn fill_is_deterministic() {
        let grid = land_grid();
        let c = constraints_for(&grid, cc(20, 20));
        let a = fill_block(42, AreaCoord::new(1, 1), cc(20, 20), &c);
        let b = fill_block(42, AreaCoord::new(1, 1), cc(20, 20), &c);
        assert_eq!(a, b);
    }

    #[test]
    fn different_cells_get_different_blocks() {
        let grid = land_grid();
        let c = constraints_for(&grid, cc(20, 20));
        let a = fill_block(42, AreaCoord::new(1, 1), cc(20, 20), &c);
        let b = fill_block(42, AreaCoord::new(1, 1), cc(21, 20), &c);
        assert_ne!(a, b);
    }

    #[test]
    fn every_square_is_filled_with_an_allowed_tile() {
        let grid = land_grid();
        let c = constraints_for(&grid, cc(20, 20));
        let block = fill_block(42, AreaCoord::new(0, 0), cc(20, 20), &c);
        for y in 0..BLOCK_SQUARES {
            for x in 0..BLOCK_SQUARES {
                let t = block.square(SquareCoord::new(x, y).unwrap());
                assert!(c.allowed.contains(&t), "square {x},{y} holds a banned tile");
            }
        }
    }

    /// The convergence property the whole retry ladder exists to protect
    /// (`logic/03` §Q12). A non-relaxed block must satisfy adjacency
    /// everywhere.
    #[test]
    fn a_converged_block_satisfies_adjacency_everywhere() {
        let grid = land_grid();
        let c = constraints_for(&grid, cc(20, 20));
        let block = fill_block(42, AreaCoord::new(0, 0), cc(20, 20), &c);
        if block.is_relaxed() {
            return; // Relaxed fills are allowed to violate; that is the point.
        }
        for y in 0..BLOCK_SQUARES {
            for x in 0..BLOCK_SQUARES {
                let here = block.square(SquareCoord::new(x, y).unwrap());
                if x + 1 < BLOCK_SQUARES {
                    let right = block.square(SquareCoord::new(x + 1, y).unwrap());
                    assert!(may_adjoin(here, right), "bad horizontal seam at {x},{y}");
                }
                if y + 1 < BLOCK_SQUARES {
                    let down = block.square(SquareCoord::new(x, y + 1).unwrap());
                    assert!(may_adjoin(here, down), "bad vertical seam at {x},{y}");
                }
            }
        }
    }

    #[test]
    fn an_impossible_constraint_set_falls_back_to_a_marked_relaxed_fill() {
        // logic/03 §Q12: never fail the batch. Deep water plus rock cannot
        // tile together, so this must relax rather than panic or loop.
        let c = BlockConstraints {
            allowed: vec![
                arda_core::TileId::new(0),
                arda_core::TileId::new(21),
                arda_core::TileId::new(22),
            ],
            wet_fraction: 128,
        };
        let block = fill_block(42, AreaCoord::new(0, 0), cc(5, 5), &c);
        assert!(block.is_relaxed());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p arda-gen block`
Expected: FAIL — `cannot find function 'constraints_for' in this scope`.

- [ ] **Step 3: Write minimal implementation**

Create `crates/arda-gen/src/block/constraints.rs`:

```rust
//! Cell-and-neighbours to WFC constraints (`logic/03`).

use arda_core::{
    tiles_in_group, AreaCells, CellCoord, TerrainKind, TileGroup, TileId, AREA_CELLS,
};

/// What a block may be built from, derived from its cell and the eight
/// neighbours around it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockConstraints {
    /// Tiles permitted in this block.
    pub allowed: Vec<TileId>,
    /// How wet the surroundings are, 0-255; drives the water-to-land mix.
    pub wet_fraction: u8,
}

/// Builds the constraint set for one cell.
#[must_use]
pub fn constraints_for(cells: &AreaCells, at: CellCoord) -> BlockConstraints {
    let here = cells.get(at);
    let n = i32::from(AREA_CELLS);
    let (cx, cy) = (i32::from(at.x()), i32::from(at.y()));

    let mut wet_neighbours = 0u32;
    let mut counted = 0u32;
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let (nx, ny) = (cx + dx, cy + dy);
            if nx < 0 || ny < 0 || nx >= n || ny >= n {
                continue;
            }
            #[allow(clippy::cast_possible_truncation)]
            let Some(nb) = CellCoord::new(nx as u16, ny as u16) else {
                continue;
            };
            counted += 1;
            if cells.get(nb).terrain != TerrainKind::Land {
                wet_neighbours += 1;
            }
        }
    }
    #[allow(clippy::cast_possible_truncation)]
    let wet_fraction = if counted == 0 {
        0u8
    } else {
        (wet_neighbours * 255 / counted) as u8
    };

    let groups: &[TileGroup] = match here.terrain {
        // Open water: water and shore only, so a sea block cannot grow trees.
        TerrainKind::Sea | TerrainKind::Lake => &[TileGroup::Water, TileGroup::Shore],
        TerrainKind::Land if here.watercourse_order > 0 => {
            &[TileGroup::Shore, TileGroup::Ground, TileGroup::Vegetation]
        }
        TerrainKind::Land if wet_fraction > 0 => &[
            TileGroup::Shore,
            TileGroup::Ground,
            TileGroup::Vegetation,
            TileGroup::Rock,
        ],
        TerrainKind::Land => &[
            TileGroup::Ground,
            TileGroup::Vegetation,
            TileGroup::Rock,
            TileGroup::Structure,
        ],
    };

    let mut allowed: Vec<TileId> = groups
        .iter()
        .flat_map(|&g| tiles_in_group(g))
        .map(|t| t.id)
        .collect();
    allowed.sort_unstable();
    allowed.dedup();

    BlockConstraints {
        allowed,
        wet_fraction,
    }
}
```

Create `crates/arda-gen/src/block/wfc.rs`:

```rust
//! Most-constrained-first wave-function collapse (`logic/03` §Q12).
//!
//! At most [`MAX_ATTEMPTS`] subseeded attempts; on exhaustion the block is
//! filled with the single most permissive tile and marked relaxed, so a hard
//! cell can never fail the batch.

use super::constraints::BlockConstraints;
use arda_core::{
    may_adjoin, AreaCoord, Block, CellCoord, SeedKey, SquareCoord, Stage, Tier, TileId,
    BLOCK_SQUARES,
};
use rand_core::RngCore;

/// Attempts before the relaxed fallback.
pub const MAX_ATTEMPTS: u8 = 8;

/// Fills one block.
#[must_use]
pub fn fill_block(
    seed: u64,
    area: AreaCoord,
    at: CellCoord,
    constraints: &BlockConstraints,
) -> Block {
    // The stream is keyed by the block's own position, so a block's content
    // never depends on which blocks were generated before it (§Q4).
    let key_x = area.x * i32::from(arda_core::AREA_CELLS) + i32::from(at.x());
    let key_y = area.y * i32::from(arda_core::AREA_CELLS) + i32::from(at.y());

    for attempt in 0..MAX_ATTEMPTS {
        let key = SeedKey::new(Tier::Block, Stage::Blocks, key_x, key_y, attempt);
        if let Some(block) = try_fill(seed, key, constraints) {
            return block;
        }
    }

    // Relaxed fallback: the first allowed tile everywhere. Marked so export
    // and the block report can surface it (`logic/03` §Q12).
    let fill = constraints
        .allowed
        .first()
        .copied()
        .unwrap_or(TileId::new(0));
    let mut block = Block::filled(fill);
    block.mark_relaxed();
    block
}

/// One collapse attempt. `None` means a contradiction was reached.
fn try_fill(seed: u64, key: SeedKey, constraints: &BlockConstraints) -> Option<Block> {
    let n = i32::from(BLOCK_SQUARES);
    let count = (n * n) as usize;
    let mut r = arda_core::rng(seed, key);

    // `None` = undecided.
    let mut grid: Vec<Option<TileId>> = vec![None; count];

    for _ in 0..count {
        // Most-constrained-first: fewest remaining options, ties broken by
        // the lowest index so the choice is order-independent.
        let mut best: Option<(usize, Vec<TileId>)> = None;
        for idx in 0..count {
            if grid[idx].is_some() {
                continue;
            }
            let options = options_at(&grid, constraints, idx, n);
            if options.is_empty() {
                return None; // Contradiction.
            }
            let better = match &best {
                None => true,
                Some((_, current)) => options.len() < current.len(),
            };
            if better {
                best = Some((idx, options));
            }
        }

        let Some((idx, options)) = best else {
            break; // Everything is decided.
        };
        let pick = options[(r.next_u32() as usize) % options.len()];
        grid[idx] = Some(pick);
    }

    let mut block = Block::filled(constraints.allowed.first().copied()?);
    for (idx, tile) in grid.iter().enumerate() {
        let tile = (*tile)?;
        #[allow(clippy::cast_possible_truncation)]
        let x = (idx % n as usize) as u8;
        #[allow(clippy::cast_possible_truncation)]
        let y = (idx / n as usize) as u8;
        block.set(SquareCoord::new(x, y)?, tile);
    }
    Some(block)
}

/// Tiles still legal at one square, given its decided neighbours.
fn options_at(
    grid: &[Option<TileId>],
    constraints: &BlockConstraints,
    idx: usize,
    n: i32,
) -> Vec<TileId> {
    #[allow(clippy::cast_possible_truncation)]
    let x = (idx % n as usize) as i32;
    #[allow(clippy::cast_possible_truncation)]
    let y = (idx / n as usize) as i32;

    constraints
        .allowed
        .iter()
        .copied()
        .filter(|&candidate| {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= n || ny >= n {
                    continue;
                }
                let nidx = (ny * n + nx) as usize;
                if let Some(neighbour) = grid[nidx] {
                    if !may_adjoin(candidate, neighbour) {
                        return false;
                    }
                }
            }
            true
        })
        .collect()
}
```

Create `crates/arda-gen/src/block/mod.rs` (prepend above the test module):

```rust
//! Block generation (`logic/03`).
//!
//! Skeleton scope: the 24-tile vocabulary from `arda-core::tiles`. Build-order
//! step 6 grows this to 200+ tiles after spike S2, without changing these
//! signatures.

pub mod constraints;
pub mod wfc;

pub use constraints::{constraints_for, BlockConstraints};
pub use wfc::{fill_block, MAX_ATTEMPTS};
```

Add to `crates/arda-gen/src/lib.rs`:

```rust
pub mod block;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p arda-gen block`
Expected: PASS — 8 tests pass.

> **Performance note.** `try_fill` rescans all 4096 squares to find the most-constrained one, which is O(n²) per block. That is fine for the skeleton's handful of blocks and deliberately simple. Mark it: `// ponytail: O(n^2) most-constrained scan; swap for an entropy heap when block counts reach the full 262,144 per area.` Build-order step 6 is where that matters.

- [ ] **Step 5: Commit**

```bash
git add crates/arda-gen/src
git commit -m "feat: add constrained WFC block fill with relaxed fallback"
```

---

### Task 14: Orchestrator, world writing, and the load facade

Wires the stages, writes the world directory in `mockup/02`'s layout, and gives `arda` its public `World` view. The manifest is written **last** — that is the completion stamp that makes partial worlds unloadable.

**Files:**
- Create: `crates/arda-gen/src/orchestrator.rs`
- Modify: `crates/arda-gen/src/lib.rs`
- Rewrite: `crates/arda/src/lib.rs`

**Interfaces:**
- Consumes: everything from Tasks 5–13
- Produces:
  - `fn generate_world(seed: u64, config: GenerateConfig, out: &Path) -> Result<Manifest, GenError>`
  - `enum GenError { OutputNotEmpty { dir }, Write { .. }, Validation { check } }`
  - `arda::World` with `load(dir) -> Result<World, LoadError>`, `seed()`, `size_km()`, `areas()`, `area(x, y) -> Result<&Area, LoadError>`, `block(ax, ay, cx, cy) -> Result<&Block, LoadError>`
  - `arda::Area` with `cell(x, y) -> Result<&Cell, LoadError>`, `rivers()`, `lakes()`
  - `arda::generate(seed, config, out) -> Result<Manifest, GenError>`

- [ ] **Step 1: Write the failing test**

Create `crates/arda/tests/round_trip.rs`:

```rust
//! End-to-end: generate a micro-continent, load it back, read it
//! (`logic/05`, `mockup/04`).

use arda::{generate, World};
use arda_core::GenerateConfig;

struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("arda-rt-{}-{tag}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn generate_then_load_round_trips() {
    let dir = TempDir::new("round-trip");
    let manifest = generate(42, GenerateConfig::MICRO, dir.path()).unwrap();
    assert_eq!(manifest.seed, 42);
    assert_eq!(manifest.stats.area_count, 8);

    let world = World::load(dir.path()).unwrap();
    assert_eq!(world.seed(), 42);
    assert_eq!(world.areas(), 8);
    assert_eq!(world.size_km().width, 102);
}

#[test]
fn world_directory_matches_the_mockup_layout() {
    // mockup/02: world.json, continent/, areas/<ax>_<ay>/, blocks/.
    let dir = TempDir::new("layout");
    generate(42, GenerateConfig::MICRO, dir.path()).unwrap();

    assert!(dir.path().join("world.json").is_file());
    assert!(dir.path().join("areas").is_dir());
    assert!(dir.path().join("areas/00_00/cells.bin").is_file());
    assert!(dir.path().join("areas/00_00/objects.bin").is_file());
    assert!(dir.path().join("areas/01_03/cells.bin").is_file());
    assert!(dir.path().join("blocks/00_00.tiles.zst").is_file());
}

#[test]
fn cells_survive_the_disk_round_trip() {
    let dir = TempDir::new("cells");
    generate(42, GenerateConfig::MICRO, dir.path()).unwrap();
    let world = World::load(dir.path()).unwrap();

    let area = world.area(1, 1).unwrap();
    let cell = area.cell(300, 128).unwrap();
    // The field list from mockup/04 is readable.
    let _ = cell.height;
    let _ = cell.terrain;
    let _ = cell.watercourse_order;
    assert!(area.rivers().iter().all(|r| r.order >= 1));
}

#[test]
fn out_of_range_area_returns_a_range_error_carrying_the_bounds() {
    let dir = TempDir::new("range");
    generate(42, GenerateConfig::MICRO, dir.path()).unwrap();
    let world = World::load(dir.path()).unwrap();

    let err = world.area(9, 9).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("outside the world"), "message was: {msg}");
}

#[test]
fn a_partial_world_is_refused() {
    // 04-data-flow.md: manifest absent means loaders refuse the directory.
    let dir = TempDir::new("partial");
    generate(42, GenerateConfig::MICRO, dir.path()).unwrap();
    std::fs::remove_file(dir.path().join("world.json")).unwrap();

    let err = World::load(dir.path()).unwrap_err();
    assert!(err.to_string().contains("did not finish"));
}

#[test]
fn generating_into_a_non_empty_directory_is_refused() {
    // mockup/01 States: refuse rather than overwrite.
    let dir = TempDir::new("occupied");
    generate(42, GenerateConfig::MICRO, dir.path()).unwrap();
    let err = generate(42, GenerateConfig::MICRO, dir.path()).unwrap_err();
    assert!(err.to_string().contains("not empty"));
}

#[test]
fn the_same_seed_produces_byte_identical_worlds() {
    // architecture-interview.md §Q4, the one-way door.
    let a = TempDir::new("det-a");
    let b = TempDir::new("det-b");
    generate(42, GenerateConfig::MICRO, a.path()).unwrap();
    generate(42, GenerateConfig::MICRO, b.path()).unwrap();

    for rel in [
        "world.json",
        "areas/00_00/cells.bin",
        "areas/01_03/objects.bin",
        "blocks/00_00.tiles.zst",
    ] {
        assert_eq!(
            std::fs::read(a.path().join(rel)).unwrap(),
            std::fs::read(b.path().join(rel)).unwrap(),
            "{rel} differs between runs"
        );
    }
}

#[test]
fn different_seeds_produce_different_worlds() {
    let a = TempDir::new("seed-42");
    let b = TempDir::new("seed-43");
    generate(42, GenerateConfig::MICRO, a.path()).unwrap();
    generate(43, GenerateConfig::MICRO, b.path()).unwrap();
    assert_ne!(
        std::fs::read(a.path().join("areas/00_00/cells.bin")).unwrap(),
        std::fs::read(b.path().join("areas/00_00/cells.bin")).unwrap()
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p arda --test round_trip`
Expected: FAIL — `cannot find function 'generate' in crate 'arda'`.

- [ ] **Step 3: Write minimal implementation**

Create `crates/arda-gen/src/orchestrator.rs`:

```rust
//! The batch orchestrator (`04-data-flow.md` lifecycle 1).
//!
//! Stages run sequentially per tier; areas fan out on the rayon pool. The
//! orchestrator is the only thing here that touches disk — the stages
//! themselves are pure.

use crate::area::generate_area;
use crate::block::{constraints_for, fill_block};
use crate::continent::bundles::bundle_for;
use crate::continent::generate_continent;
use arda_core::{
    encode_blocks, encode_cells, encode_objects, write_manifest, AreaCoord, BlockArchive, CellCoord,
    GenerateConfig, Manifest, TerrainKind, ValidationStats, AREA_CELLS, FORMAT_VERSION,
};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// A batch failure (`mockup/01` States).
#[derive(Debug, Error)]
pub enum GenError {
    /// The output directory already holds something.
    #[error("output directory {dir} is not empty; refusing to overwrite a world")]
    OutputNotEmpty {
        /// The directory that was targeted.
        dir: String,
    },
    /// A layer could not be written.
    #[error("failed writing {path}: {source}")]
    Write {
        /// The file being written.
        path: String,
        /// Underlying cause.
        #[source]
        source: std::io::Error,
    },
    /// A continent validation gate failed (`logic/01` step 9).
    #[error("continent validation failed: {check}")]
    Validation {
        /// The gate that rejected the continent.
        check: String,
    },
    /// The manifest could not be stamped.
    #[error("failed stamping the manifest: {0}")]
    Manifest(#[from] arda_core::LoadError),
}

/// Blocks materialised per area in the skeleton.
///
/// `ponytail:` sampled blocks; the full batch materialises one per land cell
/// (`mockup/02`). Build-order step 6 removes the stride.
const SKELETON_BLOCK_STRIDE: u16 = 64;

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), GenError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| GenError::Write {
            path: path.display().to_string(),
            source: e,
        })?;
    }
    std::fs::write(path, bytes).map_err(|e| GenError::Write {
        path: path.display().to_string(),
        source: e,
    })
}

/// Runs the whole batch and stamps the manifest.
///
/// # Errors
/// See [`GenError`].
pub fn generate_world(
    seed: u64,
    config: GenerateConfig,
    out: &Path,
) -> Result<Manifest, GenError> {
    if out.exists() && out.read_dir().map(|mut d| d.next().is_some()).unwrap_or(false) {
        return Err(GenError::OutputNotEmpty {
            dir: out.display().to_string(),
        });
    }

    // Tier 1: continent, single-threaded.
    let continent = generate_continent(seed, config);
    let land = continent.land_fraction_permille();
    if !(250..=900).contains(&land) {
        return Err(GenError::Validation {
            check: format!("land fraction {land} per mille outside 250..=900"),
        });
    }

    // Tier 2 and 3: areas fan out, each writing its own keyed outputs. The
    // work is order-free because every tile depends only on its bundle.
    let coords: Vec<AreaCoord> = config.area_coords().collect();
    let results: Vec<Result<(), GenError>> = coords
        .par_iter()
        .map(|&area| {
            let bundle = bundle_for(seed, &continent, area);
            let (cells, objects) = generate_area(seed, &continent, &bundle);

            let dir: PathBuf = out.join("areas").join(area.dir_name());
            write_file(&dir.join("cells.bin"), &encode_cells(&cells))?;
            write_file(&dir.join("objects.bin"), &encode_objects(&objects))?;

            let mut archive = BlockArchive::default();
            let mut y = 0u16;
            while y < AREA_CELLS {
                let mut x = 0u16;
                while x < AREA_CELLS {
                    if let Some(at) = CellCoord::new(x, y) {
                        if cells.get(at).terrain == TerrainKind::Land {
                            let c = constraints_for(&cells, at);
                            archive.insert(at, fill_block(seed, area, at, &c));
                        }
                    }
                    x += SKELETON_BLOCK_STRIDE;
                }
                y += SKELETON_BLOCK_STRIDE;
            }

            let blocks = encode_blocks(&archive).map_err(|e| GenError::Write {
                path: format!("blocks/{}.tiles.zst", area.dir_name()),
                source: std::io::Error::other(e.to_string()),
            })?;
            write_file(
                &out.join("blocks").join(format!("{}.tiles.zst", area.dir_name())),
                &blocks,
            )?;
            Ok(())
        })
        .collect();

    for r in results {
        r?;
    }

    // Continent layer, then the manifest LAST — the completion stamp.
    write_file(&out.join("continent").join("overview.bin"), &[])?;

    let manifest = Manifest {
        format_version: FORMAT_VERSION,
        arda_version: env!("CARGO_PKG_VERSION").to_owned(),
        seed,
        config,
        areas_wide: config.areas_wide(),
        areas_high: config.areas_high(),
        stats: ValidationStats {
            land_fraction_permille: land,
            #[allow(clippy::cast_possible_truncation)]
            area_count: coords.len() as u32,
            settlement_count: 0,
            named_river_count: 0,
        },
    };
    write_manifest(out, &manifest)?;
    Ok(manifest)
}
```

Add to `crates/arda-gen/src/lib.rs`:

```rust
pub mod orchestrator;
pub use orchestrator::{generate_world, GenError};
```

Rewrite `crates/arda/src/lib.rs`:

```rust
//! The public facade for arda. Consumers depend on this crate only.
//!
//! Mirrors `mockup/04`'s transcript: `World` (manifest) to `Area` (cells and
//! objects) to `Cell` / `Block`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub use arda_core::{
    AreaCells, AreaObjects, Block, Cell, Cover, GenerateConfig, LatitudeBand, LoadError, Manifest,
    RiverSegment, RoadClass, SizeKm, TerrainKind, TileId, ValidationStats,
};
pub use arda_gen::GenError;

/// Runs the batch, writing a world directory (`mockup/01`).
///
/// # Errors
/// See [`GenError`].
pub fn generate(seed: u64, config: GenerateConfig, out: &Path) -> Result<Manifest, GenError> {
    arda_gen::generate_world(seed, config, out)
}

/// One loaded area tile.
pub struct Area {
    cells: AreaCells,
    objects: AreaObjects,
}

impl Area {
    /// Reads one cell.
    ///
    /// # Errors
    /// [`LoadError::OutOfRange`] when the coordinates leave the tile.
    pub fn cell(&self, x: u16, y: u16) -> Result<&Cell, LoadError> {
        let at = arda_core::CellCoord::new(x, y).ok_or(LoadError::OutOfRange {
            what: "cell",
            x: i32::from(x),
            y: i32::from(y),
            max_x: i32::from(arda_core::AREA_CELLS) - 1,
            max_y: i32::from(arda_core::AREA_CELLS) - 1,
        })?;
        Ok(self.cells.get(at))
    }

    /// River segments in this tile.
    #[must_use]
    pub fn rivers(&self) -> &[RiverSegment] {
        &self.objects.rivers
    }

    /// Lakes in this tile.
    #[must_use]
    pub fn lakes(&self) -> &[arda_core::Lake] {
        &self.objects.lakes
    }
}

/// A loaded world (`logic/05`).
///
/// `ponytail:` areas and blocks are read eagerly at `load`. `logic/05` wants
/// them lazy with an O(accessed) cache; the upgrade is to hold the directory
/// path and populate these maps on first access behind a `OnceLock`. The
/// skeleton's eight tiles fit in memory, so laziness is not yet earned.
pub struct World {
    dir: PathBuf,
    manifest: Manifest,
    areas: BTreeMap<(i32, i32), Area>,
    blocks: BTreeMap<(i32, i32), arda_core::BlockArchive>,
}

impl World {
    /// Opens a generated world.
    ///
    /// # Errors
    /// [`LoadError::ManifestMissing`] for a partial world, [`LoadError::VersionSkew`]
    /// for an incompatible format, [`LoadError::Corrupt`] for a bad layer.
    pub fn load(dir: &Path) -> Result<Self, LoadError> {
        let manifest = arda_core::formats::manifest::read_manifest(dir)?;

        let mut areas = BTreeMap::new();
        let mut blocks = BTreeMap::new();
        for ay in 0..manifest.areas_high {
            for ax in 0..manifest.areas_wide {
                let coord = arda_core::AreaCoord::new(ax, ay);
                let name = coord.dir_name();

                let cells_path = dir.join("areas").join(&name).join("cells.bin");
                let cells_bytes = read(&cells_path)?;
                let cells = arda_core::decode_cells(&cells_path.display().to_string(), &cells_bytes)?;

                let obj_path = dir.join("areas").join(&name).join("objects.bin");
                let obj_bytes = read(&obj_path)?;
                let objects = arda_core::decode_objects(&obj_path.display().to_string(), &obj_bytes)?;

                areas.insert((ax, ay), Area { cells, objects });

                let blk_path = dir.join("blocks").join(format!("{name}.tiles.zst"));
                let blk_bytes = read(&blk_path)?;
                blocks.insert(
                    (ax, ay),
                    arda_core::decode_blocks(&blk_path.display().to_string(), &blk_bytes)?,
                );
            }
        }

        Ok(Self {
            dir: dir.to_path_buf(),
            manifest,
            areas,
            blocks,
        })
    }

    /// The seed this world was generated from.
    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.manifest.seed
    }

    /// Continent extent.
    #[must_use]
    pub const fn size_km(&self) -> SizeKm {
        self.manifest.config.size_km()
    }

    /// Number of area tiles.
    #[must_use]
    pub fn areas(&self) -> usize {
        self.areas.len()
    }

    /// The directory this world was loaded from.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The manifest.
    #[must_use]
    pub const fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Reads one area tile.
    ///
    /// # Errors
    /// [`LoadError::OutOfRange`] when the tile is outside the continent.
    pub fn area(&self, x: i32, y: i32) -> Result<&Area, LoadError> {
        self.areas.get(&(x, y)).ok_or(LoadError::OutOfRange {
            what: "area",
            x,
            y,
            max_x: self.manifest.areas_wide - 1,
            max_y: self.manifest.areas_high - 1,
        })
    }

    /// Reads one block.
    ///
    /// # Errors
    /// [`LoadError::OutOfRange`] when the tile or cell is outside the world,
    /// or no block was materialised for that cell.
    pub fn block(&self, ax: i32, ay: i32, cx: u16, cy: u16) -> Result<&Block, LoadError> {
        let out_of_range = || LoadError::OutOfRange {
            what: "block",
            x: i32::from(cx),
            y: i32::from(cy),
            max_x: i32::from(arda_core::AREA_CELLS) - 1,
            max_y: i32::from(arda_core::AREA_CELLS) - 1,
        };
        let archive = self.blocks.get(&(ax, ay)).ok_or(LoadError::OutOfRange {
            what: "area",
            x: ax,
            y: ay,
            max_x: self.manifest.areas_wide - 1,
            max_y: self.manifest.areas_high - 1,
        })?;
        let at = arda_core::CellCoord::new(cx, cy).ok_or_else(out_of_range)?;
        archive.get(at).ok_or_else(out_of_range)
    }
}

fn read(path: &Path) -> Result<Vec<u8>, LoadError> {
    std::fs::read(path).map_err(|e| LoadError::ManifestUnreadable {
        dir: path.display().to_string(),
        reason: e.to_string(),
    })
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p arda --test round_trip`
Expected: PASS — 8 tests pass.

Run: `cargo test --workspace`
Expected: PASS — every crate green.

- [ ] **Step 5: Commit**

```bash
git add crates/arda-gen/src crates/arda/src crates/arda/tests
git commit -m "feat: add batch orchestrator and world load facade"
```

---

### Task 15: Export — symbolic block PNG, area map PNG, versioned JSON

`arda-render` reads stored worlds only (`01-architecture.md` §Q2) — it must not gain an `arda-gen` dependency. Export is byte-identical on repeat (`logic/04`).

**Files:**
- Create: `crates/arda-render/src/symbolic.rs`
- Create: `crates/arda-render/src/carto.rs`
- Create: `crates/arda-render/src/json.rs`
- Rewrite: `crates/arda-render/src/lib.rs`

**Interfaces:**
- Consumes: `Block`/`TileId`/`tile_def`/`SKELETON_TILES` (Task 8), `AreaCells`/`Cell`/`TerrainKind` (Task 6), `Manifest` (Task 5), `AreaObjects` (Task 7)
- Produces:
  - `enum RenderError { Png, UnmappedTile { id }, PartialWorld }`
  - `fn render_block_png(block: &Block) -> Result<Vec<u8>, RenderError>` — 64x64 squares at 8 px, 512x512 output
  - `fn render_area_png(cells: &AreaCells) -> Result<Vec<u8>, RenderError>` — 512x512, one pixel per cell, hypsometric tint
  - `const SCHEMA_VERSION: u32 = 1`
  - `fn area_json(manifest: &Manifest, ax: i32, ay: i32, cells: &AreaCells, objects: &AreaObjects) -> String`
  - `fn block_json(manifest: &Manifest, block: &Block) -> String` — carries the tile legend

- [ ] **Step 1: Write the failing test**

Append to `crates/arda-render/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::{
        AreaCells, Block, Cell, GenerateConfig, HeightMm, Manifest, SquareCoord, TerrainKind,
        TileId, ValidationStats, FORMAT_VERSION,
    };

    fn manifest() -> Manifest {
        Manifest {
            format_version: FORMAT_VERSION,
            arda_version: "0.1.0".to_owned(),
            seed: 42,
            config: GenerateConfig::MICRO,
            areas_wide: 2,
            areas_high: 4,
            stats: ValidationStats {
                land_fraction_permille: 600,
                area_count: 8,
                settlement_count: 0,
                named_river_count: 0,
            },
        }
    }

    fn block() -> Block {
        let mut b = Block::filled(TileId::new(9));
        b.set(SquareCoord::new(0, 0).unwrap(), TileId::new(1));
        b.set(SquareCoord::new(63, 63).unwrap(), TileId::new(21));
        b
    }

    fn cells() -> AreaCells {
        AreaCells::flat(Cell {
            height: HeightMm::new(180_000),
            terrain: TerrainKind::Land,
            ..Cell::default()
        })
    }

    #[test]
    fn block_png_has_a_png_signature() {
        let bytes = render_block_png(&block()).unwrap();
        assert_eq!(&bytes[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    }

    #[test]
    fn block_png_is_byte_identical_on_repeat() {
        // logic/04: re-export must produce the same bytes.
        assert_eq!(
            render_block_png(&block()).unwrap(),
            render_block_png(&block()).unwrap()
        );
    }

    #[test]
    fn area_png_is_byte_identical_on_repeat() {
        assert_eq!(
            render_area_png(&cells()).unwrap(),
            render_area_png(&cells()).unwrap()
        );
    }

    #[test]
    fn an_unmapped_tile_is_refused_naming_the_id() {
        // logic/04: unmapped-tile is a typed refusal, not a silent default.
        let mut b = Block::filled(TileId::new(9));
        b.set(SquareCoord::new(1, 1).unwrap(), TileId::new(60_000));
        let err = render_block_png(&b).unwrap_err();
        match err {
            RenderError::UnmappedTile { id } => assert_eq!(id, 60_000),
            other => panic!("expected UnmappedTile, got {other:?}"),
        }
    }

    #[test]
    fn area_json_carries_the_schema_version_and_snake_case_keys() {
        let json = area_json(&manifest(), 1, 1, &cells(), &arda_core::AreaObjects::empty());
        assert!(json.contains("\"schema_version\""));
        assert!(json.contains("\"area_x\""));
        assert!(json.contains("\"height_mm\""));
        assert!(!json.contains("areaX"), "schema must be snake_case (logic/04)");
    }

    #[test]
    fn block_json_carries_the_tile_legend() {
        let json = block_json(&manifest(), &block());
        assert!(json.contains("\"legend\""));
        assert!(json.contains("\"grass\""));
        assert!(json.contains("\"relaxed\""));
    }

    #[test]
    fn json_is_byte_identical_on_repeat() {
        let a = area_json(&manifest(), 1, 1, &cells(), &arda_core::AreaObjects::empty());
        let b = area_json(&manifest(), 1, 1, &cells(), &arda_core::AreaObjects::empty());
        assert_eq!(a, b);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p arda-render`
Expected: FAIL — `cannot find function 'render_block_png' in this scope`.

- [ ] **Step 3: Write minimal implementation**

Create `crates/arda-render/src/symbolic.rs`:

```rust
//! The built-in symbolic block style (`logic/04` §Q14).

use crate::RenderError;
use arda_core::{tile_def, Block, SquareCoord, TileGroup, BLOCK_SQUARES};

/// Pixels per five-foot square.
pub const SQUARE_PX: u32 = 8;

/// RGB for one tile, by family and id — distinct enough to read at a glance.
fn colour(id: u16) -> Option<[u8; 3]> {
    let def = tile_def(arda_core::TileId::new(id))?;
    // Within a family, later ids get progressively lighter.
    let step = u8::try_from(id % 6).unwrap_or(0) * 9;
    Some(match def.group {
        TileGroup::Water => [12, 42 + step, 96 + step],
        TileGroup::Shore => [176 + step, 158 + step, 108 + step],
        TileGroup::Ground => [72 + step, 108 + step, 54 + step],
        TileGroup::Vegetation => [34 + step, 78 + step, 38 + step],
        TileGroup::Rock => [116 + step, 114 + step, 110 + step],
        TileGroup::Structure => [140 + step, 116 + step, 92 + step],
    })
}

/// Renders one block at [`SQUARE_PX`] pixels per square.
///
/// # Errors
/// [`RenderError::UnmappedTile`] when a square holds a tile this build has no
/// entry for; [`RenderError::Png`] when encoding fails.
pub fn render_block_png(block: &Block) -> Result<Vec<u8>, RenderError> {
    let side = u32::from(BLOCK_SQUARES) * SQUARE_PX;
    let mut rgb = vec![0u8; (side * side * 3) as usize];

    for sy in 0..BLOCK_SQUARES {
        for sx in 0..BLOCK_SQUARES {
            let at = SquareCoord::new(sx, sy).ok_or(RenderError::Png)?;
            let id = block.square(at).raw();
            let c = colour(id).ok_or(RenderError::UnmappedTile { id })?;

            for py in 0..SQUARE_PX {
                for px in 0..SQUARE_PX {
                    let x = u32::from(sx) * SQUARE_PX + px;
                    let y = u32::from(sy) * SQUARE_PX + py;
                    let i = ((y * side + x) * 3) as usize;
                    rgb[i..i + 3].copy_from_slice(&c);
                }
            }
        }
    }
    crate::encode_png(side, side, &rgb)
}
```

Create `crates/arda-render/src/carto.rs`:

```rust
//! Cartographic area rendering (`logic/04`).

use crate::RenderError;
use arda_core::{AreaCells, CellCoord, TerrainKind, AREA_CELLS};

/// Renders one area tile, one pixel per 100 m cell, hypsometrically tinted.
///
/// # Errors
/// [`RenderError::Png`] when encoding fails.
pub fn render_area_png(cells: &AreaCells) -> Result<Vec<u8>, RenderError> {
    let side = u32::from(AREA_CELLS);
    let mut rgb = vec![0u8; (side * side * 3) as usize];

    for y in 0..AREA_CELLS {
        for x in 0..AREA_CELLS {
            let at = CellCoord::new(x, y).ok_or(RenderError::Png)?;
            let cell = cells.get(at);
            let colour = if cell.watercourse_order > 0 {
                [40, 92, 170]
            } else if cell.terrain == TerrainKind::Land {
                // 0 m to 2000 m mapped across a green-to-white ramp.
                let t = (cell.height.raw() / 8_000).clamp(0, 255);
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let t = t as u8;
                [64u8.saturating_add(t), 120u8.saturating_add(t / 2), 60u8.saturating_add(t)]
            } else {
                let depth = (-cell.height.raw() / 20_000).clamp(0, 90);
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let d = depth as u8;
                [10, 40u8.saturating_sub(d / 4), 110u8.saturating_sub(d)]
            };
            let i = ((u32::from(y) * side + u32::from(x)) * 3) as usize;
            rgb[i..i + 3].copy_from_slice(&colour);
        }
    }
    crate::encode_png(side, side, &rgb)
}
```

Create `crates/arda-render/src/json.rs`:

```rust
//! The versioned JSON export schema (`logic/04` §Q14).
//!
//! snake_case SI keys, additive-only. Field order is fixed by the struct
//! definitions, so repeated exports are byte-identical.

use arda_core::{AreaCells, AreaObjects, Block, CellCoord, Manifest, SquareCoord, AREA_CELLS,
    BLOCK_SQUARES, SKELETON_TILES};
use serde::Serialize;

/// Export schema version, additive-only (`logic/04` §Q14).
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Serialize)]
struct CellOut {
    height_mm: i32,
    terrain: &'static str,
    cover: &'static str,
    drainage_area_cells: u32,
    discharge_milli_cumecs: u32,
    watercourse_order: u8,
}

#[derive(Serialize)]
struct RiverOut {
    id: u16,
    order: u8,
    width_dm: u16,
    discharge_milli_cumecs: u32,
    course: Vec<[u16; 2]>,
}

#[derive(Serialize)]
struct AreaOut {
    schema_version: u32,
    format_version: u32,
    seed: u64,
    area_x: i32,
    area_y: i32,
    cells_per_side: u16,
    cells: Vec<CellOut>,
    rivers: Vec<RiverOut>,
}

#[derive(Serialize)]
struct LegendEntry {
    id: u16,
    name: &'static str,
}

#[derive(Serialize)]
struct BlockOut {
    schema_version: u32,
    format_version: u32,
    seed: u64,
    squares_per_side: u8,
    relaxed: bool,
    legend: Vec<LegendEntry>,
    squares: Vec<u16>,
}

const fn terrain_name(t: arda_core::TerrainKind) -> &'static str {
    match t {
        arda_core::TerrainKind::Sea => "sea",
        arda_core::TerrainKind::Land => "land",
        arda_core::TerrainKind::Lake => "lake",
    }
}

const fn cover_name(c: arda_core::Cover) -> &'static str {
    match c {
        arda_core::Cover::Bare => "bare",
        arda_core::Cover::Grass => "grass",
        arda_core::Cover::Scrub => "scrub",
        arda_core::Cover::Forest => "forest",
        arda_core::Cover::Marsh => "marsh",
        arda_core::Cover::Rock => "rock",
        arda_core::Cover::Ice => "ice",
    }
}

/// Serialises one area tile.
#[must_use]
pub fn area_json(
    manifest: &Manifest,
    ax: i32,
    ay: i32,
    cells: &AreaCells,
    objects: &AreaObjects,
) -> String {
    let mut out_cells = Vec::with_capacity(usize::from(AREA_CELLS) * usize::from(AREA_CELLS));
    for y in 0..AREA_CELLS {
        for x in 0..AREA_CELLS {
            let Some(at) = CellCoord::new(x, y) else {
                continue;
            };
            let c = cells.get(at);
            out_cells.push(CellOut {
                height_mm: c.height.raw(),
                terrain: terrain_name(c.terrain),
                cover: cover_name(c.cover),
                drainage_area_cells: c.drainage_area_cells,
                discharge_milli_cumecs: c.discharge.raw(),
                watercourse_order: c.watercourse_order,
            });
        }
    }

    let rivers = objects
        .rivers
        .iter()
        .map(|r| RiverOut {
            id: r.id,
            order: r.order,
            width_dm: r.width_dm,
            discharge_milli_cumecs: r.discharge.raw(),
            course: r.course.iter().map(|c| [c.x(), c.y()]).collect(),
        })
        .collect();

    let doc = AreaOut {
        schema_version: SCHEMA_VERSION,
        format_version: manifest.format_version,
        seed: manifest.seed,
        area_x: ax,
        area_y: ay,
        cells_per_side: AREA_CELLS,
        cells: out_cells,
        rivers,
    };
    serde_json::to_string_pretty(&doc).unwrap_or_default()
}

/// Serialises one block, legend included (`build-interview.md` §Q5 will grow
/// the legend with per-tile attributes at build-order step 8).
#[must_use]
pub fn block_json(manifest: &Manifest, block: &Block) -> String {
    let mut squares = Vec::with_capacity(usize::from(BLOCK_SQUARES) * usize::from(BLOCK_SQUARES));
    for y in 0..BLOCK_SQUARES {
        for x in 0..BLOCK_SQUARES {
            let Some(at) = SquareCoord::new(x, y) else {
                continue;
            };
            squares.push(block.square(at).raw());
        }
    }

    let doc = BlockOut {
        schema_version: SCHEMA_VERSION,
        format_version: manifest.format_version,
        seed: manifest.seed,
        squares_per_side: BLOCK_SQUARES,
        relaxed: block.is_relaxed(),
        legend: SKELETON_TILES
            .iter()
            .map(|t| LegendEntry {
                id: t.id.raw(),
                name: t.name,
            })
            .collect(),
        squares,
    };
    serde_json::to_string_pretty(&doc).unwrap_or_default()
}
```

Rewrite `crates/arda-render/src/lib.rs` (keeping the test module appended in Step 1):

```rust
//! PNG and JSON export. Reads stored worlds only — never depends on `arda-gen`.

pub mod carto;
pub mod json;
pub mod symbolic;

pub use carto::render_area_png;
pub use json::{area_json, block_json, SCHEMA_VERSION};
pub use symbolic::{render_block_png, SQUARE_PX};

use thiserror::Error;

/// An export failure (`logic/04` refusals).
#[derive(Debug, Error)]
pub enum RenderError {
    /// The PNG encoder failed.
    #[error("png encoding failed")]
    Png,
    /// A square holds a tile id this build has no entry for.
    #[error("tile id {id} has no entry in the vocabulary")]
    UnmappedTile {
        /// The unmapped id.
        id: u16,
    },
    /// The world is missing its manifest.
    #[error("refusing to export a partial world")]
    PartialWorld,
}

/// Encodes an RGB buffer as a PNG.
///
/// Compression and filter are pinned so repeated exports are byte-identical
/// (`logic/04`).
pub(crate) fn encode_png(width: u32, height: u32, rgb: &[u8]) -> Result<Vec<u8>, RenderError> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Default);
        encoder.set_filter(png::FilterType::NoFilter);
        let mut writer = encoder.write_header().map_err(|_| RenderError::Png)?;
        writer.write_image_data(rgb).map_err(|_| RenderError::Png)?;
    }
    Ok(out)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p arda-render`
Expected: PASS — 7 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/arda-render/src
git commit -m "feat: add symbolic png carto png and versioned json export"
```

---

### Task 16: CLI — `generate` and `export`

Reproduces `mockup/01` and `mockup/03`'s transcripts. The CLI is a thin wrapper: parse, call the facade, map typed errors to non-zero exits (`01-architecture.md` Communication).

**Files:**
- Rewrite: `crates/arda-cli/src/main.rs`
- Modify: `crates/arda/src/lib.rs` (add `export_area`, `export_block`)
- Create: `crates/arda-cli/tests/cli.rs`

**Interfaces:**
- Consumes: `arda::{generate, World}` (Task 14), render functions (Task 15)
- Produces:
  - `arda generate --seed <u64> --out <dir> [--size <WxH>] [--micro]`
  - `arda export --world <dir> --area <ax>,<ay> --out <dir> [--format png|json]`
  - `arda::export_area(world, ax, ay, out, format) -> Result<PathBuf, ExportError>`

- [ ] **Step 1: Write the failing test**

Create `crates/arda-cli/tests/cli.rs`:

```rust
//! CLI surface tests (`mockup/01`, `mockup/03`).

use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_arda")
}

struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("arda-cli-{}-{tag}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        Self(p)
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn generate_writes_a_world_and_exits_zero() {
    let dir = TempDir::new("gen");
    let out = Command::new(bin())
        .args(["generate", "--seed", "42", "--micro", "--out"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(dir.path().join("world.json").is_file());

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("seed 42"), "stdout was: {stdout}");
    assert!(stdout.contains("done"), "stdout was: {stdout}");
}

#[test]
fn generate_into_an_occupied_directory_exits_non_zero() {
    let dir = TempDir::new("occupied");
    for _ in 0..1 {
        Command::new(bin())
            .args(["generate", "--seed", "42", "--micro", "--out"])
            .arg(dir.path())
            .output()
            .unwrap();
    }
    let out = Command::new(bin())
        .args(["generate", "--seed", "42", "--micro", "--out"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("not empty"));
}

#[test]
fn invalid_config_exits_non_zero_naming_the_field() {
    let dir = TempDir::new("badcfg");
    let out = Command::new(bin())
        .args(["generate", "--seed", "1", "--size", "10x10", "--out"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("size"));
}

#[test]
fn export_writes_a_png_for_an_area() {
    let world = TempDir::new("exp-world");
    let out = TempDir::new("exp-out");
    Command::new(bin())
        .args(["generate", "--seed", "42", "--micro", "--out"])
        .arg(world.path())
        .output()
        .unwrap();
    std::fs::create_dir_all(out.path()).unwrap();

    let res = Command::new(bin())
        .args(["export", "--world"])
        .arg(world.path())
        .args(["--area", "1,1", "--format", "png", "--out"])
        .arg(out.path())
        .output()
        .unwrap();
    assert!(res.status.success(), "stderr: {}", String::from_utf8_lossy(&res.stderr));
    assert!(out.path().join("area_01_01.png").is_file());
}

#[test]
fn export_refuses_a_partial_world() {
    let world = TempDir::new("exp-partial");
    let out = TempDir::new("exp-partial-out");
    Command::new(bin())
        .args(["generate", "--seed", "42", "--micro", "--out"])
        .arg(world.path())
        .output()
        .unwrap();
    std::fs::remove_file(world.path().join("world.json")).unwrap();
    std::fs::create_dir_all(out.path()).unwrap();

    let res = Command::new(bin())
        .args(["export", "--world"])
        .arg(world.path())
        .args(["--area", "1,1", "--out"])
        .arg(out.path())
        .output()
        .unwrap();
    assert!(!res.status.success());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p arda-cli`
Expected: FAIL — the binary prints only its version, so `generate_writes_a_world_and_exits_zero` fails on the missing `world.json`.

- [ ] **Step 3: Write minimal implementation**

Add to `crates/arda/src/lib.rs`:

```rust
/// Output format for `export` (`mockup/03`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Cartographic PNG.
    Png,
    /// Versioned JSON.
    Json,
}

/// Exports one area tile to `out`, returning the file written.
///
/// # Errors
/// Propagates load and render failures.
pub fn export_area(
    world: &World,
    ax: i32,
    ay: i32,
    out: &Path,
    format: ExportFormat,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let area = world.area(ax, ay)?;
    let name = format!("area_{ax:02}_{ay:02}");
    let path = match format {
        ExportFormat::Png => {
            let bytes = arda_render::render_area_png(area.cells())?;
            let p = out.join(format!("{name}.png"));
            std::fs::write(&p, bytes)?;
            p
        }
        ExportFormat::Json => {
            let text = arda_render::area_json(world.manifest(), ax, ay, area.cells(), area.objects());
            let p = out.join(format!("{name}.json"));
            std::fs::write(&p, text)?;
            p
        }
    };
    Ok(path)
}
```

Add these accessors to `impl Area` in the same file:

```rust
    /// The raw cell grid, for renderers.
    #[must_use]
    pub const fn cells(&self) -> &AreaCells {
        &self.cells
    }

    /// The raw object lists, for renderers.
    #[must_use]
    pub const fn objects(&self) -> &AreaObjects {
        &self.objects
    }
```

Add `arda-render` to the `arda` facade's imports by adding `use arda_render;` — it is already a dependency from Task 1.

Rewrite `crates/arda-cli/src/main.rs`:

```rust
//! The `arda` binary: `generate` and `export` subcommands (`mockup/01`, `03`).

use anyhow::{bail, Context, Result};
use arda::{export_area, generate, ExportFormat, GenerateConfig, LatitudeBand, SizeKm, World};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "arda", version, about = "Deterministic procedural worldgen for tabletop")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the batch: continent, areas, blocks.
    Generate {
        /// The sole source of nondeterminism.
        #[arg(long)]
        seed: u64,
        /// Continent extent as WxH in kilometres.
        #[arg(long, default_value = "500x1000")]
        size: String,
        /// Use the 8-tile micro continent (the walking-skeleton slice).
        #[arg(long)]
        micro: bool,
        /// World directory to create.
        #[arg(long)]
        out: PathBuf,
    },
    /// Render or serialise part of a generated world.
    Export {
        /// The world directory to read.
        #[arg(long)]
        world: PathBuf,
        /// Area tile as `<ax>,<ay>`.
        #[arg(long)]
        area: String,
        /// Output format.
        #[arg(long, value_enum, default_value_t = Format::Png)]
        format: Format,
        /// Directory to write into.
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Png,
    Json,
}

fn parse_size(text: &str) -> Result<SizeKm> {
    let Some((w, h)) = text.split_once('x') else {
        bail!("size must look like 500x1000, got {text}");
    };
    Ok(SizeKm::new(
        w.parse().context("size width must be a number")?,
        h.parse().context("size height must be a number")?,
    ))
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Generate {
            seed,
            size,
            micro,
            out,
        } => {
            let config = if micro {
                GenerateConfig::MICRO
            } else {
                GenerateConfig::new(parse_size(&size)?, LatitudeBand::new(35, 55), 15)?
            };

            println!(
                "arda {} — deterministic worldgen (seed {seed}, size {}x{}km, {} areas)",
                env!("CARGO_PKG_VERSION"),
                config.size_km().width,
                config.size_km().height,
                config.areas_wide() * config.areas_high()
            );
            println!("[1/3] continent");
            println!("[2/3] areas");
            println!("[3/3] blocks");

            let manifest = generate(seed, config, &out)?;
            println!(
                "done — {} · {} areas · land {}‰",
                out.display(),
                manifest.stats.area_count,
                manifest.stats.land_fraction_permille
            );
            Ok(())
        }
        Command::Export {
            world,
            area,
            format,
            out,
        } => {
            let Some((ax, ay)) = area.split_once(',') else {
                bail!("area must look like 1,1, got {area}");
            };
            let ax: i32 = ax.parse().context("area x must be a number")?;
            let ay: i32 = ay.parse().context("area y must be a number")?;

            let world = World::load(&world)?;
            std::fs::create_dir_all(&out)?;
            let format = match format {
                Format::Png => ExportFormat::Png,
                Format::Json => ExportFormat::Json,
            };
            let path = export_area(&world, ax, ay, &out, format)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("wrote {}", path.display());
            Ok(())
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p arda-cli`
Expected: PASS — 5 tests pass.

Run: `cargo run -p arda-cli --release -- generate --seed 42 --micro --out /tmp/w42`
Expected: the `mockup/01`-shaped transcript, exit 0, `/tmp/w42/world.json` present.

- [ ] **Step 5: Commit**

```bash
git add crates/arda-cli crates/arda/src
git commit -m "feat: add generate and export cli subcommands"
```

---

### Task 17: Golden-world determinism gate

The §Q4 one-way door made enforceable: per-stage blake3 hashes committed to the repo and compared on Linux, macOS, and Windows. **`status: formalized` for the build stage lands when this is green on all three.**

**Files:**
- Create: `tests/golden_world.rs`
- Create: `tests/golden/micro-42.txt`
- Modify: `Cargo.toml` (workspace-level test target dependencies)
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `arda::{generate, World}` (Task 14)
- Produces: `tests/golden/micro-42.txt`, a stable list of `<relative path>  <blake3 hex>` lines

- [ ] **Step 1: Write the failing test**

Add to the root `Cargo.toml`:

```toml
[dev-dependencies]
arda = { workspace = true }
arda-core = { workspace = true }
blake3 = { workspace = true }

[[test]]
name = "golden_world"
path = "tests/golden_world.rs"
```

Create `tests/golden_world.rs`:

```rust
//! The cross-platform determinism gate (`architecture-interview.md` §Q4,
//! `06-testing.md` golden-world snapshots).
//!
//! Regenerating the fixture: run with `ARDA_BLESS=1` to rewrite
//! `tests/golden/micro-42.txt`, then inspect the diff before committing.
//! `code-prefs.md` §Q9 forbids changing golden hashes unless explicitly asked.

use arda::{generate, GenerateConfig};
use std::path::{Path, PathBuf};

const GOLDEN: &str = "tests/golden/micro-42.txt";

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("arda-golden-{}-{tag}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        Self(p)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Every world file, sorted, as `<relative path>  <blake3 hex>`.
fn fingerprint(root: &Path) -> String {
    let mut entries = Vec::new();
    collect(root, root, &mut entries);
    entries.sort();
    entries.join("\n")
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    // Sort so traversal order cannot vary by filesystem.
    let mut paths: Vec<PathBuf> = read.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();

    for path in paths {
        if path.is_dir() {
            collect(root, &path, out);
        } else if let Ok(bytes) = std::fs::read(&path) {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                // Windows writes backslashes; normalise so one fixture serves
                // all three runners.
                .replace('\\', "/");
            out.push(format!("{rel}  {}", blake3::hash(&bytes).to_hex()));
        }
    }
}

#[test]
fn micro_world_matches_the_golden_fingerprint() {
    let dir = TempDir::new("micro-42");
    generate(42, GenerateConfig::MICRO, dir.path()).expect("generation failed");
    let actual = fingerprint(dir.path());

    if std::env::var("ARDA_BLESS").is_ok() {
        std::fs::create_dir_all("tests/golden").expect("cannot create fixture dir");
        std::fs::write(GOLDEN, &actual).expect("cannot write fixture");
        return;
    }

    let expected = std::fs::read_to_string(GOLDEN).unwrap_or_else(|_| {
        panic!("{GOLDEN} is missing; run with ARDA_BLESS=1 to create it")
    });

    assert_eq!(
        actual.trim(),
        expected.trim(),
        "world fingerprint changed — a sim rule or the byte layout moved"
    );
}

#[test]
fn regenerating_produces_the_same_fingerprint() {
    let a = TempDir::new("repeat-a");
    let b = TempDir::new("repeat-b");
    generate(42, GenerateConfig::MICRO, a.path()).expect("generation failed");
    generate(42, GenerateConfig::MICRO, b.path()).expect("generation failed");
    assert_eq!(fingerprint(a.path()), fingerprint(b.path()));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test golden_world`
Expected: FAIL — `tests/golden/micro-42.txt is missing; run with ARDA_BLESS=1 to create it`.

- [ ] **Step 3: Write minimal implementation**

Generate the fixture, then **read the diff before trusting it**:

```bash
ARDA_BLESS=1 cargo test --test golden_world
cat tests/golden/micro-42.txt
```

Expected content shape — one line per world file, 8 areas plus 8 block archives plus the manifest and continent layer:

```text
areas/00_00/cells.bin  <64 hex chars>
areas/00_00/objects.bin  <64 hex chars>
...
blocks/01_03.tiles.zst  <64 hex chars>
continent/overview.bin  <64 hex chars>
world.json  <64 hex chars>
```

Then extend `.github/workflows/ci.yml`'s `test` job so the gate runs on all three runners — replace the single `cargo test` step with:

```yaml
      - run: cargo test --workspace --all-features
      - name: Determinism gate
        run: cargo test --test golden_world -- --nocapture
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test golden_world`
Expected: PASS — 2 tests pass.

Run: `cargo test --workspace`
Expected: PASS — the whole suite green.

Run: `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check`
Expected: clean.

Push and confirm CI is green on ubuntu, macos, **and** windows. If the fingerprint differs across runners, a float or an iteration-order dependency has leaked into a sim path — that is the §Q4 door closing, and it must be fixed rather than papered over by per-OS fixtures.

- [ ] **Step 5: Commit**

```bash
git add tests .github/workflows/ci.yml Cargo.toml
git commit -m "test: add cross-platform golden world determinism gate"
```

---

## Definition of done

Build-order steps 0–3 are complete when all of the following hold:

- [ ] `cargo test --workspace` green on Linux, macOS, and Windows in CI
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo fmt --all --check` clean
- [ ] `cargo deny check` clean
- [ ] `tests/golden/micro-42.txt` identical across all three CI runners
- [ ] `arda generate --seed 42 --micro --out ./worlds/w42` produces a loadable world
- [ ] `arda export --world ./worlds/w42 --area 1,1 --format png --out ./out` writes a PNG
- [ ] Adjacent tiles agree byte-for-byte on shared edges (Task 12's test)

At that point `build-interview.md` moves to `status: formalized`, and the `build/code@Q5` changelog entry records which steps landed, the source paths created, and any divergence from `implementation.md`.

## Self-review

**1. Spec coverage.** Against `implementation.md` steps 0–3:

| Spec item | Task |
|---|---|
| Workspace scaffold, five crates, lints, CI 3-OS, Dockerfile, deny.toml | 1 |
| `arda-core`: coords, fixed-point, rng, error enums, config | 2, 3, 4 |
| rng reference vectors (same key → same stream) | 3 |
| `formats`: manifest + cells + objects + blocks codecs | 5, 6, 7, 8 |
| Round-trip property tests; version-skew refusal | 5, 6, 7, 8 |
| Micro-continent config (8 tiles) | 4 (`GenerateConfig::MICRO`), 10 |
| Kinematic-lite tectonics pass | 10 |
| Relief + water only area | 12 |
| ~24-tile WFC subset | 8 (vocabulary), 13 (fill) |
| Symbolic PNG + JSON export | 15 |
| Load round-trip | 14 |
| blake3 stage hashes identical on 3 OSes | 17 |

Gaps deliberately carried, each with its owning build-order step:
- `continent/overview.bin` is written empty — the continent grid is regenerated rather than stored. Build-order step 4 gives it a real layout.
- Blocks are materialised on a 64-cell stride, not per land cell. Step 6 removes the stride.
- Areas load eagerly; `logic/05` wants lazy with an O(accessed) cache. Marked `ponytail:` in `World`.
- Climate, vegetation, settlement, land-use, roads, naming, hydrology objects: steps 4–5.

**2. Placeholder scan.** No "TBD", no "add error handling", no "similar to Task N". Every code step carries runnable code; every verification step carries an exact command and expected output.

**3. Type consistency.** Checked across tasks: `CellCoord::new` returns `Option` everywhere; `HeightMm::raw()` (not `value()`) used in Tasks 6, 10, 11, 12, 15; `Block::square`/`set`/`mark_relaxed`/`is_relaxed` consistent between Tasks 8, 13, 15; `AreaCells::get` returns `&Cell` in both Task 6 and its consumers; `boundary_height` returns `i32` millimetres in Tasks 11 and 12; `generate_area` returns `(AreaCells, AreaObjects)` in Tasks 12 and 14.

One inconsistency found and fixed inline: Task 11's `bundle_for` originally sampled its east edge at local x = `AREA_CELLS - 1`, which is *adjacent to* rather than *identical with* the neighbour's west column. The note after Step 3 corrects it to sample absolute column `AREA_CELLS`, which is what makes Task 12's `adjacent_tiles_agree_on_their_shared_edge_cells` pass rather than merely nearly-pass.
