# Roadmap: world, area and illustrated tactical maps

Written 2026-09-21; map-detail requirements clarified 2026-09-22. This is a proposed roadmap for Arda and DiceMaster, grounded in the current code and project documents. It records the intended work and acceptance criteria; it is not an implementation-completion claim or a replacement for the existing approved plans.

## Goal

Generate playable locations with the visual richness of the supplied [map reference](../../dicemaster/capstone/uiux/assets/references/map.jpg): believable buildings and interiors, worn paths, integrated vegetation, detailed water edges, furnishings, small props, coherent lighting, and few visible repetitions. Their visible geometry must agree with movement, elevation, cover, and visibility.

The production pipeline is:

**World context → shared settlement/site plan → detailed terrain and structures → constrained WFC assembly where useful → furnishing and decoration → illustrated tactical rendering → playable VTT.**

WFC is one assembly tool within this pipeline. Solving adjacency does not establish a sensible town plan, an accessible building, or good art direction. Some features should be placed directly by layout rules or templates; others benefit from WFC. Illustrated assets can span several gameplay squares, and several visual layers can occupy the same square.

## Map detail by scale — confirmed 2026-09-22

| Map scale | Required visible detail |
|---|---|
| World | A substantially more detailed geographical render, plus city, town and village outlines, main roads, and other features observable at that distance. Settlement outlines represent their built-up extent. |
| Area | Individual building outlines and smaller roads/paths, alongside the larger features already established in the world view. |
| Tactical | Full detail: terrain surfaces, structures, entrances, interiors, furnishings, vegetation and props, with playable geometry and overlays. |

All three views use the same underlying feature identities and geometry. The
shared settlement/site plan establishes settlement extents, road hierarchy and
alignments, crossings and building footprints. Each renderer selects and
simplifies those features for its scale, preserving their locations, extent and
connections. Zooming in adds detail to the same place.

World and area rendering can consume this plan before interiors and furnishings
are generated. The 100 m terrain raster does not limit building-outline precision:
footprints and routes need separate geometry in world coordinates. Detailed
generation must respect those published outlines and connections. If a layout
revision changes them, update the shared record and dependent views together.

## World-map rendering milestone — clarified 2026-09-22

The world-map target includes a substantial increase in visible geographical
detail over the current presentation. Adding settlement outlines and road lines
alone does not satisfy it. The existing physical world is the starting point;
its overall visual quality has not been declared finished.

- Establish a world-scale visual reference and normal viewing size separately from the close-up tactical reference. Candidate rendering improvements include relief shading that exposes ridges and valleys, clearer coast/water edges, and richer treatment of vegetation and land use where their data exists.
- Use the saved terrain and connected water geometry for these improvements. Increasing export resolution alone does not add geographical information; finer physical features require generation work when the existing data cannot support them.
- Add forest, farmland and built-up-area appearance from actual generated cover and human-geography records as those producers become available. Rendering must preserve the shared positions, extents and connections used by area and tactical views.
- Assess remaining geography defects separately from presentation. Recheck the historically reported parallel drainage, angular shores and rectangular basins on representative outputs before deciding which need correction. A rendering pass cannot establish that an underlying shape defect is fixed.
- Rendering improvements and human-geography work can proceed on the current baseline. Additional terrain/water corrections remain valid work when inspection identifies a problem; the whole world generator does not have to be restarted to improve its presentation.
- **Complete when:** matched-scale before/after views demonstrate substantially richer world-map detail, settlements and main roads remain readable, and the review distinguishes rendering improvements from unresolved generation limits.

## Starting point

