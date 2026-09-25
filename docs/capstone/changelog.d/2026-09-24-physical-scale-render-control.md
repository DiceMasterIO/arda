# Physical-scale renderer diagnostic

- Measured oversteep native geometry in the height-gained probe: above2 km median slope68.483°,89.10% of area steeper than45°. Along-crest uplift already varies substantially, so no generic crest modulation follows.
- Added a scratch-only explicit-spacing Atlas constructor. Default100 m image is byte-identical; the one483.4857455 m interpretation recalculates both slope material and light with unchanged heights/palette/sun.109 renderer tests and independent review pass.
- Actual nadir/60° comparisons look less wall-like at the wider physical extent, but mountain faces remain soft and reference quality is not reached. Correct physical camera scaling, prior projection parity, nadir image identity and unclamped shown interiors are verified.
- Full source crop51.2→247.545 km is a physical reinterpretation, not the same geography, new resolution or an evolution run. Production and saved world remain unchanged. Evidence: `features/2026-09-23-terrain-corrections/evidence/physical-scale-render-control/`.
