# Preferred terrain: preserve physical scale in larger domains

Added a fixed-reference calibration and signed relief filter alongside the native-compatible integer source. Five 20/40/80 km cases repeat exactly and match the independent oracle within 1 mm. Actual Arda renders retain rugged detail that per-domain normalization flattened. Thirteen focused tests, Clippy and workspace checks pass. Original saved world and earlier evidence remain unchanged.

Normal world generation is not switched over. Macro/coast composition, drainage, fine persistence, full-world validation and final 32K delivery remain open. See `features/2026-09-23-terrain-corrections/evidence/physical-spectral-scale/README.md`.
