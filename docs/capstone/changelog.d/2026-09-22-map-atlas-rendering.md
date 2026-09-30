## 2026-09-22 - map: Atlas rendering reference refresh
key: map/all@342d03e55120-atlas-rendering

- Trigger: source changed across the renderer, facade and CLI; source stamp `342d03e55120`.
- `01-architecture.md`: styled export entry points, renderer/facade ownership, sequential neighbor reads and bounded sampling/streaming.
- `02-models.md`: in-memory MapStyle, AtlasNeighbor, AtlasHalo and AtlasTerrain, exact sampling and validation; persisted schemas remain unchanged.
- `04-data-flow.md`: style refusal, halo derivation, per-pixel interpolation, publication and failure paths.
- `06-testing.md`: focused tests, current Linux gates, independent Classic baseline and real-map evidence; preserves historical records.
- `07-operations.md`: CLI selection and legacy compatibility, actual 32K timings/RSS and bounded neighboring context.
- `logic/04-export.md`: absorbs style branches, eight-neighbor loading, class filtering, water authority and publication.
- `logic/07-preview.md`: absorbs Atlas preview and explicit-style conflicts with legacy `--px`.
- `mockup/03-export.md`: current user-visible choices, filenames and errors.
- `mockup/04-crate-api.md`: additive implemented styled APIs alongside explicitly retained future design.
- `logic/README.md`, `mockup/README.md`, `00-index.md`: companion/index descriptions reflect the delivered first pass.
- `open-items.md`, `implementation.md`, `README.md`, `../tactical-map-roadmap.md`: preserve delivered generation work and earlier queue; mark only the Atlas slice delivered, with ecology/human/tactical scope still open.
- `../map-legend.md`: separates Classic legend from Atlas relief/depth interpretation; neither elevation palette implies forest or simulated snow.
- Divergences: Atlas `--detail` intentionally uses the streamed 4096 standard filename; Classic retains the legacy `_detail` filename. The concept is a visual direction, not exact generated detail.
- Unchanged source topics and retained generation scenarios were left untouched; no frontend/UI chapter or cross-repository contract was introduced. This is a targeted reference refresh, not a claim that every unrelated reference stamp is current.
- Verification: source-citation spot checks, 147 resolved local links, 40 previously documented absent historical feature links, no unexpected missing links and a clean whitespace check; a separate Sol documentation review found three stale detail-filename descriptions, corrected before completion.
