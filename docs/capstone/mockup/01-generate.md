---
generated_date: 2026-08-24
scenarios: [batch-generate]
implements: [Q9, Q10, Q18, Q20, Q22]
generated_at_commit: 8d3c9d9
---

# 01 — `arda generate` (CLI): the batch

The product's front door (§Q18: offline prep only). One seed in, one
continent out, fully materialized to compressed block tile-IDs (§Q22).
Command and flag names are assumed throughout; only their semantics are
interview-backed.

## Layout

```text
$ arda generate --seed 42 --out ./worlds/w42
arda 0.1.0 — deterministic worldgen (seed 42, size 500x1000km, 190 areas)

[1/3] continent   ██████████████████████  done   18m
      relief · climate · regions · inter-area rivers · settlement density
[2/3] areas       ████████░░░░░░░░░░░░░░  71/190 2h 10m elapsed
      per area: relief → water → climate → vegetation → settlement
      → land-use → roads   (§Q10 order, causal)
[3/3] blocks      ░░░░░░░░░░░░░░░░░░░░░░  waiting
      64×64 tile-ID grids per cell, zstd-compressed

done — 6h 41m · ./worlds/w42 · 61 GB
  continent: 190 areas, 1,204 settlements, 388 named rivers
  validation: Horton 3.9 · Hack 0.56 · rank-size ok · sinuosity 1.27
```

Element tree: command → progress section (one row per tier, §Q8's three
tiers) → summary footer (counts + §Q13 validation statistics).

## Elements

| Element | Exact form | Does | Traces to |
|---|---|---|---|
| `generate` subcommand | `arda generate` | Runs the whole batch: continent → all areas → all block tile-IDs | Q18, Q20, Q22 (name assumed) |
| `--seed <u64>` | required-or-random; printed either way | Sole source of nondeterminism; same seed + config = identical world | Q9, Q20 |
| `--size <WxH>` | default `500x1000` (km) | Continent extent; flat map, ocean at edges | Q22 (flag name assumed) |
| `--out <dir>` | required | World directory to create (the docker volume target) | Q17 |
| `--config <file>` | optional | Overrides defaults (size, climate distribution, settlement density); schema deferred | Q17 "drill down later" (assumed) |
| progress rows | tier name + bar + counts | Hours-long batch must show position (§Q20 "hours OK") | Q20 (rendering assumed) |
| validation footer | Horton / Hack / rank-size / sinuosity | Prints §Q13's plausibility statistics for the generated world | Q13 |

## States

- **Success**: exit 0; world directory complete (see `02-world-layout.md`); summary footer printed.
- **Empty/initial**: `--out` must not contain a world already; refuse with "directory not empty" rather than overwrite (assumed).
- **Loading**: per-tier progress as above; safe to abort — nothing consumes a partial world, re-run regenerates identically from the seed (determinism §Q9; resume-vs-restart not interviewed, restart assumed).
- **Error**: invalid config → exit non-zero naming the field; disk-full mid-batch → exit non-zero, partial directory left for inspection, re-run restarts (assumed).