- Arda already implements continental relief/climate, one shared fine-terrain domain, connected drainage, globally shared lakes and rivers, and representative annual water accounting. The regional-relief correction and erosion across area boundaries are delivered work.
- Area height, temperature, rainfall, slope, drainage, discharge, channel properties and wetness have producers. Simple Bare/Grass/Marsh ground cover exists; full ecological vegetation and forest density remain future work.
- Format-4 storage, lazy loading, owned area reads, PNG/JSON exports, configurable map quality through 32K, the CLI, Rust API and Docker definition are implemented. These are foundations for this roadmap, not tasks to rebuild.
- The Atlas geographical rendering first pass is delivered at source `342d03e55120`: neighbor-aware relief, earthy elevation colours, sea-depth colour and per-output-pixel interpolation over the saved 100 m geography. Classic remains the default. The [current status and detailed work queue](capstone/open-items.md) records its matched real-map panel and verification. Straight/parallel drainage forms and stepped shorelines remain visible; wider terrain acceptance remains open.
- Tactical generation exists as a **24-tile skeleton**, materialized at a **64-cell sampling stride**. Its constraints mainly select broad allowed tile groups. Its solver makes uniform selections and updates immediate neighbors; it does not yet implement the richer constraint and propagation system described below.
- Block PNGs use an **eight-pixel-per-square symbolic renderer**. Block JSON contains tile IDs, an ID/name legend, dimensions, versions, a seed, and a relaxed-fill marker. It is not yet a complete playable scene description.
- Full vegetation, settlements, roads, buildings, furnishings, and tactical geometry are still unfinished. Existing types and retained design descriptions do not mean their producers exist.
- DiceMaster describes a future renderer with its own tileset and shared spatial geometry. Its flat cartographic visual specification conflicts with the illustrated target. Its reference mockup embeds an image; that is visual evidence, not a working procedural pipeline.

Current evidence: [open items](capstone/open-items.md), [block constraints](../crates/arda-gen/src/block/constraints.rs), [solver](../crates/arda-gen/src/block/wfc.rs), [tile vocabulary](../crates/arda-core/src/tiles.rs), [export behavior](capstone/mockup/03-export.md), and [DiceMaster renderer plan](../../dicemaster/capstone/01-architecture.md).

