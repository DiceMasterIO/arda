## 2026-09-23 — Atlas lake depth across map scales

key: feat/atlas-lake-depth@0fc9666

- Added validated saved-surface-minus-bed lake colour to Atlas area and overview maps using the shared Atlas water-depth ramp. Membership, river precedence, world data, Classic output and existing constructor fallbacks are preserved; lake-aware constructors are additive.
- Kept context bounded: each halo sample is 16 bytes on the verified target, with a transient 2 MiB lake-depth lookup per area/neighbor. No output-sized depth buffer or new saved field is introduced.
- Verified 94 renderer tests, 12 facade area-export tests, 14 CLI tests, scoped strict Clippy, formatting and two independent dry code reviews. Added corner and buffered/streamed mixed-axis coverage after review. Actual matched clean-baseline images were inspected; Classic lake-margin output is byte-identical. Current 32K output completed in 69.34 s at 76,904 KiB peak child RSS.
- Refreshed architecture, models, data flow, tests, operations, glossary, export/preview behavior, map legend and current status. No full workspace or remote CI run is claimed for this rendering continuation.
- Full visual acceptance remains open. Controlled terrain experiments are isolated evidence; ordinary broad relief plus longer evolution shows a partial branching improvement on one crop but has no accepted production integration yet.
