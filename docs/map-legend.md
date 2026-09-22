# Map legend

Open the [colour legend](map-legend.html) in a browser. It is a standalone,
offline copy of the **Classic** palette legend for world and detailed area renders.

Land colours blend continuously between elevation anchors. Green indicates
low elevation, not forest cover; white indicates high elevation, not simulated
snow. Classic does not add hillshading or a biome overlay.

`--style atlas` adds an earthy elevation palette, northwest relief lighting
computed from neighboring saved heights, and depth-based sea colours. Land
palette and lighting are interpolated separately at every output pixel.
Neither style models forest cover or snow from elevation colour. Atlas keeps
the saved 100 m land/water masks, so close-up coastlines can still look stepped.

World-map river colours represent annual mean discharge. Streams below 4 m³/s
are omitted from the overview, and line widths are symbols rather than physical
river widths. Detailed area maps use saved channel widths and depth-dependent
water colours; the 512² preview adds a faint marker for subpixel channels.

The Classic palette is defined in `crates/arda-render/src/carto.rs`; Atlas
palette, sampling and relief lighting are in `atlas.rs`, with detailed
channel colours in `channels.rs` and overview drawing in `overview.rs`.