The [detailed work queue](capstone/open-items.md#detailed-work-queue--2026-09-22) is the step-by-step delivery companion: it includes current fixes, dependencies, code targets, completion gates and a mapping of every roadmap step below. Its OI-01–OI-31 entries also cover CI, ecology, society, transport, calibration and releases.

## Steps

### 1. Make the illustrated reference the explicit acceptance target

- Record which qualities must carry across: readable overhead spaces, cohesive hand-painted materials, natural edges, convincing scale, purposeful furnishing, and restrained detail variation.
- Define a separate world-map visual target and comparison set. Require a richer geographical presentation as well as settlement/road overlays; retain the agreed detail split between world, area and tactical views.
- Reconcile DiceMaster's flat-fill/repeating-symbol specification with this target. Retain useful interaction and readability requirements while replacing the incompatible art direction.
- Compare true overhead and the currently specified slight orthographic tilt using the same sample scene. Fix the camera and permitted rotation before producing a large asset library; painted perspective and baked shadows must remain believable in that camera.
- Establish the normal play zoom, closest useful zoom, and review image size. Test candidate asset densities, such as 64/96/128 pixels per square, instead of assuming more pixels alone create better art.
- Choose the first visual benchmark: a small riverside customs house or warehouse with a bridge, dock, road, roofless interior, trees, cargo, and shallow/deep water.
- **Complete when:** the project has a concrete visual brief, comparison views, and a checklist against which a real rendered location can be judged.

### 2. Resolve physical scale and cross-block coordinates

- Reconcile the current constants: **64 × 1.524 m = 97.536 m**, while the parent terrain cell spans **100 m**. Merely stretching the artwork would leave the physical meaning of movement and positions unresolved.
- Choose an explicit mapping between the world terrain lattice and the tactical grid. A candidate is a continuous five-foot global grid whose 64-square chunks sample the world terrain by physical position; assess the required changes to the current one-cell/one-block addressing before adopting it.
- Define origins, axes, units, rounding, cell ownership, and conversions between world positions, tactical squares, rendered positions, and pointer picks.
- Define canonical ownership of shared borders and objects that cross them. A bridge, road, wall, riverbank, or building must have one identity and one layout across all affected chunks.
- Update dependent formats and assumptions together, including entity coordinates, fog indexing, saved block keys, and DiceMaster's spatial calculations. Preserve old worlds through explicit version handling.
- **Complete when:** neighboring test chunks align in physical space, and movement/selection can cross their boundary without a gap, overlap, or coordinate drift.

### 3. Define the tactical scene contract and ownership

- Separate semantic information from appearance: ground material, elevation, structures, obstacles, and movement surfaces describe the scene; asset IDs and visual variants describe its presentation.
- Include shared settlement extents, building footprints, road/path hierarchy, route geometry and stable feature IDs. World, area and tactical views derive their representations from these records; simplified map views do not require detailed interiors to exist first.
- Model ground surfaces, water surfaces/depth where needed, walls, doors, openings, room/building identity, object footprints, heights, and stable object IDs. Use sparse objects for furniture and props rather than forcing every combination into one tile ID.
- Account for stacked surfaces where required: a bridge deck can be walkable above water. A single elevation and one occupancy flag per square cannot express both surfaces adequately. Define the first supported vertical cases and defer other cases explicitly.
- Give meaningful objects collision and interaction metadata. Keep cosmetic grass, stains, leaves, and tiny stones free of gameplay effects unless the scene deliberately marks them otherwise.
- Assign responsibility: Arda owns reproducible base scene generation; DiceMaster owns its visual asset catalog and presentation; shared scene geometry feeds client previews and server resolution; campaign state owns changed doors, moved objects, and similar mutable state.
- **Complete when:** one fixture can be exported, loaded, rendered, and queried without inferring walls, doors, or movement rules from image pixels or human-readable tile names.

### 4. Produce a small, coherent starter asset kit

- Build only the materials needed for the benchmark: grass, dirt, mud, cobblestone, stone floors, timber, water, banks, masonry walls, doors, docks, bridge pieces, trees, reeds, crates, barrels, tables, shelves, sacks, and carts.
- Establish common scale, palette, line/detail density, perspective, contrast, and light direction. Compare every asset in the map at play zoom, not only in isolation.
- Author structural joins and transitions: wall ends/corners, door frames, road intersections, floor borders, shore edges, and bridge approaches. Edge matching needs both semantic compatibility and believable artwork.
- Supply variants selectively: silhouettes for trees and rocks, wear patterns for floors and roads, and grouped cargo arrangements. Avoid spending effort on hundreds of nearly identical files.
- Define an asset manifest with stable IDs, dimensions, anchor/pivot, footprint, render layer, allowed rotations, variants, and links to semantic object kinds. Track source and redistribution rights for supplied, commissioned, or generated art.
- **Complete when:** the kit can assemble the benchmark scene at the agreed camera and scale without mismatched lighting, obvious joins, or missing structural pieces.

### 5. Build an early visual and gameplay proof

- Assemble one deliberately authored test location using the proposed scene format and starter kit. This is a controlled fixture to evaluate art and geometry before requiring the generator to design everything.
- Include the difficult interactions early: a door in a wall, a walkable bridge over water, furniture that provides obstruction, a large tree canopy, and a road/shore transition.
- Render independent layers for ground, surfaces, structures, furnishings, vegetation, shadows, and gameplay overlays. Keep the grid, tokens, fog, and highlights out of the artwork.
- Add minimal movement and visibility inspection plus overlays for footprints and heights. The fixture should prove that the art depicts the same usable space as the geometry.
- Compare it directly with the reference at matched scale and zoom. Iterate the asset kit and renderer until the quality direction is credible.
- **Complete when:** one interactive, authored location looks close enough to the target to justify proceduralizing it. A full-scene background image alone does not pass this milestone.

### 6. Supply the missing world-to-site context

- Audit which saved world fields have real producers, which are defaults, and which consumers merely assume exist. Complete the inputs needed for the first site rather than implementing the entire society simulation first.
- Produce or derive suitable vegetation, ground-cover, moisture, slope, and land-use inputs so a marsh, forest edge, road verge, and town yard receive different treatment.
- Add the settlement, road, crossing, and site-purpose information needed to justify where the benchmark location exists. A riverside warehouse should have access to both the water and a usable land route.
- Build a deterministic site input bundle containing the relevant terrain, connected water geometry, access corridors, settlement context, and neighboring constraints.
- Use clearly identified fixtures while these producers are incomplete, then replace them with real generated inputs. Do not present fixture-derived layouts as evidence of end-to-end world generation.
- **Complete when:** a saved-world location supplies enough context to choose and constrain an appropriate tactical site reproducibly.

### 7. Generate the site layout before tile assembly

- Reserve the primary layout: river corridor, banks, roads, bridge approaches, parcels, building footprints, yards, and access routes. Work over the whole site when it spans multiple storage chunks. Record how this changes the retained block-input rule of one cell plus eight neighbors: either publish a shared upstream site layout for blocks to consume or explicitly expand that input contract.
- Persist the shared layout needed by world and area maps before furnishing or WFC detail: settlement outlines and main roads for world scale; building outlines and smaller roads/paths for area scale. Tactical generation consumes these same footprints and connections.
- Choose building and site templates by purpose and surroundings. Vary dimensions and orientation within explicit constraints instead of scattering isolated wall tiles.
- Make connections intentional: roads meet entrances, paths reach docks, bridge approaches are traversable, and cargo areas have access to the warehouse.
- Reserve useful open space, circulation, encounter routes, and visibility breaks. Density should serve the location and gameplay rather than maximize object count.
- Produce fixed anchors, allowed regions, forbidden regions, entrances, and required connectivity for subsequent stages. Ensure a deterministic owner derives layouts crossing chunk boundaries.
- **Complete when:** a simple debug drawing already reads as a plausible place and its required destinations connect, before decorative art is added.

### 8. Refine terrain, water, and traversable surfaces

- Derive tactical elevation from the saved world terrain and add controlled local features where justified: banks, terraces, steps, ditches, ramps, and foundations. Interpolating 100 m heights alone cannot recover those details.
- Preserve the authoritative river corridor, water surface, and neighboring boundary conditions. Add bank shape and local variation without inventing disconnected water or incompatible elevations at seams.
- Construct usable road grades, level building floors, stairs/ramps, dock surfaces, and bridge decks. Validate clearance and access where one surface passes over another.
- Define difficult ground, non-traversable ground, water access, and hazards explicitly. Let fine visual edges differ from the square grid only within the documented collision and selection rules.
- Keep terrain and water generation independent from visual resolution. Exporting a larger image must not change playable geometry.
- **Complete when:** local elevation and surface transitions are believable, connected across boundaries, and sufficient for the intended movement/visibility queries.

### 9. Generate buildings and interiors as coherent structures

- Refine each shared building footprint into rooms, walls, doors, windows/openings and floor elevations. Derive both visual structure and collision geometry from this layout while preserving the outline published to the area map. Any required footprint revision updates the shared plan and its dependent views together.
- Apply purpose-specific room rules: storage space, office, workshop, dwelling, or tavern have different proportions, access needs, and furnishing zones.
- Validate wall closure where intended, doorway dimensions, accessible entrances, room connectivity, and clearance around stairs or level changes.
- Handle large buildings across chunks as one building. Cropping a scene for storage or rendering must not generate another entrance, cut away a wall, or duplicate a room.
- Implement roof/canopy display rules so playable interiors and tokens remain legible. Add upper floors only when the scene, movement, visibility, and presentation contracts can all represent them.
- **Complete when:** several generated structures have coherent interiors and no unintended sealed rooms or visually open but physically blocked entrances.

### 10. Upgrade WFC to assemble constrained local detail

- Replace broad wetness-based compatibility with directional rules appropriate to the pieces: wall connections, door openings, road edges, floor boundaries, and terrain transitions.
- Accept per-position domains, pinned pieces, reserved paths, and terrain/structure masks from the layout stages. Major spatial decisions already made by those stages must remain fixed.
- Propagate domain reductions through a work queue until they stabilize or a contradiction occurs. Add weighted choices, deterministic tie-breaking, and cached selection state so variation and performance remain controllable.
- Use WFC where local compatibility is useful. Keep whole-building planning, global connectivity, and purposeful object groups in layout/template/validation stages unless there is a demonstrated reason to integrate them into the solver.
- Bound retries and measure contradictions. Design a fallback that preserves mandatory boundaries, routes, and structures while reducing optional detail; otherwise report a site that could not be generated. The existing first-allowed-tile fill is insufficient for a playable location.
- **Complete when:** seed sweeps produce varied, coherent scenes; hard constraints survive success and fallback; propagation, contradictions, and fallback are exercised by meaningful fixtures. Tile count alone is not an acceptance criterion.

### 11. Furnish and dress the generated location

- Place functional groups by context: sacks and crates near loading access, shelves along suitable walls, desks with chair clearance, carts in usable yards, and reeds along wet banks.
- Separate meaningful obstacles from cosmetic scatter. Large furniture, trunks, fences, and carts receive authoritative footprints; decorative grass and stains do not quietly block movement.
- Reserve circulation and interaction clearance before placing objects. Revalidate required routes after furnishing; a plausible floor plan can become unusable through clutter.
- Add wear where activity explains it: dirt near entrances, wheel tracks along traffic, dampness by water, and moss in suitable margins. Use controlled clustering instead of uniform noise.
- Seed decoration separately from structural layout. An art-only variation should not move a doorway, alter a path, or change collision.
- **Complete when:** furnished sites look inhabited, remain navigable, and vary purposefully without repeated cargo arrangements dominating the view.

### 12. Develop the illustrated renderer to production quality

- Render all three scales from the shared plan: settlement outlines and main roads on the world map; building outlines and smaller roads/paths on area maps; full illustrated detail on tactical maps. Choose detail by display scale without relocating or independently regenerating the features.
- Implement the world-map detail milestone alongside those overlays, using terrain relief and supported vegetation/land-use detail. Review saved geography independently so presentation improvements do not conceal a generation defect or imply that historical visual issues are resolved.
- Render semantic terrain through layered materials, masks, and transitions so the gameplay grid does not become a visible patchwork. Keep render tiles/chunks distinct from five-foot gameplay squares.
- Support assets larger than a square and overlapping layers: tree canopies, wall shadows, docks, bridges, floor decals, and grouped props. Implement consistent draw ordering and occlusion.
- Choose a coherent shadow approach for the fixed camera and light. Avoid doubling baked shadows with dynamic shadows, and prevent rotated variants from implying conflicting light directions.
- Add contact shading, limited color variation, shoreline detail, and optional restrained water motion after the static image works. Animation is polish, not a substitute for the illustration quality.
- Build grid/fog/selection/path/targeting layers with adjustable readability and debug views. Trees, roofs, shadows, and decorative clutter must not obscure the information required to play.
- **Complete when:** generated scenes approach the reference at normal and close zoom, maintain a consistent art style, and remain clear with tokens and tactical overlays present.

### 13. Persist and export reproducible tactical scenes

- Extend saved formats and block/scene exports to carry the required geometry, semantic objects, stable identities, provenance, and generation/schema versions. Preserve explicit handling of missing or incompatible data.
- Export the shared settlement/building outlines and route geometry needed by overview and area renderers independently of detailed interior payloads. Key cached views to the layout version so a changed footprint or crossing cannot leave another zoom level showing stale geometry.
- Pin procedural output to the relevant generation version and inputs; pin visual reproduction to the asset catalog/version as well. A seed alone cannot guarantee identical results after rules or assets change.
- Use separate deterministic random streams for structure, furnishings, and cosmetic variants. Avoid time, iteration order, or request order affecting the generated result.
- Persist campaign changes separately from the immutable generated base. Re-entering a site must preserve an opened door or moved crate when the game records those changes.
- Define actual bounded transport endpoints and payloads for DiceMaster. The currently documented HTTP interfaces are plans; implement and verify the adapter/service instead of assuming it exists.
- **Complete when:** a site survives save/load/revisit, the client and server consume compatible scene data, and regenerating the base does not erase campaign state.

### 14. Integrate authoritative tactical gameplay

- Feed movement, obstruction, visibility, cover, and targeting from the same canonical geometry that drives the image. Server decisions and client previews must use compatible rules and scene versions.
- Verify doors, large furniture, trees, walls, stairs, and bridge decks with concrete movement and sight examples. Do not infer obstruction merely from the presence of a painted shadow or canopy.
- Add token placement, pointer-to-world selection, route preview, destination validation, fog, and explored-state persistence across chunk boundaries.
- Ensure hidden objects and creatures are filtered appropriately by the game rather than merely painted over in the client. Overlay rendering must not become the authority for what the player may know.
- Exercise the actual project camera, token sizes, vertical positions, and mobile input. Close camera alignment is part of usable gameplay, not just screenshot quality.
- **Complete when:** representative journeys and encounters behave as the rendered environment suggests, including boundary crossings and saved-state restoration.

### 15. Replace sparse tactical sampling with a coverage strategy

- Measure the cost of detailed sites/chunks before removing the current 64-cell stride. Full-world materialization multiplies work and storage dramatically; it should not be enabled blindly.
- Compare explicit alternatives: offline generation of every supported chunk, deterministic generation on demand, or a hybrid with prepared important sites. The retained design favors offline materialization, so any change is an architectural decision to record.
- Keep global/site layout and boundary ownership independent of request order. For on-demand generation, save the source inputs and generation version needed to reproduce the same location later.
- Make unsupported or unavailable locations visible to the caller. Prevent a character from moving into unloaded or missing geometry while displaying a finished-looking map there.
- Adopt per-chunk indexing and bounded reads where needed; the current whole-area block archive decompression can become unsuitable as coverage grows.
- **Complete when:** the intended play area has dependable tactical coverage with measured generation, storage, and loading costs, and adjacent scenes remain consistent.

### 16. Stream and optimize for the target devices

- Keep the existing goals of **60 fps on desktop**, **30 fps on a mid-range phone**, and **interaction latency at or below 200 ms** visible during development. Verify representative devices and dense scenes.
- Set separate measured budgets for texture downloads, decoded texture/GPU memory, geometry, scene payloads, load latency, and generation. The existing one-megabyte compressed play-code budget does not establish an artwork budget.
- Load nearby chunks and needed asset families, prefetch likely travel, and evict distant resources. Use suitable texture levels, atlases/batching, and view culling as profiling warrants.
- Reduce decorative density, animation, or shadow expense on constrained devices while preserving the same movement/visibility geometry and stable meaningful objects.
- Test cold loads, revisits, long travel sessions, interrupted loading, and renderer recovery. Watch for memory growth and visible seams during streaming.
- **Complete when:** the benchmark and denser stress sites meet recorded device budgets during real interaction, not only while displaying a stationary screenshot.

### 17. Establish procedural, gameplay, and visual acceptance gates

- Maintain a fixed evaluation set covering the benchmark, forest, open ground, shoreline, a sloped site, a dense settlement, and boundary-spanning structures. Add adversarial fixtures for narrow doors, conflicting constraints, and difficult crossings.
- Compare the same location at world, area and tactical scales. Verify that settlement extents, road connections, crossings and building footprints agree within the declared display simplification, and that loading finer detail does not move an already visible feature.
- Verify that world/area exports work before interiors, furnishings or tactical artwork are generated or loaded. Generating that detail later must preserve the published layout; deliberate layout revisions must invalidate dependent cached views.
- Test invariants: deterministic generation, format round trips, border agreement, required route connectivity, valid doorways, clear surfaces, and matching visual/physical footprints.
- Track solver duration, retry/fallback rates, generation failures, object density, repetition, loading, memory, and frame time across many seeds. Select tolerances from observed performance and quality requirements.
- Review galleries at matched scale and zoom against the visual target. Judge composition, material coherence, natural transitions, believable furnishing, repetition, and tactical readability separately.
- Verify semantic determinism with data tests; use controlled rendering conditions and image tolerances for visual regressions rather than promising bit-identical GPU output everywhere. Preserve existing world-generation regression gates.
- **Complete when:** quality holds across an agreed representative seed set and actual play cases, rather than only one selected screenshot.

### 18. Expand the supported world deliberately

- Add new location families one at a time: woodland encounters, farms, villages, town streets, ruins, caves, and other environments required by the product.
- For each family, add its world-selection rules, layout grammar, gameplay geometry, asset kit, constraints, furnishings, boundary transitions, and acceptance scenes together.
- Vary regional materials, climate effects, building forms, density, and vegetation coherently. Avoid applying the same warehouse and tree kit to every biome.
- Introduce weather, seasonal looks, richer lighting, destructibility, or additional floors only when their data, rendering, and gameplay requirements are supported. They are optional later scope, not prerequisites for the supplied reference's static look.
- Document the actual supported locations and known visual limits. Expand the regression gallery and performance cases with each new family.
- **Complete when:** all location families required for the intended release can be generated and played at the accepted quality without relying on a handpicked exception.

## Delivery order and milestones

The numbered steps express dependencies, but do not require one strictly serial implementation pass. In particular, asset production and world-context work can proceed alongside scene-contract work once their shared scale/camera decisions are stable.

| Milestone | Required result | Main steps |
|---|---|---|
| W — detailed inhabited world | Shared settlement outlines and main roads appear over a substantially richer world render; remaining generation issues are assessed separately | 1, 3, 6–7, world portions of 12/13/17 |
| R — detailed area maps | Building outlines and smaller roads/paths appear in area views and agree with the world layout; interiors are not required to render them | 3, 7, area portions of 12/13/17 |
| A — visual proof | One authored, interactive riverside location assembled from reusable assets; art, camera, and geometry agree | 1–5, minimal portions of 12/14 |
| B — generated proof | The generator builds that same class of location from real world context with multiple seeds and at least one neighbor/boundary case; world, area and tactical views show consistent shared geometry at the confirmed detail levels | 6–12, initial 13/17 |
| C — playable integration | Saved/revisited sites support actual DiceMaster movement, visibility, fog, and campaign changes | 13–14 |
| D — reliable coverage | The chosen coverage strategy streams larger areas within measured device and storage budgets | 15–17 |
| E — world variety | Each release-required environment passes the same quality and gameplay bar | 18, repeating 17 |

**Recommended code delivery order:** shared coordinates/contracts → ecology, settlements, land use and regional roads → shared street/building layout and finalized settlement extents → detailed inhabited world render (W) → detailed area render (R) and society binding → full tactical generation and integration (B–E). World relief-rendering development can start on existing physical data; publishing stable settlement outlines requires the finalized shared plan. The small tactical asset/rendering proof (A) can run alongside the earlier work to establish its visual quality early. Begin that proof with one riverside warehouse/customs-house family; complete politics, every biome, dynamic weather and every building type are not prerequisites.

**The success condition:** given a supported saved-world location and pinned generation/assets versions, the system produces a believable illustrated tactical environment, preserves the same meaningful layout on revisit, joins its neighbors, and behaves as it looks during play.

## Evidence and design boundaries

- Current status: [Arda open items](capstone/open-items.md), especially the current-status table; historical entries below it are not treated as present implementation claims.
- Current tactical code: [coordinates](../crates/arda-core/src/coords.rs), [tiles](../crates/arda-core/src/tiles.rs), [constraints](../crates/arda-gen/src/block/constraints.rs), [WFC](../crates/arda-gen/src/block/wfc.rs), and [JSON export](../crates/arda-render/src/json.rs).
- Retained tactical intent: [block-generation design](capstone/logic/03-block-generation.md). Its 200+ vocabulary, rich constraints, object layer, and building layout describe outstanding work; 200 is not a demonstrated visual-quality threshold.
- Renderer boundary and budgets: [DiceMaster architecture](../../dicemaster/capstone/01-architecture.md), map-renderer and budgets paragraphs.
- Existing contract gap: [DiceMaster interfaces](../../dicemaster/capstone/09-interfaces.md), block payload and elevation notes. These interfaces are prescriptive, not proof of an implemented service.
- Visual mismatch: [DiceMaster play-screen design](../../dicemaster/capstone/uiux/screens/05-play-screen.md), tactical-map paragraph.
- Gameplay geometry intent: [DiceMaster spatial resolution](../../dicemaster/capstone/logic/10-vtt-spatial-resolution.md), verticality, visibility, and scale sections. This roadmap does not re-audit its tabletop rules.

No production code, approved plan, or DiceMaster specification was changed while writing this roadmap. Resolving the identified specification conflicts and choosing the undecided architecture are work items within the roadmap.
