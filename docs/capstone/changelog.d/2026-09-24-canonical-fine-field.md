# Canonical fine terrain sampling

- Added immutable `TerrainField` and typed absolute `TerrainPoint` to arda-core, with validated coverage and integer bilinear interpolation retaining fine millimetre heights.
- Added a raw-input diagnostic consumer and verified exact shared-point consistency across nested resolutions, overlapping regions and repeated processes on three procedural sources.
- Preserved the preferred actual Arda appearance: all three full views remain pixel-identical, while close-view height differences stay within 1 mm.
- Passed 100 core tests, all-target core Clippy, workspace formatting and workspace compilation; independent review found no blocking defect.
- World source generation, fine-field persistence, production rendering, drainage and final 32K delivery remain incomplete. No saved-world migration or rewrite.
