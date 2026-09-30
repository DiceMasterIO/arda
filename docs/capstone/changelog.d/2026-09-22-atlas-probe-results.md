## 2026-09-22 - progress: Atlas prototype results and workspace verification
key: progress/2026-09-22-atlas-visual-fidelity@probe-results

- Production palette source `9a16883`: `rtk cargo test --workspace --offline` passes 649 tests, 8 ignored across 21 suites; command wrapper elapsed 395.39 seconds. Production source remains unchanged by these experiments.
- Scratch ocean contour renderer: actual 2K/8K exports of saved area (2,1), matching coordinates and channel clipping; 81 scratch renderer tests pass. The 8K crop changes 3,292 of 1,048,576 pixels. Zero-height source samples force categorical fallback on 6,144 pixels under mixed quads. No production geometry or topology acceptance is claimed.
- Scratch continent routing experiment: identical pre-erosion field and unchanged erosion constants, but multiple-flow accumulation alone reduces measured local valley relief from 83.0 m to 16.2 m. Rejected as a correction because it loses the required valley shape.
- `open-items.md`: records the production verification, confirmed erosion-stage source of aligned gullies, rejected alternative, shoreline limitations and pending scope decision.
- Fixed seed-42 500×1000 km generation remains live; a separate follower waits for its terminal receipt, then inventories all areas and exports representative 8K Atlas views. It never restarts generation. Completion and visual inspection remain pending.
- The broader objective remains active. No implementation-completion marker is written.
