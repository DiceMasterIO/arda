# Fine terrain in Atlas and source-boundary correction

- **What**: normal Atlas exports consume optional saved canonical fine terrain with bounded windows and global physical sampling.
- **Source**: one fixed smooth local-range gain removes the old 400 m dead zone; an existing-rim-derived taper keeps outer source edges ocean. Source recipe 2 is explicit provenance; recipe 1 remains loadable.
- **Verification**: 250 core/render/facade tests, 15 focused gen checks, all-target workspace Clippy, formatting and diff checks pass; corrected source and 35 complete small-world files replay exactly, saved heights match, and annual water closes.
- **Preservation**: all 523 original-world files and exact file set unchanged.
- **Visual finding**: fine detail survives normal exports; broad uniform lowland and sharp relief/material front persist. Exact replay attributes the source plateau to the inherited tectonic subsidence clamp; saved wetness saturates there.
- **Open**: reference-quality appearance, default source adoption, multiple worlds, full-size runtime/resource qualification and final seed-42 32K delivery. No completion marker.
- **References**: current status, architecture, models, testing, export scenario and feature plan updated; evidence in fine-atlas-integration, source-lowland-boundary and macro-stage-attribution.
- **Next defect isolated**: saved wetness computes a different expression from its documented log-ratio index, with a demonstrated ordering reversal; no wetness or tectonic variant was implemented.
