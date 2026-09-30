# Exact native-mesh raster transfer diagnostic

- Verified all original pinned TIN faces and conditional builder edge insertion; the sole complete-mesh graph difference is an omitted fixed-hull edge, with no interior differences. Independently checked barycentric sample heights and retained original natural-neighbor field/image parity.
- On unchanged evolved heights and 100 m center locations, triangle-linear interpolation reduces central geometric sampled basin capacity4.180306→2.015581 km³ and maximum depth180.269→139.916 m. Native capacity0.020302 km³ / maximum6.399 m remains much smaller. These are not canonical annual-water results.
- A valid native minimax outlet route reaches183.79183 m maximum, but its contiguous pixel chain samples heights up to367.885 m. Actual Atlas changes local faceting without resolving dense ridge walls; no production adoption. Full reference-quality landforms remain open.
- Evidence: `features/2026-09-23-terrain-corrections/evidence/triangle-linear-transfer-control/`.
