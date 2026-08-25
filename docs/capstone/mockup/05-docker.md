---
generated_date: 2026-08-24
scenarios: [batch-generate, inspect-volume, export-vtt]
implements: [Q17, Q19]
generated_at_commit: 8d3c9d9
---

# 05 — Docker container

Third delivery form (§Q17: "a docker container (with a volume where you
can look at or import or copy the maps generated)"). Same binary and
commands as 01/03; the volume is the interface. Registry GHCR assumed
(§Q19 left it unspecified).

## Layout

```text
$ docker run -v $PWD/worlds:/worlds ghcr.io/<owner>/arda \
    generate --seed 42 --out /worlds/w42
... identical output to 01 ...

$ ls worlds/w42          # inspect/copy from the host — the §Q17 scenario
world.json  continent/  areas/  blocks/

$ docker run -v $PWD/worlds:/worlds -v $PWD/maps:/maps ghcr.io/<owner>/arda \
    export --world /worlds/w42 --area 03_11 --out /maps
```

Element tree: image → entrypoint (the CLI) → mounted volume(s) holding
02's world layout and 03's exports.

## Elements

| Element | Does | Traces to |
|---|---|---|
| Image `ghcr.io/<owner>/arda` | Ships the CLI; no daemon, no ports — runs to completion and exits | Q17, Q18 (registry assumed) |
| `/worlds` volume | Host-visible world directories (02); the "look at or import or copy" surface | Q17 |
| Entrypoint = `arda` | Any 01/03 subcommand passes through verbatim | Q17 (assumed) |

## States

- **Success/error**: exit codes pass through from the CLI (01/03 States).
- **No volume mounted**: world written inside the container is lost on exit; warn when `--out` is not on a mount (assumed).
