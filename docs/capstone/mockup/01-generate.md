---
generated_date: 2026-09-07
scenarios: [batch-generate]
generated_at_commit: 8be0a0a
absorbed_from: features/03-climate-driven-refinement@2026-08-27
capstone_version: 6.4
---

# 01 — `arda generate` (CLI): the batch

> Retained design and dated implementation history. Current implementation: `../01-architecture.md`, `../04-data-flow.md`.

The product's front door (offline prep only). One seed in, one
continent out, fully materialized to compressed block tile-IDs.
Command and flag names are design assumptions; the intended semantics are confirmed decisions.

## Layout

```text
$ arda generate --seed 42 --out./worlds/w42
arda 0.1.0 — deterministic worldgen (seed 42, size 500x1000km, 190 areas)

[1/3] continent   ██████████████████████  done   18m
      relief · climate · regions · inter-area rivers · settlement density
[2/3] areas       ████████░░░░░░░░░░░░░░  71/190 2h 10m elapsed
      per area: relief → water → climate → vegetation → settlement
      → land-use → roads   (causal order)
[3/3] blocks      ░░░░░░░░░░░░░░░░░░░░░░  waiting
      64×64 tile-ID grids per cell, zstd-compressed

done — 6h 41m ·./worlds/w42 · 61 GB
  continent: 190 areas, 1,204 settlements, 388 named rivers
  validation: Horton 3.9 · Hack 0.56 · rank-size ok · sinuosity 1.27
```

Element tree: command → progress section (one row per tier, three tiers) → summary footer (counts + validation statistics).

## Elements

| Element | Exact form | Does | Status / notes |
| --- | --- | --- | --- |
| `generate` subcommand | `arda generate` | Runs the whole batch: continent → all areas → all block tile-IDs | (name assumed) |
| `--seed <u64>` | required-or-random; printed either way | Sole source of nondeterminism; same seed + config = identical world | Confirmed design |
| `--size <WxH>` | default `500x1000` (km) | Continent extent; flat map, ocean at edges | (flag name assumed) |
| `--out <dir>` | required | World directory to create (the docker volume target) | Confirmed design |
| `--config <file>` | optional | Overrides defaults (size, climate distribution, settlement density); schema deferred | "drill down later" (assumed) |
| progress rows | tier name + bar + counts | Hours-long batch must show position ("hours OK") | (rendering assumed) |
| validation footer | Horton / Hack / rank-size / sinuosity | Prints the chosen plausibility statistics for the generated world | Confirmed design |

## States

- **Success**: exit 0; world directory complete (see `02-world-layout.md`); summary footer printed.
- **Empty/initial**: `--out` must not contain a world already; refuse with "directory not empty" rather than overwrite (assumed).
- **Loading**: per-tier progress as above; safe to abort — nothing consumes a partial world, re-run regenerates identically from the seed (determinism; resume-vs-restart not specified, restart assumed).
- **Error**: invalid config → exit non-zero naming the field; disk-full mid-batch → exit non-zero, partial directory left for inspection, re-run restarts (assumed).

## Observed — validation footer (2026-08-27, feature 03)

The done line now reports the continent river count alongside the area count
and land fraction, e.g. `done —./worlds/w42 · 8 areas · land 487‰ · 5 rivers`.
The count is `ValidationStats.river_count` and covers continent river objects,
which are unnamed until `logic/01` step 8 — `named_river_count` stays 0. The
Horton / Hack / rank-size / sinuosity figures the mocked footer shows remain
unimplemented (build-order step 12).
