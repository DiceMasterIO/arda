## 2026-09-23 — Saved-wetness Atlas material and reference refresh

key: map/all@3f95fcca59a2

- Source `3f95fcc` uses saved drainage/slope wetness to tint Atlas land with Q12 weight `wetness×2048/(wetness+12)` toward RGB `[81,126,73]`, before existing rock/snow blending. Target and eight-neighbor halo samples carry the existing byte; the optional sample remains 16 bytes. Geometry, saved worlds, Classic and public options remain unchanged.
- Verify 98 renderer library tests, 12 public area-export tests, the final focused mixed-axis test, formatting and strict renderer Clippy. Two fresh independent GPT-6 Sol review rounds find no actionable defect; retain exact source hashes and commands in the local component receipt. These are scoped checks, not a new full-generation or remote CI run.
- Accept matched actual-world material comparisons as a partial improvement. Retain the invalid initial old-world comparison separately; adoption uses the corrected structural-relief world. No ecological cover or soil-moisture producer is claimed.
- Refresh `01-architecture.md`, `02-models.md`, `04-data-flow.md`, `07-operations.md` and `08-glossary.md` for saved-field flow, integer material ordering, fixed context size and display semantics.
- Refresh `06-testing.md` for exact tint, zero-wetness compatibility, cardinal/diagonal context, water isolation and mixed-axis buffered/streamed coverage. Absorb the shared area/overview behavior into `logic/04-export.md`.
- Refresh `open-items.md` and `00-index.md` to distinguish committed improvements from the open reference-quality objective. A replayed coarse-field diagnostic identifies persistent collision-source uplift as a contributor to smooth crests; its bounded convergence-magnitude prototype is rejected and is not a production feature. A subsequent adapted released multiscale-erosion preset is likewise rejected on the fixed crop; retain its source provenance, numerical controls and limitations without a production claim.
- Leave documentation unstaged during the continuing implementation run; no broad feature completion marker, push or PR is made.
- Verify actual repeated 15,522 × 32,768 output: 327,031,502 bytes, 76.16/76.05 s and 66,516/66,284 KiB child RSS. Both PNGs match exactly, all 523 world files remain unchanged, and root inspects the downsampled completed-map preview.
