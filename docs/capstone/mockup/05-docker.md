---
generated_date: 2026-09-07
scenarios: [batch-generate, inspect-volume, export-vtt]
generated_at_commit: 8d3c9d9
capstone_version: 6.4
---

# 05 — Docker container

> Retained design and dated implementation history. Current implementation: `../01-architecture.md`, `../04-data-flow.md`.

Third delivery form ("a docker container (with a volume where you
can look at or import or copy the maps generated)"). Same binary and
commands as 01/03; the volume is the interface. Registry GHCR assumed
(no registry was specified).

## Layout

```text
$ docker run -v $PWD/worlds:/worlds ghcr.io/<owner>/arda \
    generate --seed 42 --out /worlds/w42
... identical output to 01...

$ ls worlds/w42          # inspect/copy from the host — the volume-inspection scenario
world.json  continent/  areas/  blocks/

$ docker run -v $PWD/worlds:/worlds -v $PWD/maps:/maps ghcr.io/<owner>/arda \
    export --world /worlds/w42 --area 03_11 --out /maps
```

Element tree: image → entrypoint (the CLI) → mounted volume(s) holding
02's world layout and 03's exports.

## Elements

| Element | Does | Status / notes |
| --- | --- | --- |
| Image `ghcr.io/<owner>/arda` | Ships the CLI; no daemon, no ports — runs to completion and exits | (registry assumed) |
| `/worlds` volume | Host-visible world directories (02); the "look at or import or copy" surface | Confirmed design |
| Entrypoint = `arda` | Any 01/03 subcommand passes through verbatim | (assumed) |

## States

- **Success/error**: exit codes pass through from the CLI (01/03 States).
- **No volume mounted**: world written inside the container is lost on exit; warn when `--out` is not on a mount (assumed).
