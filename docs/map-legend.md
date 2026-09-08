# Map legend

Open the [colour legend](map-legend.html) in a browser. It is a standalone,
offline copy of the legend for the current world and detailed area renders.

Land colours blend continuously between elevation anchors. Green indicates
low elevation, not forest cover; white indicates high elevation, not simulated
snow. The renderer does not add hillshading or a biome overlay.

World-map river colours represent annual mean discharge. Streams below 4 m³/s
are omitted from the overview, and line widths are symbols rather than physical
river widths. Detailed area maps use saved channel widths and depth-dependent
water colours; the 512² preview adds a faint marker for subpixel channels.

The palette is defined in `crates/arda-render/src/carto.rs`, with detailed
channel colours in `channels.rs` and overview drawing in `overview.rs`.
