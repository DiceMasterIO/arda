# Contour construction qualification

Built a headless adapter for a pinned, non-DEM author example from the 2026 iso-contour terrain paper. Source review corrected missing GUI height normalization and exterior fade before execution. An initial sampler abort was traced to signed-overflow optimization in the author RNG; an isolated `-fwrapv` build preserves the intended sequence, with full 100,000-value parity against the unoptimized probe.

Two fresh-process terrain runs complete in about 2.217 seconds each and reproduce graph, contour, mask and height data byte-for-byte. A separate explicit-mask Atlas runner passes original all-land PNG parity and renders fixed whole/central views. Root and independent visual review find broad polygonal terraces and a central mound, failing the declared broad mountain-form gate. Close this fixture without an amplification or parameter sweep; preserve the user's previous baseline.

No production code or original saved-world data changed. The reference-quality goal remains open. [Evidence and actual images](../features/2026-09-23-terrain-corrections/evidence/iso-contour-author-control/README.md).
