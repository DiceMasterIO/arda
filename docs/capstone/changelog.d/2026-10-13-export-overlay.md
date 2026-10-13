# 2026-10-13 — Settlements, roads and land use on overview and area exports

- **arda-render**: `BandOverlay` hook (`paint(y0, rows, width)`) called on each streamed overview band and area row group before encoding; `write_overview_png_with_overlay`, `write_atlas_overview_png_with_channels_overlay`, `OverviewRaster::finish_with_overlay`, `render_area_png_to_with_overlay`; `RenderError::Overlay`. `None` keeps the exact bytes (goldens untouched).
- **arda-settle**: `render::band::SocietyBandOverlay` (land-use tone, roads by class, tier symbols, painted in 128-column strips from whole-image geometry); `layers.rs` split into band-paintable pieces (`road_lines`/`paint_roads`, `symbol_list`/`paint_symbols`, `land_tone`) with `arda-settle render` byte-identical.
- **arda-midzoom**: `land::Works` (parcels, tone, roads, plans) factored out of `LandWindow`; `land::export::AreaLandOverlay` for area PNGs; `export` module (`SocietyMode`, `export_overview`, `export_area`).
- **arda**: `export_overview_with_overlay`, `export_area_with_overlay`.
- **arda-cli**: `--society auto|off` on `preview` and `export`.
- **arda-server**: `?overlay=society|none` on `/v1/overview.png` and overview tiles; default `society` for worlds that have it; cache keys include it.
- **Why**: a GM looking at the world map, and DiceMaster's static base map, need the human layer, not bare terrain (OI-12, OI-14).
- **Tests**: arda-render hook (call order, `None` identity, row buffer independent of height); arda-settle band, strip and window seam equivalence; arda-midzoom border-settlement equality and Off/On export; arda-server overlay routes.
