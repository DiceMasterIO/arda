---
generated_date: 2026-09-25
generated_at_commit: 14a70b9144dd
content_hash: 56d24d54ae68
paths_covered:
  - ":(top)crates/**"
  - ":(top)tests/**"
  - ":(top).github/workflows/**"
  - ":(top)Cargo.toml"
  - ":(top)Dockerfile"
  - ":(top)rust-toolchain.toml"
absorbed_from:
  - features/2026-09-23-terrain-corrections@2026-09-23
  - features/2026-09-07-area-water-terrain-realism@2026-09-08
  - features/2026-09-22-geographical-rendering-first-pass@2026-09-22
---

# Implementation status and open items

## Current status — accepted fine-terrain look delivered at 32K


**Saved fine terrain is now connected to normal Atlas exports for opt-in worlds.** Area and overview rendering use bounded fine windows, physical slopes and source coordinates shared across area seams; legacy worlds keep their existing path. Recipe 2 removes the hard lowland gain cutoff and preserves the forced-ocean rim. A corrected seed-42 102×204 km world generated in 22.6 s source + 26.7 s world time and repeats exactly across all 35 files. All saved heights match the canonical field, the annual water ledger closes, and all 523 original-world files remain unchanged. Current checks passed 250 core/render/facade tests plus 15 focused generation tests, all-target workspace Clippy and formatting. [Actual fine exports and verification](features/2026-09-23-terrain-corrections/evidence/fine-atlas-integration/README.md).

**The user has accepted the current appearance:** “I love it” and “Not much more is needed,” referring to the actual recipe-2 fine Atlas area render. Preserve that source and palette. Further wetness/tectonic appearance work is deferred; those documented limitations do not override the user’s acceptance. Additional seeds 43 and 44 at 102×204 km pass exact saved-height, receiving-link, global-table and annual-water checks and render with the same style. The full 500×1000 km seed-42 source/world/32K export is complete: 15,522×32,768 pixels, 259,955,341 bytes. Source took 6m 47s, world generation 20m 24s, verification 2.6 s and rendering 3m 13s: 30m 26s combined. All 44,826,624 exported cell heights and 500,000 continent heights match the fine field; the annual water budget closes exactly. The full-map preview and mountain area (5,4) were inspected. Production command access is implemented as `generate --terrain fine` and reproduces all 35 accepted small-world files exactly. The accepted-look delivery is complete; exact reference-image parity is not claimed. [Accepted-look qualification](features/2026-09-23-terrain-corrections/evidence/accepted-look-qualification/README.md).

Remaining differences from the reference include broad uniform plains, long straight river runs, dense small highland lakes, coarse categorical water outlines and locally rectangular outer coasts. The wetness formula discrepancy remains recorded for future correctness work; it is not being used to restart visual experiments.

## Earlier milestone log

The entries below record the state at each earlier experiment; the current status above supersedes their then-open implementation items.

**First complete world using the preferred source is generated (opt-in):** the whole-domain integer source now has bounded admission, modeled-fringe coverage, streaming persistence and a manifest descriptor. `generate_world_from_fine_terrain` derives both the 1 km climate/drainage grid and the 100 m prepared bed from that same file, bypassing legacy detail and evolution. Seed 42 attempt 0 at 102×204 km completed source generation in 23.3 s and world generation in 26.4 s, publishing eight areas and complete hydrology. All five 64×64 km candidates failed the unchanged major-river gate; no threshold was relaxed. The existing 160-step erosion was tested once and rejected for this source: it recreates broad smooth faces and repeated deep gullies (maximum change 998 m). Fresh-process replay matches all 35 world files; all 2,097,152 saved cell heights and 20,808 continent heights match the fine file. Saved drainage tables validate and the annual ledger closes exactly. Workspace tests passed 730 with 8 ignored; final focused checks and all-target workspace Clippy passed. Normal CLI generation remains on the existing path. Fine Atlas export, the inherited gate’s suppression of lowland detail below 400 m regional range (identified by exact source/saved-height replay), preservation of the macro ocean rim (positive fine samples occur on three source edges), multiple-world drainage/appearance qualification and final 500×1000 km seed-42 32K delivery remain open. [Complete-world evidence](features/2026-09-23-terrain-corrections/evidence/fine-world-integration/README.md), [erosion handoff](features/2026-09-23-terrain-corrections/evidence/spectral-evolution-handoff/README.md).

**Fine source persistence and world seed identity are implemented:** a versioned streaming terrain codec reopens the inland/coastal fields within an 80 KiB reader-buffer allowance and reproduces all four render outputs byte-for-byte. The full 64-bit world seed and attempt now feed Arda’s existing deterministic RNG path; five fixed cases repeat exactly and match the filter oracle within 1 mm. All 104 core tests and 20 focused generation tests pass. Normal world generation has not yet been switched over: source-stage admission, fine/evolved terrain authority, coast/climate/drainage and normal manifest/export wiring remain before the small complete-world run and final 32K delivery. [Persisted terrain evidence](features/2026-09-23-terrain-corrections/evidence/fine-terrain-persistence/README.md), [world-seed evidence](features/2026-09-23-terrain-corrections/evidence/world-spectral-seed/README.md).

**Preferred terrain now composes with existing mountain belts and lowlands:** new integer composition helpers reuse the existing local-relief gate continuously across sea level. Inland and coastal 80 km diagnostics repeat exactly and match the independent integer oracle byte-for-byte; actual Arda views show rugged mountain flanks, quieter lowlands and irregular coast. Eighteen focused spectral/composition tests, all-target gen Clippy, workspace formatting/compilation pass. The normal world path remains unchanged: resource admission, full-world seed/coverage, fine persistence and the fine-versus-evolved 100 m drainage authority must be integrated before final 32K delivery. [Implemented composition, views and limits](features/2026-09-23-terrain-corrections/evidence/spectral-macro-composition/README.md).

**Preferred procedural source now retains physical relief across larger domains:** a fixed 20 km wavelength bound and one reference amplitude replace per-domain normalization in the new signed-relief API. Five fixed cases across 20/40/80 km and seeds 42–44 repeat exactly and agree with an independent oracle within 1 mm. Seed-42 RMS grade stays 0.464/0.445/0.438 instead of falling to 0.464/0.238/0.099. Actual Arda views retain local ruggedness; complete mountain-belt/lowland/coast composition and drainage remain unresolved. Thirteen focused spectral tests, all-target gen Clippy, workspace formatting/compilation pass. This is outside normal world generation; fine persistence and final 32K delivery are still open. [Implementation, comparisons and limits](features/2026-09-23-terrain-corrections/evidence/physical-spectral-scale/README.md).

**Preferred procedural source ported to integer Rust — appearance reproduced:** repository-owned MT19937 phase generation, fixed-point cosine and a Q60 Fourier filter now generate the selected source from a seed. Three 512² sources plus a 1024² numerical check match the native saved height fields within 1 mm, and all five fixed cases (including a declared rectangular extension) repeat exactly. Actual Arda renders retain the preferred look. Nine spectral tests, gen all-target Clippy, workspace formatting/compilation and regenerated-table checks pass. Normal world generation, full 64-bit seed mapping, physical-domain/coast composition, fine persistence, drainage and final 32K delivery remain open. [Source implementation and rendered evidence](features/2026-09-23-terrain-corrections/evidence/integer-spectral-source/README.md).

**Canonical fine-field sampling implemented — preferred appearance preserved:** `arda-core::TerrainField` now owns fine millimetre heights and samples absolute coordinates using integer bilinear interpolation. A real diagnostic consumer verifies matching points across resolutions, overlapping windows and fresh processes for all three preferred-source seeds. Full Arda images remain pixel-identical to the controls; close views differ by at most 1 mm in sampled height. All 100 core tests, core all-target Clippy, workspace formatting and compilation pass. This is a working sampling primitive, not yet a world-generation, persistence or normal-export integration. Full-domain source/coast/drainage and the final 32K world remain open. [Implementation, actual images and limits](features/2026-09-23-terrain-corrections/evidence/canonical-fine-field/README.md).

**Preferred procedural source qualified locally across three seeds:** seeds 42, 43 and 44 reproduce exactly in fresh processes and retain distinct rugged masses in actual Arda views. Seed 42 is the user-liked input. Two delivery constraints are now measured: independently regenerating at a different resolution relocates mountains (1,183 m shared-coordinate RMSE), and a 100 m storage round trip visibly loses fine detail (8.09% RMS-grade reduction). Perimeter spill screens also identify unresolved basins; no world drainage is implied. Preserve this visual direction while implementing one consistent field and fine-detail retention. All 523 original world files and the earlier baseline remain unchanged. No production integration or new full-world/32K result. [Images, measurements and next implementation direction](features/2026-09-23-terrain-corrections/evidence/procedural-source-qualification/README.md).

**Latest user direction — prefer the procedural input; increase mountain grandeur:** after the particle comparison, the user calls the image "amazing" and explicitly prefers the procedural input to the corrected erosion result. Use that input as the next visual baseline; preserve its rugged detail. The previous agent assessment does not override this preference. A single matched +50% vertical-relief preview is specified to test whether height improves the daunting sense of scale. This is a visual experiment, not production acceptance; world integration and drainage remain open. [Feedback and comparison design](features/2026-09-23-terrain-corrections/evidence/procedural-relief-scale/README.md).

**Particle formation control completed — smoother mounds, appearance gate failed:** a first-step check rules out the unchanged hydraulic demo as a physical reference because it produces negative sediment inventory before slippage. A separately reviewed particle operator has a velocity-sign inconsistency in both released code and its cited thesis. One isolated serial correction, with native coefficients and a fixed procedural Fourier input, passes finite-state and fresh-run equality checks. Actual same-scale Arda views retain the input's broad mounds, remove fine texture and add short directional cuts rather than the required grand rugged mountain structure. Roughly 88.7% of termination sediment is discarded at the lifetime limit inside the field, a declared nonphysical sink. Close this fixed trial without tuning or production adoption; retain the preferred rugged baseline and all world-delivery requirements. [Actual comparison and accounting](features/2026-09-23-terrain-corrections/evidence/particle-formation-control/README.md), [reference preflight](features/2026-09-23-terrain-corrections/evidence/hydraulic-construction-audit/README.md).

**Dendry native fixture completed — appearance gate failed:** the exact pinned third teaser runs twice in 9.33/9.41 seconds with byte-identical 1024² fields. Actual Arda full/detail views show branching corridors but repeated wedge-shaped masses, broad smooth faces and insufficient rugged detail. A fixed grid-boundary screen detects no material finite jump; large faces are not explained by that screen as sampling seams. The perimeter basin screen finds material enclosed relief, with no declared ocean/outlet semantics. The released bool-valued control-range quirk is retained and disclosed. Close the exact fixture without tuning, extra erosion or production adoption. The preferred rugged baseline remains the benchmark; all subsequent integration/world gates remain open. [Actual renders, audit and decision](features/2026-09-23-terrain-corrections/evidence/dendry-author-control/README.md).

**Independent river-first construction completed — not adopted:** the unmodified pinned author example runs twice in about 17 seconds with byte-identical native and captured outputs. Actual Arda renders show branching valleys but persistent comb ribs and blocky terraces. Final heights reverse 3,543 of 56,684 assigned river links; a permissive raster screen finds 2,528 land pixels requiring more than 1 m filling at the declared display scale. Close this candidate without extra erosion or a parameter sweep. The user's criterion remains reduced repetition with preserved rugged detail; the preferred completed baseline remains the visual benchmark. The old experimental graph is replaceable, but the original saved world is immutable. No production adoption or accepted appearance improvement. [Actual renders, audit and decision](features/2026-09-23-terrain-corrections/evidence/river-network-author-control/README.md).

**Secondary-valley surface test completed — stopped before erosion:** after the source topology repair, the same fixed-box route rule finds a 2.409 km path entirely within the central face. Maximum proposed chord lowering falls from 809 m to 178 m, and the downstream ridge crossing is gone. One parameter-free harmonic displacement test preserves all existing ridge/river/boundary heights and repeats exactly in fresh processes. However, only two of nine unobstructed transverse sections place the axis below both flanks, the same count as before; actual matched Arda views retain the broad sheet face. This is a lowered slope, not a verified continuous valley. No erosion continuation, depth/width sweep or production adoption. The source-blend audit independently finds the central/lower broad faces already present before smoothing; the next formation method must couple valley-floor and flank geometry rather than merely inserting a downhill line. [Actual renders, measurements and decision](features/2026-09-23-terrain-corrections/evidence/guarded-secondary-valley-surface/README.md), [source diagnosis](features/2026-09-23-terrain-corrections/evidence/source-blend-mechanism/README.md).

**Source topology correction — reconstructed and rendered; no broad-fold improvement:** the saved experimental mesh omits 287 of 6,611 prescribed ridge/river edges, and its fine control paths have 18 incompatible crossings despite a valid coarse backbone. Exact replay traces the crop's final 1,129 m ridge–river conflict to ridge perturbation. The deterministic geometry guard retains 357 of 364 parent paths and substitutes seven straight fallbacks, preserving fixed junctions and network values. A new constrained mesh now preserves all 6,611 segments and linear heights on 1,474 added protected vertices. Two fresh runs produce identical arrays and receipts; independent numerical review passes. Matched actual Arda renders reproduce the original controls exactly and show a local corrected ridge–valley junction, while the broad sheet-like mountain faces remain. Retain the correctness fix as an experimental foundation; do not select it as the broad-fold solution or start erosion from this visual result alone. No production change; the preferred rugged baseline and all 523 original world files remain unchanged. [Actual before-erosion comparison and decision](features/2026-09-23-terrain-corrections/evidence/topology-preserving-refinement-control/reconstruction/root-visual-review.md), [independent numerical review](features/2026-09-23-terrain-corrections/evidence/topology-preserving-refinement-control/reconstruction/independent-review.md).

**Source structure audit completed — no new erosion candidate:** the fixed mountain crop has a real branched river network, but the marked central sheet has little explicit secondary valley geometry. A separate audit finds 763 of 6,548 original control connections crossing the opposite ridge/river constraints. Exhaustive nearest-visible replacements are incomplete (12 ridge sites have none) and their material height effects largely miss the marked broad faces. Exact original mesh replay passes; after unchanged interpolation and smoothing, only four of 81 central-face vertices change by more than 10 m. Root and independent review stop this partial correction before rasterization or erosion. The preferred rugged baseline remains preserved; secondary valley structure requires graph feasibility before another terrain trial. No production change or improved-render claim. [Evidence, unchanged-render overlays and decision](features/2026-09-23-terrain-corrections/evidence/tributary-structure-audit/README.md).

**Latest user preference — preserve the rugged baseline:** the user calls the preserved completed baseline “really cool” and the new complete erosion recipe “a bit like plastic.” Neither is final. The preserved baseline is the preferred visual direction; root withdraws the earlier stronger-candidate ranking for the new recipe. The user explicitly clarifies: reduce the repetition, but preserve the rugged detail. Its reduced comb gullies also remove too much valued detail. Retain the recipe comparison as diagnostic evidence and target specific defects while preserving rugged texture and mass. Both experimental runs completed in about 41 seconds with exact replay; that numerical success does not override the user's visual preference. [Comparison and updated interpretation](features/2026-09-23-terrain-corrections/evidence/complete-erosion-transfer/README.md), [recorded feedback](features/2026-09-23-terrain-corrections/evidence/formation-structure-review/user-feedback-baseline.md).

**Gully audit and fixed ordering comparison completed — appearance gate failed:** an exact 1024 replay rules out receiver-floor clipping in that run (zero events) and finds strong early flow lag with little instantaneous late effect. The released method deliberately uses that lag; it is not a demonstrated CPU bug or proven comb cause. A fixed comparison then redistributed the same 2000 thermal steps through the same 700 erosion steps, followed by the original 200 deposition steps. It preserves rugged contrast but leaves the parallel grooves and sometimes sharpens them. Root and independent review close it at 1024 without a cadence sweep. The original control reproduces all saved endpoints; both candidate runs reproduce fields and numerical receipts exactly (88.60/88.61 seconds). All operator ledgers close, all requested thermal sweeps execute, and all 523 original-world files remain unchanged. The preferred baseline remains the visual direction. [Actual comparison and result](features/2026-09-23-terrain-corrections/evidence/interleaved-thermal-control/README.md), [preceding research and replay](features/2026-09-23-terrain-corrections/evidence/gully-formation-audit/README.md). The underlying slope/tributary structure remains unresolved; no new terrain variant is selected by this result alone.

**Original-budget material comparison completed — no continuation:** the same saved material field, scaled once to match the baseline's initial removal, preserves much more ruggedness than the earlier lower-budget treatment. It still leaves prominent parallel grooves and repeated ribs. Root and independent visual review decline the conditional 2048 continuation; the preferred baseline remains unchanged. Two fresh runs complete in 176.74/176.09 seconds with exact seven-field replay, original baseline parity and unchanged fixed rims. All 523 original world files retain their hashes. The missing original-strength comparator is now closed without another field/strength/scale sweep. [Actual comparison and result](features/2026-09-23-terrain-corrections/evidence/material-budget-control/README.md), [independent visual review](features/2026-09-23-terrain-corrections/evidence/material-budget-control/final-visual-review.md).

**Broad-face localization completed:** fixed visible cliff-edge lines are mostly adjacent to original ridge constraints, but that association does not prove the ridges are defective. An exact mesh replay recovered a real mismatch between the river heights used for initial slope interpolation and river-constraint heights. Its direct effect is small over the crop (median 0.488 m; only 4 of 6,548 Poisson points exceed 100 m), so it is not selected as a broad-fold fix. Prior smoothing statistics change little when corrected. The paper's omitted post-mesh noise is only qualitatively bounded; no exact unpublished erosion recipe can be claimed. [Localized evidence and limits](features/2026-09-23-terrain-corrections/evidence/fold-control-localization/README.md), [independent review](features/2026-09-23-terrain-corrections/evidence/fold-control-localization/independent-review.md).

**Storage-resolution limit verified:** the current experimental 2048 field spans 20 km (9.77 m samples), while saved Arda cells are 100 m. One fixed area-mean storage/reconstruction test preserves mountain masses and main valleys but loses many narrow gullies and ribs; broad folds persist. Fine experimental images therefore do not prove that this look survives the existing saved-world representation. Retaining finer terrain information is an integration requirement. This is a height-storage sensitivity test, not an exact preview of native production shading. [Actual detail comparison](features/2026-09-23-terrain-corrections/evidence/native-height-sensitivity/renders/comparison-detail.jpg), [method and limits](features/2026-09-23-terrain-corrections/evidence/native-height-sensitivity/assessment.md).

**Latest implementation and evidence (September 24):** the matched material-resistance comparison is complete through 2048. A subtle interruption of grooves survives relative to uniform resistance, but both reduced-strength treatments lose some fine incision compared with the preserved baseline; broad folded faces remain. Root and independent final review find no clear gain sufficient to select this frozen material field. Close this candidate without a strength/wavelength sweep. Frozen-input, finite/rim and original-render parity checks pass; all 523 original saved-world files retain their hashes. [Completed actual comparison](features/2026-09-23-terrain-corrections/evidence/material-incision-control/continuation-renders/comparison-deposition-overview.jpg), [result and limits](features/2026-09-23-terrain-corrections/evidence/material-incision-control/README.md), [independent review](features/2026-09-23-terrain-corrections/evidence/material-incision-control/continuation-independent-review.md).

**Complete author pipeline qualification finished:** the fixed 256→512→1024 sequence completes in 17.32/17.27 seconds with exact fresh-process field replay. It materially improves the raw surface with branching valleys and rougher faces. Root and independent review still find a central cliff/shelf ring, repetitive gullies on some slopes and abrupt coastal cuts; this authored fixture does not pass production appearance acceptance. All 201 frozen artifacts and 523 original saved-world files remain unchanged. The thermal coordinate-upload API correction is explicitly disclosed, including its changed noise phase versus the broken original upload. No coefficient, source-contour or noise sweep follows automatically. The user-liked corrected-scale baseline is preserved. [Actual comparison and limits](features/2026-09-23-terrain-corrections/evidence/iso-contour-complete-pipeline/README.md), [independent review](features/2026-09-23-terrain-corrections/evidence/iso-contour-complete-pipeline/final-review.md).

**Contour construction qualification complete:** the pinned author's non-DEM Figure 2 example runs in about 2.217 seconds and repeats graph/contour/mask/DEM bytes in fresh processes. A diagnosed RNG compiler issue was corrected with explicit signed wraparound; the failed build remains preserved. Actual Arda whole/central views still show broad polygonal terraces and a central mound. Root and independent review close this particular raw fixture without amplification or a contour/noise sweep; this does not reject the paper's full eroded method. No production adoption or original-world write. [Actual view and result](features/2026-09-23-terrain-corrections/evidence/iso-contour-author-control/README.md), [independent review](features/2026-09-23-terrain-corrections/evidence/iso-contour-author-control/final-review.md).

**Latest user feedback (September 24):** the user finds the last posted image “kind of nice.” Preserve the corrected-scale comparison as a visual baseline worth improving. The earlier appearance rejection means the experiment has not met the full reference-quality acceptance criteria; it does not mean the image has no visual merit. Further changes must be compared against this baseline for lost mountain mass and appeal as well as reduced repetition. [Feedback and preserved baseline](features/2026-09-23-terrain-corrections/evidence/formation-structure-review/user-feedback-baseline.md).

**Previous full-resolution experiment:** the reference-quality mountain goal remains unmet. The corrected20km direct-mesh control completed all256→2048 stages in about17minutes and was visually rejected: broad folded faces and dense parallel gullies remain. No postprocessing or production integration followed. Retained operator views show erosion amplifying the ribs; later smoothing leaves them visible. A bounded comparison with actual released shaders weakens a gross CPU arithmetic-error explanation, while long-run parity remains unproven. The paper’s optional varying-hardness control is absent from the release, but a fixed orientation check chiefly finds gullies following the input slopes; it does not justify an automatic noise patch. The author map is20km wide, starting at78m spacing; prior10km/39m/~165× claims were corrected to20km/78m/~41×. [Final actual comparison](features/2026-09-23-terrain-corrections/evidence/physical-scale-hierarchy-control/renders/comparison-overview.jpg), [result and limits](features/2026-09-23-terrain-corrections/evidence/physical-scale-hierarchy-control/README.md).

**Appearance requirement:** the user requires mountain grandeur as well as recognizable mountain shapes: higher summits must visibly tower above valleys and foothills, with readable differences in height and mass in the delivered map view. Taller folded walls or snow colour alone do not meet it. [Acceptance criteria](features/2026-09-23-terrain-corrections/terrain-acceptance.md).

**September 24 coastal curved-surface control — rendered and rejected:** the same constrained coastal mesh and all vertex heights are sampled with linear versus geometric cubic interpolation. The linear100m field is byte-exact to the retained original. Actual neutral Atlas shows fewer literal triangle facets, but rounded masses remain and central depression area increases17.75→60.81km² (volume35.91→271.38millionm³). Independent review confirms retention and measured overshoots. No160-step evolution, production adoption or interpolation sweep follows. [Actual paired render](features/2026-09-23-terrain-corrections/evidence/coastal-curved-surface-control/run/atlas-pair.png), [assessment](features/2026-09-23-terrain-corrections/evidence/coastal-curved-surface-control/root-assessment.md).

**September 24 analytical-start flow qualification — failed path independently verified:** the first predictor/corrector step converged at transport fraction 0.0000329776. The full bounded path then failed at its per-root retry cap: 78 accepted stages, 93 attempts, 398.37 seconds, terminal fraction 0.012977557. Independent review verifies all 422 retained Newton states, intermediate residuals, joins and terminal stopping reason. For 71 tiny-step transitions, exact sparse-solve replay and bitwise forward reconstruction replace unreliable inverse subtraction. This verifies the recorded failure, not full physical flow. No erosion or render follows; the numerical branch is closed. [Independent path review](features/2026-09-23-terrain-corrections/evidence/flow-homotopy-control/independent-path-review.md).

**September 24 branch-aware flow control — verified plateau failure:** the tested derivative-only donor selector escapes the original reversal and releases its temporary constraints by accepted step 15, but the run stops at its declared fifty-step double plateau (53 trials, 345.02 s). Raw scaled momentum L2 remains 46.06356 and maximum independent map error 0.190054 m/s; both gates fail. All 51 saved states, 53 decisions and 50 accepted endpoint face checks independently verify; three rejected endpoints were not retained. The fifty-step raw/map declines are only 0.2946%/0.6779%. Retained diagnostics distinguish wet opposed-flow map-error pockets from separate dry steep raw-error cells. No further cap extension, terrain update, new render or production adoption. An analytical zero-transport start and tangent have been evaluated for a separately reviewed numerical-continuation preflight; the start matches the original initial velocity to roundoff. The first positive-transport step now converges and independently verifies; the bounded path subsequently failed before full transport. Intermediate states cannot drive erosion. [Result and limits](features/2026-09-23-terrain-corrections/evidence/flow-branch-aware/root-assessment.md), [independent terminal review](features/2026-09-23-terrain-corrections/evidence/flow-branch-aware/terminal-review.md).

**September 24 flow continuation and matched Krylov control — completed, unqualified:** the exact-state dogleg continuation stops at 500 additional accepted steps/614 trials in 1463.77 s. Raw scaled momentum L2 is 174.28 (required 1.28e-8), with maximum velocity-map error 0.275 m/s (required 0.01); all 501 saved states and 614 decisions independently verify. The early convergence-rate forecast did not persist. A matched projected-CG control gives a much larger initial reduction but stalls after roughly 20 steps and ends at its 60-step cap: raw L2 46.20, maximum map error 0.190 m/s, 295.83 s/100 trials. Its 61 states and 100 decisions independently verify. Neither run qualifies flow, erosion or a new terrain render. [Continuation review](features/2026-09-23-terrain-corrections/evidence/flow-map-continuation/independent-postrun-review.md), [Krylov review](features/2026-09-23-terrain-corrections/evidence/flow-krylov-control/independent-post-run.md).

**September 24 Krylov stall diagnosis:** one upwind face reverses 19 times while its flux approaches zero; the trust radius collapses to 1.66e-12 although a fixed-endpoint directional check still finds descent. A single predeclared face-tangent comparison preserves that face exactly, but its large trial still increases the objective and crosses a second face. Both ordinary and constrained proposals fail acceptance; independent replay confirms them. Exact saved-trial decomposition instead attributes about 96.2% of squared model error to one cell's nonlinear water/drag response; the second face's direct contribution is negligible. No smaller-radius sweep or new trajectory follows. This identifies a branch-switching obstacle in the stalled run without establishing a general correction or blaming terrain shape. A general treatment of switching faces remains open before the conditional sediment/bed/render handoff can run. [Diagnosis](features/2026-09-23-terrain-corrections/evidence/flow-krylov-control/stall-diagnosis.md), [local comparison design](features/2026-09-23-terrain-corrections/evidence/flow-krylov-control/FACE-TANGENT-DESIGN.md).

**September 24 pocket diagnosis and velocity-map control:** the remaining fast interior pockets show near-cancellation of opposing cell velocities at shared faces and strongly unequal water/momentum response to common versus opposing perturbations. A tested nonlinear velocity-map objective preserves the physical equations and completes a fresh 60-step control in 178.61 s/83 trials, but still fails convergence. Maximum map discrepancy improves 20.43→1.407 m/s while raw scaled momentum L2 worsens 90.90→1924.12; cells exceeding 0.01 m/s increase 452→12,971. Independent checks reconcile all 61 saved states and 83 decisions; water balances. This is a localized-versus-distributed error tradeoff, not qualified flow or better terrain. No terrain update, new render or production adoption. Late descent in both errors warrants assessing a separately bounded continuation, not changing physics or declaring impossibility from the step cap. [Evidence and limits](features/2026-09-23-terrain-corrections/evidence/flow-pocket-diagnosis/root-assessment.md), [connection to the terrain goal](features/2026-09-23-terrain-corrections/evidence/flow-pocket-diagnosis/terrain-path-audit.md).

**September 24 depth-aware flow step:** exact replay attributes over 99% of a retained rejected step's model-error energy to one interior depth collapse and its drag response; the remaining baseline error is distributed. Including log-depth response in the trust geometry passes a local comparison and improves the fresh 60-step run: scaled momentum L2 90.90 versus 12,735 for velocity-only control (140× lower), in 119.17 s/88 trials. Three tests, local replay and independent checks of all 61 saved states pass; water remains balanced. The run still fails its unchanged 1.28e-8 convergence target at the step cap. No flow acceptance, terrain update, new render or production change. [Attribution](features/2026-09-23-terrain-corrections/evidence/trust-rejection-attribution/root-assessment.md), [candidate evidence and limits](features/2026-09-23-terrain-corrections/evidence/tangent-trust-flow/root-assessment.md).

The independent old momentum-map check of that earlier endpoint changes velocity by up to **20.43 m/s** in localized interior pockets (452 cells exceed 0.01 m/s). This motivated the completed pocket diagnosis above; a low global force residual had not established local equilibrium. That one-map check itself ran no coupled iteration.

**September 24 bounded exact-water flow:** the reviewed trust-region solver rejects oversized moves and takes 60 accepted steps in 94 trials (90.67 s). Its scaled momentum L2 falls about 301-fold within the run, but remains 12,735 versus a 1.28e-8 target; the declared step cap ends the experiment. Four small tests, independent first-step replay and all 61 saved-state/94-trial checks pass. Water balances throughout, but final momentum relative L1 is 1.872, so no qualified flow, terrain update or new render. Late radius cycling needs explanation before another numerical change. This is not a claim of a better terminal approximation than the older failed outer iteration. [Evidence and comparison](features/2026-09-23-terrain-corrections/evidence/reduced-trust-flow/root-assessment.md).

**September 24 flow-stagnation witness:** the retained Newton step is accurately solved and its smooth-branch derivative checks pass, but one interior face reverses at alpha 9.0008e-10, just below the last allowed trial. Crossing raises merit; a smaller diagnostic step before the crossing descends. Both one-sided donor Newton directions are inconsistent with their departure sides, so a donor flip is not adopted. Exact water elimination closes water balance and passes a local coupled derivative check, but its predeclared full velocity trial is extreme (914 face reversals; momentum merit increases sharply). No reduced trajectory, terrain update or visual improvement. The next numerical intervention must control oversized steps and the witnessed reversal without changing physics or simply relaxing gates. [Evidence and independent review](features/2026-09-23-terrain-corrections/evidence/flow-newton-stagnation/root-assessment.md).

**September 24 simultaneous-flow preflight:** the exact original measured 128×128 fixed-bed case still fails with a full coupled water/momentum Jacobian: Armijo search stalls at iteration 23 (24.48 s), reducing the scaled residual only 2.815%. Five small-field tests and independent full-field residual/accounting checks pass, but the flow equilibrium does not. Positive-flow one- and two-cell controls are stable under the older iteration, so lagged depth alone is not the established cause. No terrain update or new visual result; no production adoption. Inspect the retained numerical failure before selecting another intervention. [Evidence and limits](features/2026-09-23-terrain-corrections/evidence/transport-mechanism-preflight/root-assessment.md).

**September 24 intrinsic hillside isolation:** completed analytic fixtures, independently checked sampled disk-minimum controls, and a 40-step hillside-only run on the actual irregular TIN (188.62 s). Exact benchmark replay, retained endpoint/material checks and initial natural-neighbor raster replay pass. Actual Atlas still forms broad smooth mountain faces without channels or diffusion; the constant-speed hillside law itself is implicated. The coupled run adds 319.8 m mean lowering, with 300.8 m in changed hillside response, 19.0 m additional channel loss and -0.0104 m signed diffusion. This accounting does not uniquely assign feedback to channels versus diffusion. **No natural-mountain/grandeur acceptance or production adoption.** A replacement must address the demonstrated front-retreat mechanism; another source/parameter sweep is not selected. [Actual images, verification and limits](features/2026-09-23-terrain-corrections/evidence/hillslope-shape-control/root-assessment.md).

**September 24 process-interface intervention:** the common-bed analysis identifies a strong masked-replacement contribution to crease sharpening. The isolated maximum-process candidate now completes all 40 matched measured-input steps in 251.48 s, with 56 tests, exact original benchmark replay, independent retained-operator/material checks and actual Atlas inspection. It removes many narrow repeated cuts but produces broad smooth lobes and weaker mountain mass; central peak is 2246 m versus binary 2652 m, from 3844 m initially. **Reject production adoption and grandeur acceptance.** This intervention removes the binary switch while increasing detachment; it is not an equal-loss isolation. Intrinsic hillside shape evolution and coupling remain unresolved. Production and original world are unchanged. [Actual result and limits](features/2026-09-23-terrain-corrections/evidence/envelope-retreat-control/root-assessment.md), [common-bed analysis](features/2026-09-23-terrain-corrections/evidence/process-interface-control/README.md).

**September 24 spatial shape attribution:** independently checked all four saved TIN snapshots and registered process contributions against actual Atlas. Hillside loss dominates the flattening projection; both hillside and channel loss contribute to new sharp creases. The exclusive process mask means this does not isolate the intrinsic hillside law or prove the cause of visible combs. This motivated the completed common-bed interface analysis and bounded intervention reported above. No new terrain evolution or visual improvement is claimed; production and original world remain unchanged. [Spatial evidence and limits](features/2026-09-23-terrain-corrections/evidence/facet-shape-attribution/root-assessment.md).

**September 24 temporal convergence control:**28/56/112 substeps on the same fixed-drainage interval all pass independent verification (196 substeps total;52 release tests). Selected finite crease-feedback oscillations shrink in typical amplitude with dt. Central final RMS differences decrease0.399→0.245m, but maximum differences grow14.39→22.79m, so full convergence is not established. Actual Atlas retains essentially the same broad forms; no natural-mountain or grandeur improvement is accepted. Physical formation remains unresolved, and production/original-world data are unchanged. [Actual comparison and limits](features/2026-09-23-terrain-corrections/evidence/domain-time-control/root-assessment.md).

**September 24 process-switch timing diagnostic:** tiny perturbations barely affect the tested classification, but finite terrain evolution makes it stale. A paired one-macro intervention holding drainage and coefficients fixed finds that refreshing the domain every substep increases central mean loss20.989→25.433m. All28 substeps pass independent checks, including exact original-step replay and first-substep equality. One borderline seed discrepancy was traced to the verifier's coordinate-conversion order; no process assignments or raw terrain changed. Actual Atlas shows similar broad forms, with no accepted mountain-grandeur improvement. Process switching and temporal convergence remain unresolved. [Actual comparison and causal limits](features/2026-09-23-terrain-corrections/evidence/process-switch-audit/root-assessment.md).

**September 24 two-process evolution diagnostic:** the geometric channel-domain/facet-retreat candidate and matched control both complete 40 steps with independent numerical checks passing. Actual Atlas shows fewer repeated incisions, but candidate ridges become smoother and lower: central peak 2652 m versus control 2814 m, from 3844 m initially. Median channel eligibility changes at 55.55% of mesh sites each macro step after the first; this domain rule is not established as physically stable. Do not adopt. The hillslope/channel transition, procedural mountain form and visible grandeur remain unresolved. Production and the saved world are unchanged. [Actual comparison and limits](features/2026-09-23-terrain-corrections/evidence/facet-retreat-control/root-assessment.md).

**September 24 process-domain preflight:** implemented and tested a geometric classifier and a standalone downhill facet-gradient kernel. Actual inclined planes on the irregular mesh expose a fatal flaw in the one-lower-neighbor classifier: downstream propagation falsely labels about31% of the central planar footprint. Reject it before evolution. A shared-facet valley-crease alternative passes both planes but is not a calibrated channel-initiation rule. The slope kernel passes an independent185-case oracle; its explicit stability bound requires at least28 substeps per current timestep. No new evolved terrain or production adoption. [Findings and implementation](features/2026-09-23-terrain-corrections/evidence/channel-domain-control/root-assessment.md).

**September 24 height-cue control:** cast shadows consistent with the current Atlas sun affect only82/262144 measured central cells and mainly deepen the synthetic folded ridges. Fixed surrounding terrain is included when computing visibility; a crop-halo omission was caught and fixed. This specific shadow pass does not meet the grandeur requirement. [Corrected comparison and limits](features/2026-09-23-terrain-corrections/evidence/terrain-shadow-control/README.md).

**September 24 measured-input TIN diagnostic:** natural Alpine geometry survives the mesh transfer with 6.54 m central RMS error, then develops increasingly repeated narrow cuts and smooth steep faces under the corrected zero-source erosion model. All 40 steps and retained numerical checks pass; the central peak falls 3844→2802 m while slope p99 rises 45.2→55.0 degrees. Loss of height is expected with zero uplift; the visual change and widespread low-area incision support reviewing hillslope/channel treatment, not another source-pattern sweep. Fixed-rim material import and uncalibrated observation times limit interpretation. No production adoption; procedural realism and mountain grandeur remain open. [Actual images, census and limits](features/2026-09-23-terrain-corrections/evidence/measured-tin-survival/root-assessment.md).

**September 24 source/incision coupling:** a demonstrated timestep bias is corrected in the isolated TIN: the old uplift-then-exponential update suppressed fixed-route equilibrium slopes. The old-mode400-step replay is exact; the opt-in backward-Euler candidate passes numerical checks and independent review. Actual matched renders still show taller folded walls and repeated gullies, so reject appearance adoption. Peak4856→7672m; native geometric depression capacity0.077664→0.0000285km³, with raster depth still70.6m. Retain the numerical correction for subsequent diagnostics; mountain-form realism remains unresolved. No production or saved-world change. [Actual result and limits](features/2026-09-23-terrain-corrections/evidence/coupled-incision-control/root-assessment.md).

**September 24 visual rejection and model reassessment:** the user describes the latest experimental mountains as folded paper. Stop the source-variant sequence; do not automatically run the remaining-7-km removal. Re-inspected actual comparisons show that unchanged Atlas expresses more convincing structure with measured terrain, while the production erosion control damages that structure. The experimental TIN is a different model, so its specific cause remains unresolved. Terrain formation/preservation needs reassessment before further source-pattern or surface-detail variations. No new simulation or production adoption; the visual target remains unmet. [Evidence and next acceptance conditions](features/2026-09-23-terrain-corrections/evidence/terrain-model-reassessment/README.md).

**September 24 source-scale comparison:** removing the persistent 3 km and 1 km uplift terms completes a 400-step run but leaves the large cliff bands, with longer and smoother faces. The fine mesh and broad mass source remain fixed. Root completed alternate-formula source checks, physical saved-field checks and actual matched renders inline after Sol agents reached their usage limit. Central area above 2 km stays 70.915→71.116 km² and raster slope p99 remains about 69.6 degrees. No production adoption. The remaining 7 km source contributes 19.1093% of integrated uplift; its separate effect is unresolved, and another source variant is no longer the selected next step. [Actual comparison and limits](features/2026-09-23-terrain-corrections/evidence/coarse-source-control/root-assessment.md).

**September24 mountain-mass source:** the fixed equal-total source redistribution completes400 steps with independent source/bed/hull/budget/diffusion checks. Actual Atlas and native group crops show stronger broad high/low relief, but taller repeated cliff benches and smooth faces remain. Central native area above2km rises0.421→70.91km² while raster slopep99 rises56.62→69.60degrees and geometric basin capacity1.972→2.781km³. Retain the mass-allocation finding; this exact candidate is not adopted. Production and original saved world are unchanged, and the full reference objective remains open. [Actual result and images](features/2026-09-23-terrain-corrections/evidence/mountain-mass-source-control/root-assessment.md). The subsequent coarse-only comparison above tests persistent3/1km source forcing (12.38% integrated source share) and fails to resolve the repeated faces.

**September 24 varying ridge direction:** the exact retained-source replay and both 592,896-site, 400-step terrain runs complete with independent source/bed/hull/budget/diffusion checks passing. Actual matched Atlas and native crops show more varied directions, but repeated cliff benches and smooth faces remain. Global peak stays about2.32 km, and central native area above2 km falls1.124→0.421 km². Retain the directional finding; neither candidate is adopted. Broad source allocation barely changes (1.20% redistribution at7 km, 0.31% at24 km), leaving mountain-mass hierarchy unresolved. Production and the saved world are unchanged; the complete reference target is open. [Actual result and comparison](features/2026-09-23-terrain-corrections/evidence/spatial-direction-source-control/root-assessment.md).

**September 24 rock-colour control:** an isolated world-coordinate albedo layer passes114 renderer tests, exact old/off PNG and candidate-repeat checks, independent coordinate/mask review and actual area-join checks. It changes about29% of pixels by at most11 RGB levels, but root inspection finds mottling on the same smooth repeated faces, rather than convincing fractured rock. Reject this setting for production adoption. All523 saved-world files and saved lake pixels remain exact. This tests material colour separately from the earlier normal-only probe; neither result settles all possible detail methods. [Actual images and assessment](features/2026-09-23-terrain-corrections/evidence/rock-albedo-control/root-assessment.md). A subsequent graph-only check verifies spanning ridge and complementary valley trees at all three retained scales, but the plotted source remains broadly parallel. No fine source field or terrain evolution follows from that check. [Graph result](features/2026-09-23-terrain-corrections/evidence/ridge-tree-source-control/root-assessment.md).

**September 24 low-area colluvial control:** one fixed 592,896-site pair changes only erosion celerity below1 km²; both400-step arms complete, the baseline replays exactly, and independent source/bed/hull/ledger/final-diffusion checks pass. The independent matched render shows broader, greener connected valleys and fewer small crags, but also regular ridge benches and much less mountain mass. Central area above2 km falls154.27→1.108 km², while raster median slope falls52.66°→25.54° and geometric raster basin capacity11.34→2.169 km³. This is partial valley improvement, not a reference-quality mountain solution; no production adoption. The original world is unchanged and the full terrain objective remains open. [Independent paired result](features/2026-09-23-terrain-corrections/evidence/colluvial-domain-control/independent-post-run.md).

**September 24 actual mesh refinement:** the same-source592,896-site comparison completes400 steps after a narrow asymptotic-event roundoff repair; the repaired original-mesh control stays byte-exact. Actual Atlas adds crisp small cuts but worsens cellular ridge bands and still lacks broad connected valleys, so reject production adoption. Central raster median slope rises38.694°→52.657°, native geometric basin capacity0.020302→0.474240 km³ and raster capacity4.180306→11.337142 km³. Independent source/bed/hull/ledger/final-matrix checks pass; a single level is not convergence and these are not annual-water results. A subsequent read-only census concentrates the extra steepness in small-contributing-area hillslopes; it supports testing separate hillslope/channel shaping, but its pre-final-step routes do not prove causation or select a threshold. No further resolution sweep is selected. Production and the saved world remain unchanged; the full terrain-reference objective stays open. [Actual comparison and assessment](features/2026-09-23-terrain-corrections/evidence/spatial-refinement-control/root-assessment.md).

**September24 bank-step sensitivity:** eight smaller bank updates pass the independent checks and reproduce the earlier single-pass result exactly as their control. They reduce discarded virtual attack but remove15.4% less bed, give mixed valley-width proxies and increase geometric basin capacity. The actual render does not clearly improve reference fidelity; no further timestep sweep or production adoption follows. The subsequent mesh-smooth lighting comparison also fails visual acceptance: interpolated native gradients remove hard triangle edges but soften the existing terrain and leave its dense branching pattern. Exact old-mode parity and independent gradient checks pass; no production lighting change is adopted. [Lighting result](features/2026-09-23-terrain-corrections/evidence/native-smooth-light-control/README.md). [Actual comparison and limits](features/2026-09-23-terrain-corrections/evidence/lateral-substep-control/root-assessment.md).

**September24 lateral-bank control:** one fixed evolved comparison now opens some connected valley corridors, while repeated ridges and soft mountain faces remain. Exact baseline replay and independent400-step bed budgets pass; geometric basin depths/capacities improve. The small cross-section width proxy is below native spacing and does not establish realistic valley width. Significant summits survive, but the central maximum loses~473 m. A quarter of virtual bank attack is discarded by the discrete collapse rule, so event-time sensitivity remains unproved. Retain the mechanism as partial evidence; no production adoption or saved-world change. [Actual images and assessment](features/2026-09-23-terrain-corrections/evidence/lateral-bank-control/root-assessment.md).

**September 24 physical-scale diagnosis:** original native geometry in the height-gained probe is intrinsically oversteep: above2 km, median face slope68.483° and89.10% of area above45°. One fixed wider physical interpretation (sample spacing100→483.4857455 m, same heights) yields a less wall-like actual Atlas, but soft mountain faces still miss the reference. Its full crop spans247.545 km instead of51.2 km; it is neither the same geography nor new resolution/evolution. Default image parity,109 renderer tests and correctly scaled90°/60° projection checks pass. An along-crest audit finds existing uplift variation is substantial (median34% range/mean), so no generic source modulation is justified. The completed native-face lighting check gives sharper but visibly triangular terrain, so it is rejected as a production solution. All-face numerical checks and exact sampled-image reconstruction pass; geometry remains the dominant unresolved issue. [Lighting assessment](features/2026-09-23-terrain-corrections/evidence/native-face-light-control/root-assessment.md). Production and saved-world mapping remain unchanged. [Actual scale assessment](features/2026-09-23-terrain-corrections/evidence/physical-scale-render-control/root-assessment.md).

**September 24 closed-cell transfer result:** taking exact triangle minima over each100 m pixel lowers full sampled maximum hollow depth139.916→3.546 m, below the original native6.398876 m bound. Root independently verifies intersection values, exact source/face inputs, all-cell coverage and the same geometric flood; central capacity falls2.015581→0.00672595 km³. This proves a major narrow-outlet sampling contribution, but lowers full-grid heights by median30.281 m (central61.04 m) and up to435.498 m; maximum sampled peak falls261.299 m. Cell minima can also erase real basin separations. Actual Atlas retains dense cliff walls, so reject a drop-in terrain replacement. No production or saved-world changes; reference-quality landforms remain open. [Assessment and limits](features/2026-09-23-terrain-corrections/evidence/cell-minimum-transfer-control/root-assessment.md).

**September 24 native-mesh transfer comparison:** an exact-original-face triangle-linear conversion reduces central sampled geometric basin capacity 4.180306→2.015581 km³ and maximum depth180.269→139.916 m, while native capacity remains0.020302 km³ and maximum6.399 m. An original graph edge omission is isolated to one edge between fixed hull outlets; original faces and all interior edges are verified. Root independently verifies barycentric samples and traces a valid native outlet route that center sampling raises by184 m. Actual Atlas changes local facets but retains the same dense walls; no visual adoption. The completed closed-cell minimum comparison below isolates sampling loss but changes point-height semantics too much for adoption. Full reference fidelity remains open. [Assessment](features/2026-09-23-terrain-corrections/evidence/triangle-linear-transfer-control/root-assessment.md).

**September 24 evolved-height and diffusion-ratio control:** both 400×12.5 kyr arms scale the original initial bed and dense uplift by 4.834857455317221, at the retained anchor K and lambda0/n1. The D=0.011 baseline reproduces the previous vertically scaled shape through actual evolution (maximum native difference 7.584e−6 m; final receivers identical). Increasing only D to 0.11635896796154582 restores broad repeated mountain walls, so reject adoption. Root source/bed/hull and fresh diffusion-matrix checks pass, but actual Atlas, native crops and a fixed 60° view still miss the reference. Native geometric basin capacity rises 0.020302→0.248723 km³; raster falls 4.180306→1.753432 km³, with major raster-induced hollows still unresolved. These are not canonical water results. Production is unchanged and the full reference goal remains open. [Actual assessment](features/2026-09-23-terrain-corrections/evidence/height-scale-diffusion-control/root-assessment.md).

**September 24 historical-anchor K diagnostic:** one frozen 400×12.5 kyr pair tests K=5.555555555555556e−6/year, derived from the project 1 km²/9% channel-slope anchor at a hypothetical constant 0.5 mm/year uplift. The baseline reproduces all 13 retained time-refined outputs and 400 physical ledger rows exactly; source, bed and fixed-hull checks pass. Actual Atlas softens repeated walls into green hills but removes the rocky mountain range: maximum height falls 4,345.801→898.848 m. Central native geometric basin capacity falls 0.911870→0.004199 km³ even as area rises 0.6381%→0.9390%; raster capacity falls 1.653344→0.864616 km³ while area rises 2.1847%→5.3898%. These are geometric, not canonical water results. A one-factor post-evolution height-matched display check reveals more irregular small crests but still dense steep walls and no broad connected valleys; it is not a physical terrain run. Reject this coefficient as a full production replacement; no further K or source variant is selected and the reference-quality goal remains open. [Actual paired assessment](features/2026-09-23-terrain-corrections/evidence/anchor-k-control/root-assessment.md).

**September 24 TIN temporal refinement control:** the unchanged dense cosine-fold n=1 plus D=0.011 model was run for the same 5 Myr at 100 × 50 kyr and 400 × 12.5 kyr. The coarse arm reproduces all 12 retained diffusion fields and actual Atlas exactly; independent all-node source, bed and fixed-hull checks pass. Refined native heights differ by median absolute 16.87 m (p95 245.47 m), showing coupled timestep sensitivity. Central native geometric spill capacity rises 0.305058→0.911870 km³ and raster capacity 1.398812→1.653344 km³; minima count falls but does not establish fewer or smaller basins. Actual Atlas changes local cuts yet retains repeated rounded mountain bands, so hold adoption. One fourfold check proves neither convergence nor reference-quality terrain; no further timestep run is selected. [Actual comparison and limits](features/2026-09-23-terrain-corrections/evidence/tin-time-refinement-control/root-assessment.md).

**September 24 linear fold-profile control:** replacing only the saved dense fold skeleton’s cosine cross-profile with linear top-to-bottom distance, normalized to the same total uplift, completes a 100-step/5 Myr n=1 plus diffusion pair. All 12 baseline fields and 100 physical ledger rows replay exactly; independent source and bed checks pass. Actual Atlas, native and fixed 60° views show limited crest change but persistent rounded faces and repeated mountain bands. Native central geometric spill coverage/capacity falls 0.4128%/0.305058 km³→0.3575%/0.121532 km³; raster falls 2.0351%/1.398812 km³→1.9848%/1.230371 km³. These are bed geometry, not canonical lake water. Hold production adoption and retain the cosine baseline; the reference-quality landform goal remains open. [Actual comparison and limits](features/2026-09-23-terrain-corrections/evidence/linear-fold-profile-control/root-assessment.md).

**September 24 heightfield projection and source control:** a true-depth 60° presentation of the retained 100 m terrain reveals raised faces, but also repeated long cliff walls and smooth crest platforms. Both 90° images match the original Atlas pixel-for-pixel; the declared 1280² interior panels contain no clamped halo texture. Hold projection adoption. A paired zero-background fold-uplift control then reproduces all 12 baseline fields and Atlas exactly, but its actual Atlas image worsens the repeated high walls. Central native geometric basin area/capacity rises 0.4128%/0.305058 km³→10.0948%/64.420222 km³; raster rises 2.0351%/1.398812 km³→13.4289%/80.854159 km³. Reject that source. These are geometric spill measures, not canonical water. Production remains unchanged and reference-quality landforms remain open. [Projection review](features/2026-09-23-terrain-corrections/evidence/heightfield-projection-control/independent-review.md), [source result](features/2026-09-23-terrain-corrections/evidence/zero-background-uplift-control/README.md).

**September 24 rock-surface display control:** one opt-in, deterministic world-coordinate rock-light-normal comparison has completed on unchanged synthetic and saved area (5,4) terrain. Final source review, 30 Atlas tests and strict renderer Clippy pass; the final binary reproduces all four earlier saved-area 512/2048 output hashes. At 512 pixels about 14.8% of saved-area pixels change by at most two RGB levels, and all 26,237 lake pixels remain exact. Root inspection finds no convincing rock faces or meaningful landform improvement, so this fixed setting is rejected for production adoption. It changes display normals only and cannot repair ridge silhouettes, valley widths or drainage; the smooth cubic fields are not proven band-limited or alias-free. No amplitude or terrain-source variant is selected. Production remains 14a70b9 and the reference-quality goal is open. [Actual comparison and limits](features/2026-09-23-terrain-corrections/evidence/rock-surface-detail-control/root-assessment.md).

**September 24 conservative hillslope diffusion control:** a corrected single n=1 stream-power pair completes 100 steps / 5 Myr in 14.588 seconds; its baseline raw fields and actual Atlas image reproduce the retained control exactly. The candidate adds one D=0.011 m²/year signed native-TIN redistribution step with fixed hull heights. Independent solver, all-step matrix/volume and source checks pass after a first attempt stopped on a local residual gate; the corrected run changes numerical stopping only. Actual Atlas shows softer, rounder foothills but smooth repeated mountain bands and narrow incisions, so reject production adoption. Central raster geometric basin area/capacity fall 3.9093%/4.39732 km³→2.0351%/1.39881 km³, while native area/capacity rise 0.0677%/0.128764 km³→0.4128%/0.305058 km³ (central native minima 1→109). These are bed geometry, not canonical lake water. Retain the solver as diagnostic infrastructure; no D or source-parameter sweep is selected, and the reference-quality goal remains open. [Actual assessment](features/2026-09-23-terrain-corrections/evidence/hillslope-diffusion-control/root-assessment.md).

**September 24 quadratic slope control:** the preceding status-only turn made no implementation progress. This continuation revalidated the already completed threshold-operator result, then ran one frozen n=1 versus n=2 stream-power comparison on the same dense source and lowest-pass routing. The n=1 fields and Atlas reproduce the retained baseline exactly. n=2 reduces central raster slope p90/p99 from 61.736°/71.986° to 40.978°/52.413°, but actual Atlas loses irregular crags to smoother continuous ridge bands and narrow valleys. Central native geometric basin area rises 0.0677%→15.0366% and capacity 0.128764→47.499096 km³; raster capacity rises 4.397320→46.633620 km³. Reject adoption. These are bed-geometry measures, not canonical annual-water results; no exponent or coefficient sweep follows. Production and the reference-quality landform objective remain unchanged. [Actual paired assessment](features/2026-09-23-terrain-corrections/evidence/quadratic-slope-control/root-assessment.md).

**September 24 threshold-operator control:** the lambda-removal pair was completed and verified before the preceding status-only turn; this continuation revalidated and retained its result. Removing only the extra positive-above-talus term reduces central native slope concentration within 1° of 41.4874° from 52.33% to 2.94% (above 1 km, 71.86% to 3.38%). Actual unchanged Atlas has more irregular crags but oversteep terrain and narrow valleys; reject adoption. Native central geometric capacity rises 0.1122→0.1288 km³ and raster capacity 1.0675→4.3973 km³; neither predicts canonical annual lake water. Earlier frozen-initial and uniform-material controls remain visually rejected. [Operator assessment](features/2026-09-23-terrain-corrections/evidence/slope-operator-control/root-assessment.md), [prior material controls](features/2026-09-23-terrain-corrections/evidence/uniform-material-source/root-assessment.md).

**September 24 lowest-pass routing:** the prototype's old basin reconnection chose unnecessarily high passes over 37.27% of the strict central area. A deterministic lowest-pass replacement now matches exact native minimax at all 148,992 sites. Two controlled 100-step comparisons reproduce each old baseline exactly and reduce central native geometric fill from 6.15% to 0.32% (strict source) and from 9.63% to 0.38% (dense source). Actual unchanged Atlas shows more coherent valleys, but repeated cliff bands, benches and narrow incisions remain. This is a useful isolated research correction, not production adoption or canonical annual water validation. Raster depression coverage remains about 2.7% in both outputs. The material controls above subsequently show that contacts and material contrast alone do not explain or resolve the repeated bands. [Strict comparison](features/2026-09-23-terrain-corrections/evidence/lowest-pass-source/root-assessment.md), [dense comparison](features/2026-09-23-terrain-corrections/evidence/lowest-pass-dense-source/root-assessment.md).

**September 24 strict-source canonical water:** the local paired solve completes in 47.93 seconds with identical prepared climate forcing and exactly balanced annual ledgers. Actual Atlas uses solved lake surfaces: candidate central lake coverage is 7.58% versus 2.30% control, with many scattered valley fragments and larger inter-ridge lakes. The improved ridges remain promising, but this exact source is not adopted. Dry render parity confirms the images use unchanged terrain and lighting. A completed exact-mesh audit separates two defects: the candidate already has deep native basins (6.15% central site area below spill), while the smooth control acquires its central depressions after grid conversion. Native valley formation and drainage-preserving raster transfer both remain unresolved. [Actual lake-aware result and limits](features/2026-09-23-terrain-corrections/evidence/fold-strict-water-validation/root-visual-assessment.md).

**September 24 stricter ridge selection:** a sole-rule-change alternative also completes both 5 Myr arms. Actual Atlas retains irregular rocky ridges and opens the cellular source into strands, but parallel bands and hollows persist. Central geometric spill-fill coverage is 7.58% versus 2.30% in its own equal-uplift control, with maximum depth 454.87 m. A fresh exact-mesh audit finds 402 candidate local minima versus 2 control. Source checks and field ledgers pass; this remains an isolated float prototype, not adopted Arda terrain. Further graph/parameter sweeps are not selected; the subsequent canonical water test above passes accounting but exposes unresolved lake-aware appearance. [Actual result and limits](features/2026-09-23-terrain-corrections/evidence/fold-strict-source/root-assessment.md).

**September 24 multiscale ridge-forming uplift:** the equal-total-uplift pair completes100steps/5Myr on148,992 nodes. Actual unchanged Atlas now breaks broad smooth crests into irregular ridges and rocky faces: a meaningful source improvement. Cellular hollows and angular benches remain, and final heights have3,201 uphill/nonpositive links in the last pre-update tree. Root checks bed identities, areas and budgets. A fresh TIN audit finds587 candidate local minima versus0 control; raster geometric spill-fill coverage is11.04% versus2.18% in the central crop. These are real depressions, not predicted lake water. The following strict-selection experiment addresses source connectivity but does not remove the basin problem. Retain this direction, withhold production adoption; all terrain types and full-world reference fidelity remain unproven. [Actual comparison and limits](features/2026-09-23-terrain-corrections/evidence/fold-skeleton-source/root-assessment.md).

**September 24 depth-dependent material source:** a paired 148,992-node experiment now completes 5 Myr in both arms, comparing frozen initial material with true exposure of prescribed folded hard/weak layers. Independent scalar, graph and full-field checks pass. Actual Atlas shows changed slope profiles and more steep faces, but broad smooth crests and repetitive radial gullies remain; the source is rejected for adoption. No production or saved-world changes follow. [Actual comparison and limits](features/2026-09-23-terrain-corrections/evidence/stratified-source-probe/root-assessment.md).

**September 24 synthetic source and lighting:** an isolated analytical source generates branching ridges quickly and repeats byte for byte, but its uncalibrated defaults reach 8.644 km and a 57.7° median slope. Actual Atlas output is excessively steep and snow dominated; it is not adopted. A separately frozen 30° receiver-edge slope-limit control also repeats exactly, but yields smooth uniform faces (central median height 3,070 m, median raster slope 30.4°); it is rejected for adoption. The finer three-regime experiment also completed: 2,365,440 sites at nominal 50 m spacing, stopped nonconverged at its runtime budget after 614 iterations. Root verifies node-profile equations and contributing-area balances, but actual Atlas still shows repeated wedge faces and narrow valleys; this output is also rejected for adoption. An exact boundary-coordinate rounding bug in the isolated mesh builder was diagnosed and repaired before the successful bounded run. [Finer source result and actual render](features/2026-09-23-terrain-corrections/evidence/saleve-three-regime/root-assessment.md). A fixed 30° lighting comparison improves sharper source faces but accentuates repetitive gullies in the saved world, so lighting adoption is held. The audit of earlier synthetic candidates found none suitable for reconsideration solely because canonical lake support is now available. [Source control](features/2026-09-23-terrain-corrections/evidence/saleve-source-control/README.md), [lighting assessment](features/2026-09-23-terrain-corrections/evidence/sharp-terrain-lighting/root-assessment.md), [rejection audit](features/2026-09-23-terrain-corrections/evidence/synthetic-source-rejection-audit.md).

**Mountain source prototype:** measured-detail replacement produces sharper relief, but broad smooth ridges remain. A paired local canonical-water solve now passes exact annual accounting under explicit shared boundary/climate conditions. The candidate supports1,037 lakes across that diagnostic domain and has4.04% wet central area; actual Atlas inspection shows dense scattered lakes. This is not full-world validation or reference-quality acceptance, and the prototype is not adopted. The active goal now requires fully procedural, deterministic terrain; measured terrain is diagnostic only and is not an approved production template. The two-band and rounded-river experiments, hillslope incision-area gate, and narrow faceted-crest source were also rejected visually. Production remains at14a70b9. [Actual lake-aware comparison and limits](features/2026-09-23-terrain-corrections/evidence/exemplar-water-validation/README.md).

**Closed erosion research:** the faster finite-volume erosion experiment is rejected for adoption. The generated crop completes only 10 bed updates (2,722.6 years) before its flow gate fails; changes concentrate near the fixed rim and central p90 change is only 0.169 m. The measured crop's flow diverges before any bed update. Actual Atlas inspection shows no useful landform improvement. A subsequent implicit quadratic-drag test solves the inner momentum equation accurately but still fails coupled flow convergence after 100 iterations; it is also not adopted. Production remains at `14a70b9`; the land–river visual mismatch and reference-quality terrain remain open. [Results and comparison](features/2026-09-23-terrain-corrections/evidence/real-terrain-render-control/adaptive-terrain/README.md).

The Atlas renderer and authorized terrain/shoreline corrections are implemented on `codex/geographical-rendering-first-pass`. Existing physical generation, annual hydrology, saved-world loading and tactical prototypes remain implemented; they are not starting over. The requested natural-atlas reference is still the visual target, and it has not been reached.

Terrain source `4ea2271` adds globally sampled 8/4 km structural relief before 160 shared 100 m evolution steps, on top of the earlier Euclidean tectonic belts, preserved ocean-rim depths and coarse hillslope-only evolution. The reviewed deterministic fixture is adopted under explicit user authorization, and both root fingerprint/repeated-generation tests pass. The completed default seed-42 world took 54 min 16 s, contains 171 published areas and passes the named saved-water checks. Matched overview/highland/river/coast/lake inspection accepts deeper branching valleys as a partial improvement. Smooth principal crests and repetitive fine gullies remain.

Renderer source `14a70b9144dd` draws world-overview rivers from saved connected channel strips and junctions, with tapered widths and fractional pixel coverage. Display widths are eight times saved physical widths, capped at 1 km; physical area exports remain unchanged. Matched native and fitted inspection accepts narrower banks and connected tributaries as a partial improvement. Small fitted streams remain faint, D8 bends remain angular and water colour remains categorical. Mountain morphology is unchanged. The component passes 108 renderer tests, 12 public export tests, formatting, strict renderer/facade Clippy and two final independent reviews. Two 32K exports match exactly, take 93.19/92.62 seconds and leave all 523 world files unchanged. Supported minimum quality512 also passes. [Actual render and verification](features/2026-09-23-terrain-corrections/evidence/connected-river-integration/README.md).

The latest terrain diagnostic reproduces every saved 1 km height and traces the nearly level highland ridge back to a persistent collision-source ribbon in the 4 km uplift field. A wider structural gate was rejected because it produced additional depressions and radial forms. The isolated convergence-magnitude experiment lowers the highland while leaving its broad crest intact, so it is also rejected; no further terrain change is installed. A completed fixed-crop test of an adapted published multiscale-erosion preset is also rejected: it produces sparse angular ravines and smooth interfluves, with no useful principal-crest improvement. The initial resampling and changed kernel limit that result; it does not reject all multiscale methods. The subsequent warped-source concentration trial also retains the same broad crest widths and one prominent summit, so it is rejected before fine evolution. A separate saved-field audit identifies lowland relief gating and the absence of a bedrock/resistance distinction as additional limits on hills and cliffs. A subsequent discrete-anchor source-envelope trial also fails to create a second prominent 2D summit and is rejected. A scratch explicit ridge/pass/valley reconstruction does create multiple summits that survive the unchanged erosion kernel (23 conditional interior summits versus one control), but it leaves 122.16 km² of depressed terrain and unnatural trench bands, so it is rejected for adoption. Terrain-compatible outlet constraints on that same graph are infeasible: low-pass sources would have to drain toward a retained bed outlet about 1,300 m higher. No further terrain method is selected for production. The completed whole-domain coastal graph prototype retains29 conditional prominent summits after unchanged erosion, with5.529m maximum residual highland-crop spill depth; it avoids the prior high-rim conflict but still has smooth shoulders and repetitive gullies. Matched terrain-only images through the current Atlas renderer confirm the divided crest and expose remaining straight terrace edges. Its graph checks do not establish raster drainage, and all tests remain isolated from production. Numerical checks and larger PNGs do not establish visual acceptance.

Subsequent mesh auditing found that the prototype's original triangulation did not enforce all prescribed ridge/river segments. Correcting that call, preserving interpolated constraint heights and removing incompatible perturbation crossings now passes all 48,622 segment checks. The corrected crop retains 30 conditional prominent summits after unchanged evolution but still has 9.45 km² of depressed ground (maximum depth 6.480 m), smooth shoulders, terrace edges and repetitive gullies. Actual Atlas inspection rejects production adoption. A separate frozen-input finite-cover/bedrock test passes eight focused tests and exact per-step material budgets, but strips cover from 93.6146% of the central crop and amplifies artificial gullies; it is also rejected. Those terrain experiments change no production terrain source. Evidence: [corrected mesh and render](features/2026-09-23-terrain-corrections/evidence/coastal-topology-correction/EVOLUTION.md), [material test](features/2026-09-23-terrain-corrections/evidence/bedrock-face-probe/README.md). Neither diagnostic establishes natural river rendering or completes the reference-quality goal.

The remaining prototype shelf is traced to its valley-height rule and broad fill, not simply coarse triangles: northern river vertices sit a median 821 m below the original coarse bed, and the evolved field retains a 468 km² connected low-slope shelf. The reconstruction preserves sea and other islands but does not preserve the interior mainland height envelope. An exact graph-profile feasibility solve proves that this same graph and its saddle caps require at least 2,018.222 m of deviation from accepted terrain. Root independently verified all path bounds and a feasible profile. Profile coefficients alone cannot repair that mismatch, so no new raster or erosion run follows. [Cause and measurements](features/2026-09-23-terrain-corrections/evidence/coastal-plateau-diagnosis/README.md).

Connected river geometry is integrated in the Atlas saved-world overview facade and available through additive renderer APIs. Complete neighboring saved geometry is clipped in source coordinates before mapping to unequal output partitions. Dense pixel unions subdivide at 64 pieces with unchanged bounded work/depth failures; this fixes a measured minimum-quality export refusal. Classic, direct legacy Atlas APIs, physical area widths and saved-world formats remain compatible. The independent analytic boundary fixture matches every pixel; retained dense fixtures match the GEOS alpha oracle within Q20 tolerance. [Integration design](features/2026-09-23-terrain-corrections/evidence/river-geometry-audit/integration-design.md), [production receipt](features/2026-09-23-terrain-corrections/evidence/connected-river-integration/production-integration.json).

An instrumentation-only replay of the rejected finite-cover experiment reproduces final height, rock, cover and every material-ledger row exactly. The physical incision kernel has no channel-initiation gate: contributing areas below 16 cell-equivalents account for 55.95% of actual incision and 58.75% of rock cutting, but only 28.53% of cover directly removed by incision. This confirms widespread hillslope cutting without proving it alone causes the repeated gullies. The area measure is not discharge and cannot be equated to the later 40 L/s saved-channel rule. No threshold or new terrain variant is adopted. [Replay and interpretation](features/2026-09-23-terrain-corrections/evidence/channel-initiation-audit/replay/README.md).

A fixed incision-threshold trial now reproduces the zero-threshold material control exactly and passes 224 independent scalar checks. At the predeclared 900 mm-per-step threshold, cover retention improves, but actual Atlas images retain smooth shoulders and repetitive steep channel clusters. A separate transfer to the accepted initial terrain still has one broad prominent crest and is also rejected for adoption. Root independently reconciles both transfer material ledgers and layer identities; production terrain remains unchanged. [Frozen-input result](features/2026-09-23-terrain-corrections/evidence/incision-threshold-oracle/VISUAL.md), [accepted-source comparison](features/2026-09-23-terrain-corrections/evidence/incision-threshold-transfer/README.md).

A measured-elevation control now separates source and evolution defects. With unchanged Atlas and identical neutral metadata, public Alpine heights show63 saddle-defined ≥150m summits versus none in the generated crop (one global/sea-base peak). The unchanged160-step zero-uplift kernel then reduces those63 to31, visibly removes craggy detail and introduces repeated grooves; slopep99 falls47.37°→37.14°. Both starting ridge/summit geometry and erosion treatment therefore remain open. This is a diagnostic using measured terrain, not a generated-world result or adopted data source. Preparation was independently reproduced exactly. Frozen finite-cover T0/T900 controls on the same measured input also fail visual acceptance: they retain50/62 prominent summits but create dense exaggerated gullies, with median slopes41.89°/39.72° versus23.37° initially. Peak retention alone is insufficient. Three verified single-operator removals also fail visually: no incision blurs the terrain, no creep sharpens repeated cuts, and no collapse leaves the broader defect. A dependency-free CPU scalar transport prototype now passes36 constant-flow/decay cases against independently reviewed cell-average solutions, all conservation checks and exact repeat. Root pooled relative L2 error falls6.891%→0.0352% from1→256samples/cell; maximum ledger relative residual is9.42e-14. A signed-field extension now passes 52 spatial-source/decay and affine-flow cases (0.0487% pooled relative L2 at 256 samples/cell, maximum ledger residual 1.97e−12), while preserving all 36 scalar outputs exactly. An explicit-SI channel check conserves eroded/deposited/exported material through one imposed-flow bed update and validates a separate frozen momentum update; independent review is dry. The subsequent fixed-bed nonlinear water/velocity benchmark also passes on 40/80 cells: maximum fine-grid velocity/depth errors are 0.04277%/0.05967%, water discharge stays exact, and final field/history repeat byte-identically. This validates the declared conservative-velocity feedback law only. A ten-year changing-bed channel now also passes volume conservation and exact-repeat checks, including a zero-erosion control; its fixed bed faces strongly influence the flow response. The first actual-terrain water solve hit its 300-second cost limit before any bed update. A bounded tail cutoff and guarded zero-time face transition now let both initial terrain-water tests complete with zero numerical/cap loss; exact prior-field regressions and independent review pass. The full generated-tile feedback then converges in33iterations, but its first bed update fails the5m step limit even after four halvings; the measured arm stops at its runtime projection. Both final beds remain unchanged in that first attempt. The subsequent adaptive finite-volume branch accepts ten generated bed updates over 2,722.569 years, concentrated near the fixed rim, before flow convergence fails; measured terrain accepts no update. Its actual Atlas result is rejected. Solving quadratic drag implicitly stabilizes the inner momentum equation but still fails the outer measured water–velocity convergence gate. These later results supersede a blanket claim of no accepted updates; transport and terrain appearance remain unresolved, with no production adoption. Evidence: [real-terrain control](features/2026-09-23-terrain-corrections/evidence/real-terrain-render-control/README.md).

The latest mountain presentation probes are rejected: broader-scale light adds little, while stronger slope exaggeration and a lower sun deepen parallel grooves without improving the smooth crest. They are diagnostic relightings of saved heights, not production exports. No mountain shader or terrain change is adopted in the river pass; mountain visual acceptance remains open. [Latest river before/after and verification](features/2026-09-23-terrain-corrections/evidence/natural-river-pass/README.md).

### Implemented work

| Capability | Current implementation and evidence |
|---|---|
| Continental terrain and climate | Plate/tectonic relief, erosion, coastal shaping, temperature/rainfall, coarse drainage and rivers, and bounded deterministic validation/rerolls. `crates/arda-gen/src/continent/`, `crates/arda-gen/src/orchestrator.rs:357`. |
| Shared fine terrain | One modeled 100 m physical domain receives globally sampled structural relief and 160 shared evolution steps before area slicing; erosion crosses internal publication boundaries. Legacy tile evolution remains 40 steps. `crates/arda-gen/src/area/prepare.rs:21`, `crates/arda-gen/src/continent/structural_relief.rs:61`, `crates/arda-gen/src/area/evolution.rs:20`, `crates/arda-gen/src/area/evolution.rs:194`. |
| Regional detail correction | Fine-relief amplitude follows surrounding regional height differences instead of absolute altitude. C06 is installed, not pending terrain work. `crates/arda-gen/src/area/prepare.rs:48`, `crates/arda-gen/src/continent/bundles.rs:46`; commit `ed4875d`. |
| Shared annual hydrology | Fine drainage topology, physical basin hierarchy, rainfall/runoff/evaporation support, lake surfaces, annual transfers/accounting, connected reaches and cross-area records. Final areas consume shared results. `crates/arda-gen/src/orchestrator/shared_solve.rs:382`, `crates/arda-gen/src/hydrology/annual.rs:135`, `crates/arda-gen/src/orchestrator.rs:422`. |
| Produced area fields | Height, terrain, slope/aspect, temperature, rainfall, drainage, discharge, channel order/width, height above river and wetness have producers; they are not all defaults. `crates/arda-gen/src/area/shared_compose.rs:165`. |
| Basic ground cover | The canonical composer emits Bare on non-land, Marsh on qualifying floodplain and Grass otherwise. Full biome/forest vegetation is a separate unfinished layer. `crates/arda-gen/src/area/shared_compose.rs:175`. |
| Saved world data | Format-4 publication stores real continent overview/climate/drainage, area cells/objects, global water records and sampled blocks; the completion manifest is published last. `crates/arda-gen/src/orchestrator.rs:450`, `crates/arda-gen/src/orchestrator.rs:480`, `crates/arda-core/src/formats/`. |
| Resource admission | Resource limits are checked before output creation and applied through preparation, solve and publication. `crates/arda-gen/src/orchestrator.rs:338`. |
| Lazy loading | Manifest-first loading, requested area/archive caches and owned area reads for exports exist. Whole-world eager loading is no longer the implementation. `crates/arda/src/world.rs:93`, `crates/arda/src/world.rs:192`, `crates/arda/src/world.rs:208`. |
| Cartographic PNG exports | Area/world maps render saved terrain and physical water geometry. Configurable quality defaults to 8K and accepts 512–32768 pixels; quality exports stream rows/bands through temporary-file publication. Higher image resolution does not regenerate terrain. `crates/arda-render/src/quality.rs:11`, `crates/arda-render/src/overview/streaming.rs:30`, `crates/arda/src/export_quality.rs:20`; commit `c577528`. |
| Atlas rendering first pass | Natural elevation palette, saved-wetness land tint, slope-derived rock/snow appearance, neighbor-aware relief shading, sea/lake-depth colour and per-output-pixel palette/light interpolation; Classic remains the default. Bounded 512–32K exports preserve saved 100 m data; guarded integer height contours refine displayed sea/land shores while lake ownership remains categorical. `crates/arda-render/src/atlas.rs:164`, `crates/arda/src/lib.rs:28`, `crates/arda/src/export_quality.rs:35`. |
| JSON and developer surfaces | Versioned area/block JSON, reusable Rust facade, CLI `generate`/`preview`/`export`, and Docker build definition exist. `crates/arda-render/src/json.rs`, `crates/arda/src/lib.rs`, `crates/arda-cli/src/main.rs`, `Dockerfile`. |
| Tactical prototype | Seeded 64×64 blocks, 24 tile kinds, bounded WFC attempts, relaxed-fill markers, compressed persistence and symbolic PNG/JSON export exist. Coverage is still sampled. `crates/arda-gen/src/block/`, `crates/arda-gen/src/orchestrator.rs:300`, `crates/arda-render/src/symbolic.rs`. |
| Verification infrastructure | Physical/annual/boundary/resource/codec/render tests and a pinned golden world exist. CI defines Linux/macOS/Windows tests, MSRV, formatting, Clippy and dependency checks. Latest run status is recorded separately below. `crates/*/tests/`, `tests/golden_world.rs`, `.github/workflows/ci.yml`. |

### Remaining implementation and scope boundaries

| Area | Current boundary |
|---|---|
| Rich vegetation and soil moisture | `Cell.moisture` and `forest_density` lack production assignments. Current Bare/Grass/Marsh cover does not implement the planned ecological vegetation system. `crates/arda-core/src/cell.rs`, `crates/arda-gen/src/area/shared_compose.rs:217`. |
| Human geography and society | Settlement/road/building/name/realm/NPC generation remains deferred. `road` and `built_by` default; manifest settlement and named-river counts are zero. `crates/arda-gen/src/area/shared_compose.rs:217`, `crates/arda-gen/src/orchestrator.rs:527`; retained design `logic/06-society-generation.md`. |
| Richer world and area presentation | Classic and the Atlas relief/palette first pass are implemented. Rich ecological cover, settlement outlines, main roads, building outlines and local paths remain work. Detailed steps: OI-12/OI-14 below. |
| Complete tactical scenes | Preliminary constraints/WFC do not supply coherent furnished buildings, continuous site layouts, detailed assets or movement/collision data. Blocks use a 64-cell sampling stride; accessing one decompresses its area's block archive. `crates/arda-gen/src/block/constraints.rs`, `crates/arda-gen/src/block/wfc.rs`, `crates/arda-gen/src/orchestrator.rs:300`, `crates/arda/src/world.rs:274`. |
| Serving and browser integration | No `serve` subcommand or browser app is implemented. Full area JSON has no demonstrated browser parse/loading budget. `crates/arda-cli/src/main.rs`, `crates/arda-render/src/json.rs`, `07-operations.md`. |
| Full statistical calibration | Existing physical, drainage, boundary and determinism checks do not complete the proposed combined Horton/Hack/rank-size/sinuosity/farmland suite. `06-testing.md`, retained step 12 in `implementation.md`. |
| Static model scope | Annual water is implemented. Seasons, snow storage, groundwater and dynamic floods are outside the current model; their absence does not make annual hydrology pending. Terrain samples remain 100 m apart. `crates/arda-gen/src/hydrology/annual.rs:135`, `crates/arda-core/src/coords.rs:10`. |

### Verification and visual acceptance

- **Latest full-map export (`14a70b9144dd`):** Two 15,522×32,768 connected-channel PNGs match exactly (329,746,127 bytes; 93.19/92.62 s). The 512-pixel export completes; Classic2K and physical area1725 controls remain byte-identical, and all523 saved files remain unchanged. Root inspected actual native/fitted output and accepts a partial river-geometry improvement; landform and natural-river acceptance remain open. [Receipt](features/2026-09-23-terrain-corrections/evidence/connected-river-integration/production-integration.json).
- **Previous full-map export (`92689f8`):** Two 15,522×32,768 PNGs match exactly (326,768,717bytes; 80.75/80.68s). All523 saved files remain unchanged. Root inspected the actual fitted and native-pixel comparisons: variable widths, softer banks and shaded water improve integration with the terrain. Mountain forms remain unchanged. [Receipt](features/2026-09-23-terrain-corrections/evidence/natural-river-pass/surface-receipt.json).
- **Previous river-readability export (`a9ce4fe`):** Two 15,522 × 32,768 PNGs are byte-identical (325,652,273 bytes; SHA-256 `a60ed031cf7d7d95c18e1cf2e520b85161c7d5ea6f1ba7b1bceee86f3fea591d`). Runtime is 78.91/79.36 s. All 523 saved files (1,943,353,639 bytes) are unchanged. Root inspected the preview downsampled from 32K and accepted the river visibility improvement; mountains are unchanged. [Receipt](features/2026-09-23-terrain-corrections/evidence/readability-pass/render-receipt.json).
- **Previous full-map export (`3f95fcc`):** The committed renderer produced two byte-identical 15,522 × 32,768 PNGs (327,031,502 bytes each) in 76.16 s and 76.05 s, at 66,516 and 66,284 KiB peak child RSS. All 523 saved world files (1,943,353,639 bytes) retain identical before/after hashes. The 970 × 2048 preview is downsampled from the completed 32K PNG, not a separate 2K export. Root inspected it; smooth principal ridges and repetitive fine gullies remain. Receipt: `features/2026-09-23-terrain-corrections/evidence/wetness-response-probe/production/full-world-32k/receipt.json`.

- **September 23 saved-wetness material (`3f95fcc`):** 98 renderer tests, 12 facade area-export tests, formatting and strict renderer Clippy pass. The nonzero-wetness mixed-axis parity test also passes after a test-only loop cleanup. Two fresh independent GPT-6 Sol review rounds are dry. Tests cover the exact tint oracle, zero-wetness compatibility, cardinal/diagonal halo consistency, unchanged water/classes and both mixed-axis streaming layouts. Evidence: `features/2026-09-23-terrain-corrections/evidence/wetness-response-probe/production/verification.json`.

- **September 23 structural-relief adoption (`4ea2271`):** 674 non-golden workspace tests pass, with eight ignored; the root golden package was excluded. Two independent component code reviews found no confirmed implementation defect. Focused structural tests, strict all-package Clippy, formatting, Rust 1.96.1 and cached offline dependency policy pass after a test-only integer-cast correction. The cleaned private implementation reproduced all 34 saved MICRO99 files byte-for-byte against the frozen candidate. Named saved-water checks pass on small42 and MICRO99, including exact annual balance, crossing authority, positive lake depth and channel fields. The two small candidate worlds have zero both-wet published seam pairs, so those particular seam checks are vacuous; the completed default candidate passes 12,356 pairs, and the accepted small control passes 148 pairs. The independent release golden repeat passes. The pre-adoption stored fingerprint comparison failed as expected: 23 of 34 files change (12 area files, four block files and seven water tables), with no missing or extra files; the reviewed fixture change is committed under explicit user authorization. Both root release golden tests pass (192.10 s), covering strict fingerprint comparison and independent repeated generation; formatting and whitespace checks also pass. The default candidate has 171 published areas, 87 global lakes and 734,623 published channel cells; its annual source and sink both equal 153,308,678,440,000 L. Exact input hashes and all named checks are retained. Full default output and root visual inspection are complete; source and fingerprint adoption are complete in `4ea2271`. Exact installed source hashes match the independently reviewed proposal; broader visual acceptance remains open. Receipts: [candidate verification](features/2026-09-23-terrain-corrections/evidence/integer-regional-relief-probe/candidate-160/README.md). The candidate also has a verified 15,522 × 32,768 Atlas export (206,066,791 bytes): completed exports took 71.31 s and 71.40 s, repeat byte-for-byte, and preserve all 523 saved world files. Receipt: `features/2026-09-23-terrain-corrections/evidence/integer-regional-relief-probe/candidate-160/full-world-32k/receipt.json`.
- **Candidate lake change:** small42 global wet membership falls from 21,846 to 2,300 cells, including modeled fringe. In the three largest changed published fragments, the structural gate is zero at all prior lake members and old outlet witnesses; former floors rise roughly 5–6 m while old outlets fall 11–13 m. This supports drainage through evolved terrain without proving duration is the sole cause. No lake-count quota or deletion was added. The holdout's final largest saved lake depth is 3 mm; the initial coarse diagnostic pocket is not treated as the same final lake.
- **Bounded negative visual trials:** two saved-climate colour variants add only modest tint, and the sampled highland would receive essentially no terrain cast shadows under current lighting. Doubling structural amplitude creates ringlike forms and deeper crop depressions; it is not selected. Routing all contributing area to the existing steepest receiver produces long grid-aligned channels and angular terraces in a controlled crop, so that variant is also rejected. An unchanged-kernel 1,600-step crop reshapes broad relief but produces large angular facets and deeply entrenched channels; more duration alone is not selected. These are isolated diagnostics, not new ecology, lighting or generator features. The reference-quality objective remains open.
- **Saved-wetness tint trials:** the first restrained mapping was too weak to change overview appearance materially. A separate predeclared stronger mapping was accepted on matched adopted-world renders and integrated in `3f95fcc`. The initial accidental old-world comparison is retained separately and excluded from the acceptance decision. Soil moisture, forests and ecology are not inferred.

- **September 23 lake-depth continuation (`0fc9666`):** 94 renderer, 12 facade area-export and 14 CLI tests pass; scoped strict Clippy and formatting pass. Two independent code reviews are dry. Corrected clean-build before/after outputs were inspected; Classic lake-margin output is byte-identical. That stage’s 32K map completed in 69.34 s at 76,904 KiB peak child RSS. Full visual acceptance remains open.

- **September 23 material continuation (`3c06909`):** 89 renderer tests and 56 public-facade/CLI tests (six suites) pass; formatting and strict workspace Clippy pass. Two fresh independent material reviews are dry. Actual 2K overview and matched highland/coast/river exports were inspected; the updated 32K export completed in 66.92 s. These checks cover the material change, not a fresh complete generation/CI run or final visual acceptance.

- **September 23 correction checks:** broad non-golden suite 659 passed/8 ignored before the final test additions; then 18 tectonic tests, five continent tests, strict golden comparison and independent repeat generation pass. Formatting, strict workspace Clippy, Rust 1.96.1 and offline dependency policy pass. Two fresh dry review rounds cover `7028f9b..757b2ab`. This is combined evidence, not a new all-at-once workspace run; no new remote/Windows CI result is claimed.
- **September 23 actual worlds:** small/default seed42 and MICRO99 have exact saved annual balances. The default has 118 lakes, 799,834 reaches and 2,123 crossings, each agreeing with its canonical reach. Its 32K map repeats byte-identically in 65.65 s at 57,240 KiB peak RSS; all 523 world files remain unchanged. Detailed receipts are in [testing](06-testing.md#terrain-and-shoreline-correction-verification--2026-09-23).
- **September 23 visual diagnosis:** coarse-incision removal reduces straight-cut artifacts in the controlled small-world comparison; the shared sea/land contour removes many staircase edges while protecting thin features. Parallel valleys and broad smooth mountain forms remain. The default world's largest lake is a real 30,709.15 km² positive-depth basin at 1,177.501 m; independently, the coarse bed has an enclosed 32,337 km² basin with escape elevation 1,173.357 m. This supports the generated terrain as its cause, not a misplaced render polygon. No lake-size quota, arbitrary flattening/deletion or invented texture was added.

- **Recorded C06 verification, September 8:** the durable ledger reports five frozen worlds, 38 selected JSON checks, 238 exports, exact annual balances and matching IDs/surfaces on 20,712 both-wet neighboring pairs. Its combined broad/focused/approved-golden record reports 590 passing tests. Source: `changelog.md`, `snapshot/2026-09-07-area-water-terrain-realism@c06-committed-cleanup` and `fix/2026-09-07-area-water-terrain-realism@lake-district-c06-verification`.
- **Later export verification:** commit `c577528` records full 32K area/world exports, 611 workspace tests and 74 final targeted checks, formatting, strict Clippy, Rust 1.96.1 and cached dependency checks. These counts can overlap; they are not added together. Generation, saved formats and goldens were unchanged by that commit.
- **Fresh Atlas verification, September 22:** source `342d03e55120` passes 648 Linux workspace tests (8 ignored), formatting, strict Clippy, Rust 1.96.1 checks and dependency audit. Three old-executable Classic comparisons match byte-for-byte. The retained seed-42 200×300 km panel includes matched 8K views of five areas and repeated deterministic 32K area/overview exports; all 55 saved files and the five re-exported area JSONs are unchanged. Two three-lens GPT-6 Sol review rounds found no issues. Local acceptance/evidence: `features/2026-09-22-geographical-rendering-first-pass/acceptance.md`.
- **Palette continuation verification, September 22:** production source `9a16883` passes 649 workspace tests (8 ignored, 21 suites), in addition to the recorded 80 renderer tests, formatting, strict renderer Clippy and real-image compatibility checks. Scratch shoreline and drainage experiments are excluded from this production-source result. Local receipt: `features/2026-09-22-atlas-visual-fidelity/evidence/workspace-tests-palette.json`.
- **Base CI, checked September 22:** [run 34260968213, attempt 1](https://github.com/DiceMasterIO/arda/actions/runs/34260968213), on base commit `23dfb0999672c8f15deee6d94dc5dee89b946ce8`, completed September 8 with a Windows failure. Linux/macOS test jobs, lint, MSRV and dependency checks passed. The [Windows job](https://github.com/DiceMasterIO/arda/actions/runs/34260968213/job/102178507146) failed `micro_world_matches_the_golden_fingerprint`: actual text used LF and expected text CRLF. All 34 logged fingerprint entries match after line-ending normalization; the same-seed repeat test passed. This is an unresolved golden-text comparison failure, not evidence of different generated fingerprints. No newer run/retry was present when checked.
- **Delivered visual correction:** the C06 record reports seed436342 changing from 2,653 to 186 lakes, and 50 km squares with at least 50 lake anchors changing from 18 to zero. The repeated lake-district artifact was corrected. Those observations are not lake-count quotas for every world.
- **Historical September 22 visual gaps (superseded by the correction evidence above):** parallel drainage/ravines, angular shorelines and large rectangular regional basins remained open on September 8. No later terrain/hydrology behavior change was found establishing their resolution. The September 22 stage probe exactly reproduces all 60,000 saved regional heights of seed 42, 200×300 km: the straight valleys in area (2,0) appear during the 25 continent-erosion iterations, then area evolution deepens them. The current renderer preserves categorical 100 m shore steps. These findings localize two visible artifacts; they do not resolve them or the wider rectangular-basin finding.
- **Evidence available here:** the original terrain-realism feature reports/gallery remain absent; their absence does not undo committed work. A new retained Atlas feature folder now contains its own generated world, actual PNGs, hashes, test receipts and acceptance report. These are newly dated evidence, not a reconstruction of the old gallery.
- **Completion boundary:** the broader terrain-realism feature has no completion marker in the durable ledger because overall visual acceptance remained open. Its implemented terrain, water, storage, loading and rendering must not be treated as unstarted.

Current build-order reconciliation: [implementation.md](implementation.md).
Current contracts: [models](02-models.md), [testing](06-testing.md),
[operations](07-operations.md) and [export behavior](logic/04-export.md).

## Detailed work queue — 2026-09-22

This is the actionable queue for the remaining work, incorporating the requested
world, area and illustrated tactical maps. The implementation evidence above is
the baseline. Steps below describe the remaining scope except where a dated delivery is explicitly recorded; a
decision listed inside a task must be settled before implementing its dependent
steps. Existing approved plans and the dated evidence remain available below.
New module names explicitly marked **proposed** do not exist yet.

Each item records the starting point, dependencies, code targets, ordered steps
and a completion gate. Closing an item requires its evidence, not merely code
that compiles. Update its status and link the resulting verification when it
ships. This queue provides the detailed delivery order and includes non-map leftovers.

### Required result at each scale

| Scale | Required result | Source of truth |
|---|---|---|
| World | Substantially richer geographical detail; city, town and village **outlines**, main roads and other features observable at that distance. | Saved terrain/water/ecology plus canonical settlement extents and regional routes. |
| Area | Building outlines, smaller roads/paths and the larger world features, legible at useful settlement zoom. | The same shared site plan, with finer geometry than the 100 m terrain raster. |
| Tactical | Full terrain, structures, entrances, interiors, furnishings, vegetation and props, with movement/visibility geometry. | Refinement of that shared plan; artwork illustrates the semantic scene. |

World and area maps must work before tactical interiors or furniture exist.
Society and geography establish settlements, functions, populations and access;
site planning supplies streets and building shells; society binds inhabitants
and workplaces to those records. Only bounded feasibility feedback is needed.
A building that cannot fit its required function may cause an explicit shared
layout revision; tactical generation must not silently redraw published maps.

Arda owns the reproducible generated base and semantic geometry. DiceMaster
owns its artwork presentation, game-rule interpretation and campaign changes.
WFC assembles compatible pieces where useful; it does not replace terrain,
town planning, architecture, furnishing rules or an illustrated renderer.

### Delivery order and dependencies

The IDs are stable references, not a demand to finish every row serially.
In particular, start the asset/camera proof early, and improve world relief
rendering while human geography is being built.

| Milestone | Work items | Exit result |
|---|---|---|
| A — trustworthy baseline | OI-01–OI-05 | Reproducible current images and CI; each historical terrain finding has a current disposition. Confirmed fixes can proceed alongside unrelated work. |
| B — shared data and ecology | OI-06–OI-08; begin OI-23 | Stable coordinates, versioned layout contract, staged generation and meaningful vegetation inputs. |
| C — inhabited, richer world | OI-09–OI-13 | Finalized shared settlement/site layout, land use and main roads on a substantially improved geographical render. Rendering development starts before layout finalization. |
| D — coherent area maps | OI-14–OI-15 | Streets/building outlines and linked society using that shared plan; world/area exports work without tactical files. |
| E — visual feasibility, in parallel from A | OI-21 | One structured, asset-backed benchmark proves camera, scale, art and gameplay readability early. |
| F — generated tactical places | OI-16–OI-20, OI-22, complete OI-23 | Detailed, furnished, illustrated sites that preserve the shared layout and publish real geometry. |
| G — playable delivery at scale | OI-24–OI-28 | Coverage, bounded loading, serving and a working DiceMaster scene within measured budgets. |
| H — acceptance and distribution | OI-29–OI-30 | Reproducible quality reports, full CI and verified release artifacts. Checks also accompany every earlier milestone. |
| I — expand content | OI-31 | More environments and building families meet the same acceptance bar. |

### OI-01 — Fix the Windows golden-text comparison

**Status:** open, diagnosed. **Depends on:** none. **Targets:**
`tests/golden_world.rs`, `tests/golden/micro-42.txt`, `.github/workflows/ci.yml`.
The exact-HEAD Windows failure concerns CRLF versus LF; all 34 logged hashes
match after normalization. This does not justify regenerating the golden.

1. Add a focused comparison fixture demonstrating equivalent LF/CRLF text and a genuinely different fingerprint that must still fail.
2. Choose canonical text comparison or an explicit checkout-line-ending policy. Normalize only transport line endings; preserve fingerprint keys, values, record count and meaningful content.
3. Apply the narrow fix. Keep independent same-seed generation comparisons; do not bless new hashes to conceal a portability failure.
4. Run the focused check and golden comparison, then obtain a fresh three-platform CI result for the corrected commit.
5. Record the exact commit/run and any remaining failure. Keep previous successful evidence and the September 8 failure dated separately.

**Done when:** equivalent line endings pass, changed fingerprints fail, and the
fresh CI run passes every configured job. The approved hashes remain unchanged.

### OI-02 — Restore a reproducible visual and measurement baseline

**Status:** integrated baseline retained and measured: seed42 at 200×300 and 500×1000 km, plus MICRO99 holdout. The same six default-world coordinates are exported at 8K, and the full 32K default map repeats identically without modifying saved data. Broader natural-landscape acceptance remains open. **Depends on:** current generator; OI-01 only for Windows text comparison.

1. Attempt to locate the original feature evidence through its recorded paths. If unavailable, retain its ledger citations and generate a newly dated baseline; never label recreated images as original evidence.
2. Freeze a manageable panel including default seeds 42 and 436342, MICRO controls and additional representative climates/landforms. Record configuration, source revision and generator version.
3. Save world/area views at declared, matched physical scales and selected JSON/geometry measurements. Preserve enough input data to rerender without regenerating the world.
4. Mark each old finding—parallel ravines, angular shores, rectangular basins—as reproduced, absent or inconclusive. Distinguish 100 m quantization, image presentation and underlying terrain shape.
5. Capture intermediate stages only where diagnosis needs them: coarse heights, initialized/evolved fine relief, drainage/basins, wet membership and final rendered output. Keep diagnostic tools outside production paths unless they earn a supported interface.
6. Store commands, locations, timings and hashes with the results. Separate deterministic numerical assertions from visual review notes; record missing evidence explicitly.

**Done when:** another developer can reproduce the current gallery and its
measurements, and every historical visual finding has a current disposition.

### OI-03 — Diagnose and correct reproduced parallel drainage/ravines

**Status:** corrected in part September 23. Controlled full-world removal of coarse 1 km incision reduces the measured straight-cut trough from 203.8 to 83.1 m and long horizontal runs from seven to two; the production stage retains 25 creep passes and existing shared 100 m incision. Broad parallel valleys remain in the default highland gallery. **Depends on:** OI-02. **Targets:** `crates/arda-gen/src/continent/erode.rs`, `crates/arda-gen/src/area/evolution.rs`.

1. Trace a reproduced regular pattern back to the first stage that introduces it. Measure valley convergence, drainage orientation and incision along the same slopes before and after evolution.
2. Recheck current interpolation: `coarse_height` already calls the bounded, affine-preserving sampler. The older smoothstep-bilinear diagnosis is historical; the separate 4 km→1 km sampler still uses bilinear interpolation.
3. Compare controlled planes, rotated slopes, ridges, converging valleys and real catchments to separate natural parallel drainage from repeated lattice artifacts.
4. Change the demonstrated source—relief construction, interpolation, convergence or erosion behavior—using controlled alternatives. Do not prescribe a noise replacement merely because the old inventory named one.
5. Preserve C06's regional-relief amplitude, genuine bowls, shared-domain continuity, annual accounting and deterministic resource bounds. Reject improvements that introduce speckled coasts or artificial pond fields.
6. Compare the same views across the frozen panel and holdout seeds. Record mechanism, visual improvement, numerical effects and any unresolved cases.

**Done when:** the reproduced defect improves at the relevant scale without
regressing physical/visual controls. No arbitrary diagonal-flow percentage,
river-count quota or noise-only metric substitutes for that evidence. If OI-02
cannot reproduce it, close as not reproduced with evidence instead of changing code.

### OI-04 — Diagnose and improve angular shorelines

**Status:** shared Atlas sea/land reconstruction implemented and verified September 23, including zero-height, thin-island/strait, ambiguous-diagonal, halo, mouth and streaming cases. Lakes and Box/mixed-axis views retain saved ownership; their quantization remains explicit. The 32K default export passes repeatability and unchanged-world checks. **Depends on:** OI-02. **Targets:** `crates/arda-render/src/atlas.rs`, `channels.rs`, `overview.rs`, `overview/streaming.rs`.

1. Inspect world and area views of the same coast/lake boundary against the saved wet cells and physical surfaces. Identify coarse terrain angles, 100 m stair steps and rasterization artifacts separately.
2. Decide which scale needs additional physical geometry and which needs better presentation. Larger PNGs alone cannot resolve either missing physical samples or poor edge styling.
3. For generation defects, correct the earliest responsible terrain/water stage and rerun relevant physical controls. For display artifacts, derive a consistent contour/coverage treatment from the authoritative geometry.
4. Preserve islands, narrow channels, mouth connections, lake identity and supported water levels. Never smooth a visible shoreline across a playable crossing while leaving collision/water elsewhere.
5. Test coastlines, small ponds, large lakes, mouths and area/band seams at several export qualities. Keep streaming and buffered rendering consistent.
6. Record the remaining quantization limit and any finer geometry proposed for tactical shorelines; do not describe cosmetic smoothing as a higher-resolution world simulation.

**Done when:** current shoreline evidence meets the declared world/area visual
target, topology remains correct, and any retained physical limit is explicit.

### OI-05 — Diagnose and correct reproduced rectangular regional basins

**Status:** traced September 23 to a real regional depression. The fine lake spans 30,709.15 km² at 1,177.501 m; the independently measured coarse basin spills at 1,173.357 m. Euclidean tectonic distance removes square belt geometry but does not remove this basin. No local lake-accounting or render-polygon defect was demonstrated. Regional terrain calibration remains open. **Depends on:** OI-02.

1. Trace each reproduced rectangular waterbody through final wet membership, basin geometry, fine terrain and coarse relief. Separate the shape of a valid filled basin from incorrect water support.
2. Check whether its sides follow real input relief, coarse interpolation, the modeled outer boundary or an internal publication boundary. Internal area boundaries must not behave as physical dams.
3. Correct the demonstrated stage. Do not clip or delete a lake merely because it is large or rectangular, and do not reinstate obsolete 100-cell/2 m thresholds.
4. Preserve real flat-bottomed bowls, connected basin hierarchy, positive-depth storage, exact annual balances, downstream transfers and global feature identities.
5. Compare default seed 42 and the wider panel, including seed436342's corrected lake district and crossing/corner basins. Measure shape and physical support separately.
6. Publish matched before/after evidence and residual resolution limits. If the finding is absent, record that disposition without speculative solver changes.

**Done when:** each reproduced artificial basin shape has an evidenced fix,
valid large lakes survive, and no cross-area water or annual-budget regression appears.

### OI-06 — Resolve coordinates and define shared layout/scene contracts

**Status:** design and implementation open. **Depends on:** none for contract
work. **Targets:** `crates/arda-core/src/{coords.rs,objects.rs,tiles.rs}` and
`crates/arda-core/src/formats/`; shared layout/scene types are **proposed**.

1. Resolve the physical mismatch: a 100 m terrain cell versus 64×1.524 m = 97.536 m tactical blocks. Evaluate an independent global five-foot grid or another explicit mapping; do not adopt one implicitly or stretch artwork as a fix.
2. Define origins, axes, physical units, rounding, negative/boundary coordinates, ownership and conversions among world positions, terrain cells, tactical chunks, rendered points and pointer picks.
3. Define stable settlement, route, crossing, parcel, building and object identities. Specify global ownership and cross-area references; reconcile global settlement IDs with today's `built_by: u16` field.
4. Store settlement extents, road alignments, street connections, parcels, footprints and required entrances as subcell geometry. The 100 m raster must not restrict a building to a 100 m square.
5. Define detailed scene surfaces, heights, water, walls/openings, doors, sparse objects, footprints and interactions. Support bridge decks over water explicitly; decide supported multi-floor cases before promising them.
6. Define optional versus required layers, layout revisions and completion states. Arda supplies semantic facts/geometry; DiceMaster owns rule-specific interpretation, assets and mutable campaign overlays.
7. Specify cross-boundary ownership and valid references before formats are published. Update the retained cell-plus-eight-neighbors block-input restriction to admit the canonical site plan where necessary.

**Done when:** contract fixtures round-trip under OI-23, neighboring chunks align
without drift, and one feature can be referenced consistently at every scale.

### OI-07 — Stage generation around shared geography and independent map outputs

**Status:** open. **Depends on:** OI-06; implement producer hooks alongside
OI-08–OI-20. **Targets:** `crates/arda-gen/src/orchestrator.rs`,
`crates/arda-gen/src/continent/bundles.rs`, public generation/loading interfaces.
Current `write_area` also generates sampled blocks; the new dependencies need
explicit orchestration rather than additional work hidden inside that function.

1. Document the dependency graph: physical world → ecology → settlements/land use/regional routes → local site plan → society binding and tactical refinement. Keep layout independent from artwork.
2. Define the planning domain and deterministic ownership for settlements/routes that cross areas. Introduce the shared inputs absent from current `TileBundle`, such as population allocation and route exits.
3. Split shared-plan production from tactical interiors/furnishings. Make completed world/area maps queryable and exportable even when tactical detail is absent.
4. Choose private intermediate staging or versioned optional sidecars so shared records can be finalized before publication. Preserve create-new semantics, bounded admission and the manifest-last completion guarantee.
5. Add bounded feedback for insufficient farmland, inaccessible sites and impossible building programs. Record shortfalls or revise the shared plan explicitly; do not regenerate the whole physical world for every failed room layout.
6. Specify layout-version invalidation and atomic publication of deliberate revisions. A completed world must never expose a half-updated combination of old roads and new footprints.
7. Exercise generation order changes and cross-border sites; keep physical terrain/water unchanged when only population or cosmetic settings change.

**Done when:** world/area exports need no tactical files, all downstream consumers
read the same finalized shared plan, and partial output is never advertised as complete.

### OI-08 — Produce ecological moisture, forest density and richer vegetation

**Status:** open beyond implemented Bare/Grass/Marsh cover. **Depends on:** saved
physical/climate fields; OI-06/OI-23 for schema changes. **Targets:**
`crates/arda-gen/src/area/{shared_compose.rs,temperature.rs}`,
`crates/arda-core/src/cell.rs`; ecology/vegetation modules are **proposed**.

1. Define ecological moisture units and normalization. Reconcile `Cell.moisture`'s soil-moisture meaning with the design's precipitation/evaporation-demand index; distinguish both from topographic wetness and atmospheric moisture.
2. Reuse produced rainfall, temperature, annual forcing, slope/aspect, HAND and wetness. Define dry/zero-demand behavior without silently replacing the accepted annual water calculation.
3. Implement deterministic vegetation suitability from warmth, moisture, elevation, soil/convexity proxies, aspect and water proximity, including regime-specific tree limits and cold/mixed/warm forest types. Decide whether aspect adjusts ecology only or persisted climate, and verify hydrology if the latter changes.
4. Generate pre-clearing forest potential and density using absolute-coordinate patchiness and neighboring terrain context, so area boundaries do not reveal themselves.
5. Define precedence for wetlands, exposed rock, alpine ground, permanent snow/ice cover, coasts and water. Static snow/ice classification is distinct from deferred dynamic snow storage. Decide which richer vegetation/surface classes need new fields versus objects; the current seven-value `Cover` enum includes Ice but does not represent every planned biome or land use.
6. Persist/export the new fields and provide diagnostic cover/moisture views. Preserve natural potential separately when OI-10 later clears or farms land.
7. Validate explainable climate/slope/aspect fixtures, cold-climate and treeline cases, density ranges, edge consistency and multi-seed distributions, while confirming authoritative water geometry is unchanged.

**Done when:** moisture/forest fields have meaningful producers and vegetation
forms coherent, reproducible distributions supported by the generated climate.

### OI-09 — Generate settlement placement, population and geographic names

**Status:** open. **Depends on:** OI-06–OI-08. **Targets:** existing configuration,
objects, bundles and orchestration; `continent/people.rs`, `continent/naming.rs`
and an area settlement producer are **proposed** under `crates/arda-gen/src/`.

1. Define coarse habitability/population allocation from configured density, including the area denominator, rounding, uninhabitable land and cross-area accounting. The artifact's roughly 40,000 inhabitants describes one area, not the entire default continent.
2. Compute suitability for water access, arable catchment, buildable slopes, safe elevation, sheltered coast, confluences and viable crossings. Apply refusal rules before positive scores.
3. Handle missing downstream-channel/HAND data explicitly; an absent measurement must not be interpreted as a proven safe terrace. Define acceptable flood exposure using the available static model.
4. Allocate population among hamlet/village/town tiers and any intended city tier, with explicit ranges, rank-size targets and deterministic rounding. Place larger centers first and enforce spacing across area boundaries.
5. Record unallocated population when suitable sites run out. Establish initial settlement extents compatible with land availability; finalize them with OI-10/OI-13 through the shared plan.
6. Assign stable IDs, truthful site tags and settlement names. Name connected major river courses, regions, ranges and seas/bays without giving every reach fragment a different river identity/name.
7. Persist objects, fill supported ownership fields and compute manifest statistics from real entities. Add queries/exports rather than leaving names only in a rendered label.

**Done when:** population accounting balances with explicit shortfalls, sites and
spacing are valid across boundaries, and stable IDs/names survive save/load.

### OI-10 — Allocate built-up land, fields and pasture

**Status:** open. **Depends on:** OI-08/OI-09 and shared schemas. **Targets:**
`crates/arda-core/src/{cell.rs,objects.rs}`, area composition;
a land-use producer is **proposed**.

1. Represent settlement extents, built-up land, farmland, pasture and ownership separately from natural vegetation. Do not encode fields as ordinary grass without retaining their meaning.
2. Resolve the retained 0.8 ha/person farm-allocation rule versus the approximately 1 ha/person plausibility target; document population denominator, productivity assumptions and shortages before calibration.
3. Allocate buildable extents by population/tier and suitable fields nearby, preferring eligible open ground before clearing forest. Allocate pasture where appropriate.
4. Resolve overlapping demands and area-boundary parcels deterministically. Preserve unique ownership and prohibit double-counting the same farm support for two settlements.
5. Apply explicit precedence among buildings, fields, pasture and natural vegetation. Preserve pre-clearing forest potential and protected water/terrain constraints.
6. Record unmet land demand and pass it to bounded settlement/site feasibility handling. Integrate roads/building parcels without silently stealing already-accounted land.
7. Expose allocated area, supported residents and shortages in queries, exports and calibration reports.

**Done when:** allocations are valid, contiguous where required, uniquely owned,
measurable and consistent across world/area views and population changes.

### OI-11 — Generate regional roads, crossings and passes

**Status:** open; `Cell.road` currently defaults. **Depends on:** OI-06–OI-10.
**Targets:** objects, bundles and orchestration; regional corridor planning and
`crates/arda-gen/src/area/roads.rs` are **proposed**.

1. Define a reproducible route-cost surface from terrain, grade, cover, land use and physical water widths. Convert the design's percentage-grade limits correctly; 30% grade is not 30 degrees.
2. Connect intended town centers with trunk routes, then villages and hamlets with lower classes. Reuse routes and crossing sites where appropriate instead of producing independent straight spokes.
3. Establish shared corridor IDs, precise area exits, widths/classes and graph connectivity. Preserve those records through later local street generation.
4. Handle islands and unreachable components explicitly. Roads cannot silently traverse sea/lakes; any ferry requires separate endpoints and its own connection semantics.
5. Classify water intersections as ford, bridge or ferry using actual channel geometry and route context. Distinguish these transport crossings from existing hydrology boundary `SharedCrossing` records.
6. Reserve approaches, landing points and enough space for supported bridge geometry. Derive passes from the route elevation profile and keep the evidence for crossing choices inspectable.
7. Validate route connectivity, forbidden surfaces, boundary agreement and generation-order independence. Export connected geometry for all three map scales.

**Done when:** intended reachable settlements connect through valid routes and
crossings, with consistent geometry and stable identities across areas.

### OI-12 — Substantially improve world-map geographical rendering

**September 23 continuation verified:** the authorized terrain and shoreline corrections are committed through `757b2ab`, with final integration tests, approved physical golden update, local lint/MSRV/dependency gates and two independent dry review rounds. Actual integrated small/default worlds and MICRO99 retain exact annual budgets; the full 32K default render repeats byte-identically. Current docs describe the corrected behavior. The first-pass renderer is working; broad mountain forms, large generated basins and repetitive drainage remain terrain-naturalness work, so the overall reference-quality objective has not been declared achieved. Local evidence: `features/2026-09-23-terrain-corrections/evidence/README.md`.

**Status:** first geographical rendering pass delivered September 22 at `342d03e55120`: earthy elevation palette, neighbor-aware shading, sea-depth colours and class-filtered per-output-pixel palette/light interpolation. The broader milestone remains open for ecological/human layers and multi-seed acceptance.
**Depends on:** existing physical data to start; OI-08–OI-11 for ecological/human
inputs; OI-13 to finalize published settlement extents; OI-02 for comparison. **Targets:**
`crates/arda-render/src/{channels.rs,overview.rs,overview/streaming.rs}`,
`crates/arda/src/export_quality.rs`; `crates/arda-render/src/atlas.rs` and `crates/arda/src/atlas.rs` now implement the relief/style context. Ecological/human overlays remain proposed.

1. Establish a separate world-scale reference, normal viewing size and comparison panel. The close-up tactical illustration alone does not define acceptable continent cartography.
2. **Delivered first pass:** consistent relief shading and natural elevation colours, using two-cell context from eight neighbors and explicit outer-world edges. Palette and lighting interpolate separately at each output pixel; water classes remain authoritative.
3. Improve coast/lake/river hierarchy, edge coverage and readability while preserving physical locations and water extents. Keep confirmed geometry corrections in OI-03–OI-05 separate from shading work.
4. Add forest, farmland and built-up appearance from real generated records as producers arrive. Define scale-dependent detail and contrast so geography remains readable under human overlays.
5. Render city/town/village extents and main roads from the shared geometry. Preserve physical extent; specify subpixel symbols/minimum strokes as presentation rules rather than enlarging the saved settlement.
6. Pass human/ecological object geometry into world rendering when its producers exist. Classic overview callbacks supply `AreaCells`; Atlas callbacks supply `(AreaCells, AtlasTerrain)`. Neither supplies settlement/building/road object geometry.
7. **Delivered for Atlas terrain:** bounded neighboring context and streamed rows/256-row bands, with preserved publication and quality limits. Repeat 32K exports peaked at about 43 MiB (area) and 65 MiB (overview) on the verification host. Future overlays must preserve these bounded structures.
8. Review matched-scale before/after outputs with and without human overlays, including multiple seeds and seams. Record visible improvement, resource cost and unresolved generation limits.

**Done when:** world views are substantially richer in geographical detail,
settlement outlines/main roads are readable and correctly placed, and progress
is demonstrably more than a larger image or extra road lines.

### OI-13 — Generate shared local streets, parcels and building footprints

**Status:** open. **Depends on:** OI-06–OI-11. **Targets:** shared objects and
staging; site-layout modules are **proposed** in `crates/arda-gen/src/`.

1. Convert settlement population, function, land allocation, terrain and regional approaches into a feasible local building/street program.
2. Lay out primary local streets, smaller roads/paths, public spaces and service access while preserving regional route endpoints and water crossings.
3. Partition suitable land into parcels and place purpose-specific building footprints, orientations and entrance/access anchors. Respect slopes, banks, setbacks and minimum usable interior dimensions.
4. Produce each cross-boundary street, building and bridge once under canonical ownership. Slice/reference its geometry in neighboring area/chunk outputs without independent rerolls.
5. Resolve insufficient space through bounded program adjustment or explicit layout revision. Finalize the settlement extent and population support before publishing the shared plan.
6. Publish building IDs, purpose and shell geometry before interiors. Reserve required access connections so later furnishing/WFC cannot erase them.
7. Check connected street/entrance graphs, footprint overlaps and alignment with the broader map. Demonstrate exports when no tactical layer has been generated.

**Done when:** a coherent site plan supplies all area-map outlines and tactical
inputs, with one shared identity/geometry for every building and route.

### OI-14 — Render useful area maps with buildings and local routes

**Status:** open overlays and viewing support. **Depends on:** OI-13 and relevant
OI-12 terrain styling. **Targets:** `crates/arda-render/src/{carto.rs,json.rs}`,
quality exports and public area queries; view-window/crop support is **proposed**.

1. Extend the existing cartographic path, which already receives `AreaObjects`, world origin and output scale, to draw local roads/paths and individual building outlines.
2. Define layering, clipping, line weights and minimum visible features. Preserve the world map's routes and settlement extents while revealing finer shared geometry.
3. Support settlement crops or view windows with correct physical scale. A full 51.2 km area at 8K is 6.25 m/pixel; that alone is insufficient for inspecting many small building outlines.
4. Expose building/route shapes through the versioned area JSON/object contract. Never require the consumer to trace an exported PNG to recover geometry.
5. Handle buildings/paths crossing areas and streamed bands without duplicates, gaps or clipped entrances. Reuse the canonical ownership rules.
6. Check representative village/town/dense-site crops and compare them with world and tactical views. Export successfully before interiors/furniture exist.

**Done when:** building outlines and local routes are legible at the declared
view scale and exactly match the shared plan later consumed by tactical generation.

### OI-15 — Bind realms and society to the shared geography

**Status:** open. **Depends on:** OI-09–OI-11/OI-13 and versioned schemas.
**Targets:** retained [society design](logic/06-society-generation.md);
`crates/arda-gen/src/society/` and Realm/Building/NPC codecs are **proposed**.

1. Define durable realm, building and NPC identities/references, and decide which game-neutral facts Arda stores versus any optional rule-system data.
2. Select realm seats and assign territory over the produced road/terrain cost surface. Treat retained realm-count and notable-count values as assumed, tunable defaults.
3. Handle zero-town worlds, disconnected islands, ties and inaccessible components. If borders snap toward rivers/ridges, preserve valid partitions and unique land ownership.
4. Create society's building records from OI-13's canonical footprint/purpose records. Move the retained post-block building dependency upstream; do not create a second building-layout generator.
5. Generate tier-appropriate notables and bind homes/workplaces/roles to real buildings and settlements. Add household relationships where required by the consumer without equating building count with population.
6. Derive other inhabitants on demand from stable world/settlement/entity keys. Stored NPC data should scale with settlements; reconstructions must not depend on request order.
7. Add exports/queries and content attribution when rule-system content is introduced. Resolve building-capacity shortages through the bounded shared-plan process, not endless world↔tactical regeneration.

**Done when:** realm ownership is valid, all society references resolve, building
geometry is shared with maps, and inhabitants reconstruct deterministically.

### OI-16 — Refine tactical terrain, banks and stacked movement surfaces

**Status:** open beyond tile-family selection. **Depends on:** OI-06/OI-13;
uses existing physical water/terrain. **Targets:** block input constraints and
**proposed** local terrain/surface producers under `crates/arda-gen/src/block/`.

1. Sample saved physical heights, water surfaces, channels, wetness and slope in the chosen coordinate system; carry layout routes, crossings and foundations into local constraints.
2. Add controlled banks, terraces, foundations, ditches, paths, steps and ramps. Interpolating 100 m samples is a base surface, not enough detail for the reference scene.
3. Preserve authoritative boundary conditions and water connectivity. Any fine bank shape must agree with world/area water semantics at crossings and shared edges.
4. Generate bridge decks, supports/clearance, approaches, docks and landings as explicit surfaces. Water and a walkable deck may occupy the same horizontal location.
5. Build traversable surface connectivity and declared elevation transitions before decoration. Distinguish impassable cliffs, wading water and supported routes through semantic attributes.
6. Validate boundary/corner scenes, roads meeting doors, both bridge approaches and water underneath. Keep geometry independent of PNG quality and chosen artwork.

**Done when:** local terrain supports coherent playable places, neighboring
scenes join, and supported vertical/crossing cases have explicit geometry.

### OI-17 — Generate interiors within the published building shells

**Status:** open. **Depends on:** OI-13/OI-16 and scene contracts.
**Targets:** **proposed** building/interior producers and the shared building record.

1. Consume each building's footprint, purpose, required entrances and access anchors. Define building-family rules for houses, farms, inns, warehouses and other initial supported functions.
2. Partition rooms and corridors with minimum usable sizes, wall thickness and circulation clearance. Fit the published shell instead of moving it to make a random layout succeed.
3. Place openings, doors, floors and supported stairs/elevation changes; distinguish visual roof treatment from the actual structure.
4. Generate a large building once, then reference/slice it across tactical chunks. Cross-chunk walls and rooms must not reroll independently.
5. Derive visual structural pieces and obstruction/movement geometry from that same layout. Keep required exits and intended room access verifiable before furniture placement.
6. For impossible programs, use bounded alternatives or an explicit shared-plan revision; invalidate dependent views if a footprint changes.

**Done when:** interiors fit their area-map outlines, required entrances connect
to local roads, intended rooms are accessible and chunk boundaries lose no structure.

### OI-18 — Replace permissive tactical WFC with meaningful constrained assembly

**Status:** prototype exists; full constraints/propagation open. **Depends on:**
OI-06/OI-13 and the relevant OI-16/OI-17 input masks. **Targets:**
`crates/arda-core/src/tiles.rs`, `crates/arda-gen/src/block/{constraints.rs,wfc.rs}`.

1. Define the semantic vocabulary from the supported terrain/building families. The retained 200+ tile target is a coverage ambition; increasing IDs alone does not improve composition.
2. Introduce directional edge compatibility, rotations and validated opposite-edge joins for structural pieces. Keep globally planned buildings/routes outside purely local adjacency unless a deliberate solver design incorporates them.
3. Supply per-position domains, fixed boundary/entrance pins, terrain masks, reserved routes and structure constraints. Replace the current one-allowed-list-for-all-squares input.
4. Propagate every domain reduction through a work queue until stable or contradictory. Current filtering reaches only uncollapsed immediate neighbors of the latest choice.
5. Add purposeful deterministic weights and tie-breaking. Separate semantic selection from cosmetic variation so changing decorative art cannot move walls or roads.
6. Measure domain propagation and cell selection at realistic vocabulary sizes; optimize bounded data structures from that evidence instead of copying the prototype's repeated full scan.
7. Validate multi-hop propagation, impossible pins, directional joins, rotation equivalence, repeatability and whole-site route constraints across seeds. Check accessibility separately where adjacency cannot guarantee it.

**Done when:** generated assemblies have coherent spatial structure and obey the
shared plan, with demonstrated propagation and bounded deterministic execution.

### OI-19 — Make contradictions, retries and fallback meaningful

**Status:** open; existing eight-attempt/first-tile fallback is insufficient for
future structural content. **Depends on:** OI-18 and mandatory layout constraints.
**Targets:** `crates/arda-gen/src/block/wfc.rs`, error/completion metadata and exports.

1. Separate mandatory boundaries, water, shells, entrances and routes from optional furnishing density or decorative preferences.
2. Construct nonempty incompatible-domain fixtures that genuinely exercise contradiction propagation and retries. The historical self-compatible-tile argument is not a general guarantee that greedy solving succeeds.
3. Define a bounded relaxation sequence that relaxes only optional preferences. Preserve mandatory geometry through every attempt and record its deterministic seed/attempt.
4. Decide whether impossible mandatory inputs produce a typed unavailable-site result or a proven safe simpler layout. Reconcile that choice with the retained design's promise that block fill cannot fail.
5. Export failure/relaxation reason and quality state. Never advertise a blank or uniform fallback as complete playable content just because an array was produced.
6. Revalidate required connectivity and boundaries after fallback; measure fallback frequency by environment in the seed panel and investigate systemic causes.

**Done when:** real contradictions exercise the policy, work is bounded and every
published playable fallback still satisfies mandatory geometry/access requirements.

### OI-20 — Add semantic attributes, POIs and purposeful furnishings

**Status:** open; current tile definitions are ID/name/group only.
**Depends on:** OI-16/OI-17, contracts and relevant OI-18 assembly.
**Targets:** `crates/arda-core/src/tiles.rs`, scene object formats,
`crates/arda-render/src/json.rs`; furnishing/POI producers are **proposed**.

1. Define materials, traversal classes, movement inputs, obstruction/cover geometry, heights and hazard/interaction tags. Keep rule-specific calculations in the consuming game.
2. Add stable sparse records for furniture, containers, large vegetation, shrines, campsites and other supported POIs. Specify footprints, orientation, ownership and any interaction anchors.
3. Place objects in functional groups: cargo near loading access, shelves against compatible walls, tables with chair space, vegetation where terrain allows it.
4. Reserve circulation and interaction clearances; recheck entrances, room paths, road widths and bridge approaches after furnishing.
5. Separate meaningful objects from visual scatter and give each deterministic seed domains. Cosmetic grass, stones and dirt must not acquire collision merely because their sprites overlap squares.
6. Export attributes and objects directly, with complete coverage of supported semantic kinds. Validate object references, bounds and save/load identity.

**Done when:** furnished scenes are believable and navigable, meaningful objects
have usable semantics, and cosmetic variation leaves gameplay geometry unchanged.

### OI-21 — Prove the camera, asset kit and illustrated target early

**Status:** open; run in parallel from milestone A. **Depends on:** an initial
OI-06 scene fixture, without waiting for full procedural generation.
**Targets:** DiceMaster's visual specification and asset pipeline; any Arda
asset-export contract is coordinated through OI-23. This is cross-repository
future work, not a claim that a renderer already exists.

1. Reconcile the supplied overhead illustrated map with DiceMaster's flat-fill/symbol specification and proposed 20° camera tilt. Compare camera choices using the same fixture before creating a large library.
2. Fix physical asset scale, normal play zoom, permitted rotations and lighting/shadow conventions. Test useful pixel densities rather than assuming more pixels make better artwork.
3. Create a small cohesive kit for a riverside warehouse/customs house: ground, banks, water edges, floors, wall joins, doors, bridge/dock pieces, trees and cargo/furniture.
4. Define a versioned catalog with asset IDs, semantic mapping, dimensions, pivots/anchors, layers, rotations, variants and source/redistribution information. Gameplay footprints remain authoritative scene data.
5. Build one authored structured scene with a bridge above water, interior, doorway, canopy and furnishing groups. Assemble reusable assets; a single painted background does not prove the pipeline.
6. Review with tokens, grid toggle, paths, fog and targeting overlays at normal zoom. Check seams, transparent margins, scale, repetition, baked shadows and readability.
7. Record the accepted benchmark, missing asset families and measured texture costs. Introduce import checks/atlas packaging based on that proof before expanding production.

**Done when:** an actual rendered, structured fixture convincingly approaches
the reference's visual richness and still communicates playable geometry.

### OI-22 — Render generated tactical scenes with layered illustration

**Status:** open; `symbolic.rs` remains a useful diagnostic renderer.
**Depends on:** OI-16–OI-21 and OI-23 contracts. **Targets:** DiceMaster's future
scene renderer and optional asset-backed offline export; `tileset.rs`-style
rendering is **proposed**, not delivered code.

1. Feed semantic scenes through the proven asset catalog, with deterministic visual variants and explicit handling for missing assets.
2. Render terrain blends, water/banks, floors, structures, furnishings, vegetation and shadows in coherent layers. Allow one object to span several squares and several layers to occupy one square.
3. Define depth sorting, cross-chunk rendering ownership, roof/canopy visibility and transparent cutaways. Large trees/bridges cannot be clipped to their anchor square.
4. Add controlled wear, edge blending, contact shading and scatter without moving geometry or duplicating incompatible shadows.
5. Keep grid, tokens, selections, fog, path previews and target templates as independent readable overlays. Preserve the intended exploration mode and camera interactions.
6. Compare generated instances with OI-21's authored benchmark across seeds, chunk boundaries and supported biomes. Retain symbolic/semantic debug views for diagnosing layout versus artwork problems.

**Done when:** generated sites meet the visual benchmark, remain readable in play
and display the same geometry used by simulation, with no baked-in token/grid state.

### OI-23 — Version and persist shared layouts, scenes and complete exports

**Status:** format-4 and current JSON implemented; new payloads open.
**Depends on:** OI-06; evolves alongside each producer, not only after rendering.
**Targets:** `crates/arda-core/src/{formats/,rng.rs}`, `crates/arda/src/world.rs`,
`crates/arda-render/src/json.rs` and facade exports.

1. Specify binary/JSON versions for shared plans and detailed scenes, including units/origin, world identity, layout revision, generator identity and layer completion. Decide explicit refusal versus migration for old formats.
2. Preserve stable IDs and canonical record ordering. Add deterministic stage domains for layout, interiors, furnishings and cosmetic variants without accidentally renumbering existing random streams.
3. Extend object/scene codecs and JSON with produced climate/ecology, settlements, roads, buildings, realms, NPC references, POIs and geometry as applicable. Current area JSON omits some already persisted cell fields; audit the contract field by field.
4. Distinguish absent, ungenerated, unsupported and corrupt layers. Validate references, enum values, counts, coordinates, lengths and decompression/allocation limits before exposing objects.
5. Define query surfaces at world/site/chunk scale rather than forcing every consumer to load an entire area's explicit JSON. Keep full exports available where appropriate.
6. Preserve immutable base-world publication and existing failure-safe PNG export. Cache representations by world/layout/generator/render/catalog versions and quality, not seed alone.
7. Keep campaign changes separate and keyed to compatible stable identities/layout versions. Define behavior when a saved campaign references a revised or unavailable base scene.
8. Verify round trips, repeatability across request order, unsupported-format refusal and failure publication paths. Pin artwork versions separately where reproducible presentation is required.

**Done when:** every implemented producer reaches a documented bounded query/
export, references round-trip, and neither artwork changes nor partial writes
silently alter published geometry or saved campaign interpretation.

### OI-24 — Replace sampled tactical coverage with an explicit delivery strategy

**Status:** open; current materialization uses a 64-cell stride and land-only
sampling. **Depends on:** representative detailed scenes and OI-23 measurements.
**Targets:** `crates/arda-gen/src/orchestrator.rs`, generation/query contracts.

1. Measure time, memory, storage and failure rates for representative detailed scenes before removing the stride. Calculate whole-world cost using the chosen physical chunk mapping.
2. Choose offline complete materialization, deterministic on-demand generation or a hybrid. Offline per-land-cell generation is the retained design; a different strategy needs an explicit documented decision.
3. Define coverage for wilderness, settlements, coasts, water crossings and intentionally unsupported environments. A bridge over a water cell cannot disappear under a land-only coverage rule.
4. Ensure large-site ownership and seed keys make adjacent results independent of request order and generation concurrency.
5. Specify completion/progress, interruption/restart behavior and resource admission for the chosen strategy. Do not silently treat current absence as a valid empty scene.
6. Exercise random coordinates, long journeys and boundary-spanning sites. Measure complete/pending/unavailable outcomes and retain diagnostic failure reasons.

**Done when:** every supported reachable location has a defined, tested path to
playable content, with declared generation cost and no accidental stride-sized holes.

### OI-25 — Bound per-scene I/O, decoding and caches

**Status:** lazy world/area loading exists; whole-area block decoding and retained
caches remain limits. **Depends on:** OI-23/OI-24 strategy. **Targets:**
`crates/arda-core/src/formats/blocks.rs`, `crates/arda/src/world.rs`, consumer caches.

1. Profile current archive reads, decompression and retained memory using realistic detailed payloads. Preserve manifest-first loading and owned area reads already available.
2. Introduce independently compressed frames with validated offsets/indexes, or another measured bounded-read layout. One chunk request must not require decoding every scene in a large area.
3. Validate offset/length bounds, duplicate keys, truncated/corrupt frames and decompression limits. Distinguish missing coverage from corrupted data.
4. Add bounded caching and eviction appropriate to library/server/browser ownership. Current `OnceLock` caches retain loaded archives; lazy access alone does not bound a long journey.
5. Prefetch nearby chunks/assets and cancel stale work without changing deterministic results. Handle cross-chunk objects through references with defined lifetimes.
6. Measure cold/warm access and long-session memory, including maximum-size sites and bad-file fixtures.

**Done when:** requesting a scene reads/decompresses bounded data, travel memory
stays within recorded budgets and missing/corrupt content produces useful errors.

### OI-26 — Implement read-only serving and the real DiceMaster transport

**Status:** open; CLI `generate`/`preview`/`export` and Rust queries already exist.
**Depends on:** OI-23/OI-25 and the chosen OI-24 behavior. **Targets:**
`crates/arda-cli/src/main.rs`, facade/load/export code; a server module is
**proposed**. [Serve design](mockup/06-serve.md) contains assumed endpoint names.

1. Reconcile those endpoint sketches with actual combined area JSON and the new world/site/scene payloads. Define supported representations, version negotiation, optional layers and asset delivery ownership.
2. Implement a read-only server reusing library loading/serialization/rendering. Decide whether the retained synchronous server model meets measured concurrency needs before adding runtime complexity.
3. Validate startup/completion, coordinate bounds, content types and missing/corrupt/unsupported layer responses. If on-demand generation is chosen, define its separate cache/job lifecycle without mutating the published base unexpectedly.
4. Define strong ETags or equivalent validation from representation identity/content. `(seed,path)` alone omits layout, generator, style and quality changes.
5. Bound cache size, request work, concurrent expensive exports and geometry/payload sizes. Specify browser origin/CORS and deployment configuration where required by the consumer.
6. Implement conditional requests and integrate DiceMaster's loaders. Test cold load, revisit, mismatched versions, absent chunk and delayed/cancelled navigation.
7. Document startup, shutdown, port configuration, world mounts and errors. Confirm responses agree with the corresponding CLI/library representations.

**Done when:** the consumer loads a real scene through the implemented contract,
caching is correct, and serving does not rewrite immutable world files.

### OI-27 — Integrate gameplay geometry, fog and campaign changes

**Status:** integration open in DiceMaster. **Depends on:** OI-16–OI-23/OI-26.
**Targets:** shared scene contract and DiceMaster's client/server spatial systems;
the sibling architecture is a design reference, not proof of implementation.

1. Bind client previews and authoritative server calculations to the same geometry and compatible scene/rule versions. Resolve movement, sight, cover and targeting from semantic surfaces/obstacles.
2. Verify coordinate conversion, picking, token footprints/heights and chunk transitions under the chosen camera. A visually plausible click must select the corresponding physical location.
3. Exercise walls/doors, furniture, trunks versus canopies, bridge decks/water and supported stairs with explicit expected paths and sight cases.
4. Integrate fog, visibility and explored-state persistence without exposing hidden scene information through rendering layers or client-only authority.
5. Apply mutable door/object/damage changes as campaign overlays with stable identities. Reload and revisit the same place without losing state or modifying Arda's immutable base.
6. Test a complete world→area→tactical journey and back, including a layout-version incompatibility and unavailable scene. Shared footprints/routes must remain consistent through zoom changes.

**Done when:** the map behaves as it looks, client previews agree with authority,
and exploration/campaign state survives revisits and boundary transitions.

### OI-28 — Measure and meet generation, transport and rendering budgets

**Status:** admission/streamed exports implemented; full-content and interactive
budgets unproven. **Depends on:** representative output from each producer;
measure throughout, then close after OI-24–OI-27. **Targets:** generation resource
accounting, export/loading benchmarks and DiceMaster instrumentation.

1. Inventory current measured budgets and retained targets separately. Existing release benches (continent ≤60 s, area erosion ≤30 s) are different workloads from proposed full-world generation ≤12 h, area export ≤60 s, block export ≤5 s and world load ≤100 ms targets.
2. Define hardware, world/scene sizes, cold/warm state and exactly what each timer includes. Ratify or revise proposed targets with evidence; do not silently apply a stage limit to complete detailed generation.
3. Extend pre-output resource admission to population planning, geometry counts, WFC domains, objects, archive indexes and export overlays. Bound new dense allocations and retry work.
4. Measure payload/network transfer, decode/parse, first useful view, CPU/RAM, texture/GPU memory and frame time separately. Full explicit area JSON can be tens of megabytes; demonstrate the browser path rather than assuming it is cheap.
5. Validate the retained DiceMaster targets of 60 fps desktop, 30 fps mobile, INP ≤200 ms and play-code ≤1 MB gzipped on declared devices. Specify separate asset/texture/network budgets; they are not included automatically in the code budget.
6. Use chunking, prefetch/eviction, levels of visual detail and appropriate atlases based on measurements. Reduce cosmetic cost on constrained devices while preserving identical meaningful geometry.
7. Test dense settlements, water/bridge boundaries, cold starts and long travel. Publish bottlenecks, achieved numbers and any explicit target changes.

**Done when:** the chosen delivery strategy meets recorded budgets on declared
workloads/devices without losing streaming, bounded memory or playable geometry.

### OI-29 — Complete statistical calibration and cross-scale acceptance

**Status:** physical/accounting/boundary/determinism tests exist; combined
calibration and new-content acceptance remain open. **Depends on:** each metric's
producer; starts with OI-02 and grows with milestones. **Targets:**
`crates/arda-gen/tests/continent_measures.rs`, existing tests, **proposed**
saved-world calibration/report suites, `docs/capstone/06-testing.md`.

1. Inventory existing checks so numerical accounting, persistence and determinism gates are retained. Separate new missing statistical coverage from tests already delivered.
2. Define metrics and sampling scale: Horton counts, Hack fits, drainage/shore/lake shape, settlement rank-size, road/straight-line ratios, cover fractions and farmland per supported resident.
3. Handle clipped networks, low sample counts, islands, absent settlements and unreachable routes explicitly. Measure canonical fine hydrology as well as any coarse probes; never mix their definitions silently.
4. Treat retained Horton 3–5, Hack near 0.55, open-ground road ratios 1.2–1.4 and farmland targets as scoped calibration hypotheses. Resolve OI-10's farm target and justify tolerances; no single statistic must be forced onto every area.
5. Use controlled fixtures plus frozen calibration and separate acceptance seeds. Report exclusions, sample sizes and uncertainty for larger offline surveys; keep smaller deterministic regression controls suitable for CI.
6. Add structural gates for settlement spacing, road/entrance connectivity, room accessibility, fallback validity, object references, coverage and request-order independence.
7. Pair those reports with matched world/area/tactical galleries. Include cross-boundary towns/buildings/bridges, several biomes and difficult seeds; verify the same IDs/geometry and independent pre-tactical world/area exports.
8. Retain before/after evidence and unresolved findings. Update golden baselines only for intentional reviewed behavior changes under the project's approval rule; never use rebaselining to hide unexplained drift.

**Done when:** every applicable metric has a reproducible definition and justified
acceptance scope, structural/physical checks pass, and visual review meets each
scale's target without unsupported claims from numerical tests alone.

### OI-30 — Finish CLI, packaging and release acceptance

**Status:** facade/CLI/Docker definition and CI delivered; release automation
and full-content acceptance open. **Depends on:** OI-01 and the capabilities
included in the release. **Targets:** current facade/CLI/Dockerfile,
`.github/workflows/`; release automation is **proposed**.

1. Audit the existing CLI/public API against newly implemented layers and queries. Add only missing entry points/options/help/errors; do not rebuild delivered generate/preview/export commands.
2. Decide supported binary targets, crate/image publication, version tags and format compatibility policy. Separate a tested local image from a published multi-architecture release.
3. Implement the planned release workflow with pinned/reproducible build inputs and appropriate artifact/version metadata. Keep credentials and actual publication in the release process.
4. Smoke-test install/load/generate/export paths on target platforms. Exercise the non-root Docker image with mounted world/input/output volumes; document UID/permissions and add serving configuration when OI-26 exists.
5. Verify packaged artifacts can read their advertised saved format and produce the same representations as source builds. Check image/binary architecture and startup errors.
6. Link actual CI/release runs and update README, operations, API and status docs from verified results. Retain historical failures as dated evidence, not current blanket status.

**Done when:** chosen release artifacts are built and verified through the
documented workflow, distribution status is accurate, and all release gates pass.

### OI-31 — Expand environments and building families after the first complete site

**Status:** later content expansion. **Depends on:** a generated, playable
reference-quality site and OI-29 acceptance machinery. **Targets:** generator
templates/rules, semantic vocabulary and DiceMaster's asset catalog.

1. Choose successive supported families: wilderness/forest, farms/villages, town streets and civic/commercial buildings, then additional climate/cultural variants.
2. Add semantic layout and gameplay requirements before artwork. Ruins, caves, mines or multi-level sites require explicit supported generation/geometry decisions where the current project does not produce them.
3. Extend ecology/land-use/site rules, purposeful furnishing groups and the cohesive asset kit together. Preserve scale, camera, lighting and vocabulary completeness.
4. Exercise each family across multiple seeds and boundaries, including difficult terrain and constrained sites. Track missing assets and fallback frequency by family.
5. Reuse cross-scale IDs, layout publication, performance and visual gates; reject diversity that reintroduces incoherent roads, repeated noise or unusable interiors.
6. Publish a coverage matrix listing supported, partial and unsupported combinations. Advance each family only when generated examples meet the benchmark in normal play.

**Done when:** each advertised family has reproducible generated examples,
complete semantics/assets and the same geometry, visual and performance acceptance.

### Scope boundaries and conditional extensions

These are explicit future choices, not hidden blockers for the map pipeline.
They must not make delivered annual hydrology or loading appear unimplemented.

| Boundary | Current disposition | Steps if the scope is activated |
|---|---|---|
| Seasons, snow storage, groundwater, dynamic floods | Outside the representative static annual model. | Specify the process/time step and required inputs; extend water/energy stores and exchanges; add versioned state/output; verify conservation and equilibrium/transient fixtures; expose the resulting seasonal gameplay/render meaning. Do not fold this into OI-08's ecological moisture by accident. |
| Finer world physical raster | Current shared terrain is 100 m; subcell object geometry and local tactical refinement are separately required. | Establish a reproduced need that OI-04/OI-16 cannot meet; measure finer-domain cost; choose multiresolution/global policy; redefine terrain/water boundaries and formats; validate conservation, seams and resource admission. Higher PNG quality is not this feature. |
| Region-file import/front door | Retained optional design, not required for generated worlds. | Specify supported source schema/units; validate and map inputs to canonical world/layout records; define deterministic conflict/error handling; test the same exports and geometry invariants as native generation. |
| Mutable in-process game API | Arda's generated base is immutable; campaign mutations belong to DiceMaster. | First specify ownership and transaction/persistence semantics; build an overlay keyed to stable IDs/layout revisions; validate replay/concurrency/version conflicts. Do not replace the existing read-only query API just to store door state. |
| General checkpoint/resume and parallel world preparation | Current physical preparation/composition is sequential; no broad resumability promise is made here. OI-24 still must define interruption behavior for its chosen coverage strategy. | Profile actual need; identify deterministic checkpoints/partition boundaries; version and validate partial state; test interrupted/resumed equivalence and bounded resources before advertising support. |

### Coverage of every earlier inventory item

Closed items are preservation obligations, not new implementations. The historical
text below is retained verbatim so its evidence and changing diagnoses remain
traceable; its present-tense claims do not override this queue.

| Earlier item | Current disposition and action |
|---|---|
| #1 — tile-sized major-river cap | Closed by entering context and later shared hydrology. Preserve upstream catchment/flow and crossing identity through OI-03–OI-05, OI-16 and OI-29. Do not rebuild a one-area solver. |
| #2 — salt-and-pepper WFC | OI-13/OI-16/OI-17 establish spatial plans; OI-18 implements constrained assembly; OI-20–OI-22 supply furnishing/art. |
| #3 — fallback not meaningfully exercised | OI-19 adds real contradictory fixtures, bounded relaxation and explicit quality/failure states. The old self-compatibility explanation is not a future correctness proof. |
| #4 — noise/terrain directional diagnosis | OI-02/OI-03 reproduce and localize current defects; OI-04/OI-05 cover the other later visual findings. The old interpolation/noise diagnoses are not current prescribed fixes. |
| #5 — rainfall/discharge stand-ins | Closed. Preserve produced climate, rainfall-driven flow and annual accounting in OI-08/OI-16/OI-29; do not restore the old fixed conversion as the final annual model. |
| #6 — empty overview | Closed; current format-4 records supersede historical format-3/18-byte records. OI-12 enriches presentation and object inputs; OI-23 preserves real saved-world provenance. |
| #7 — sparse block stride | OI-24 chooses and implements coverage, supported by OI-25/OI-28 measurements. |
| #8 — eager world loading | Closed by manifest-first/lazy/owned reads. OI-25 targets per-archive decoding and retained-cache limits, not a nonexistent whole-world eager load. |
| #9 — missing statistical suite | Physical tests exist; OI-29 completes the combined statistical, structural and visual acceptance work. |
| #10 — old lake-size/depth thresholds | Superseded by physical fine basins and annual water support. OI-05/OI-29 preserve positive-depth storage/accounting and measure shapes; do not recalibrate obsolete thresholds. |
| #11 — pinned/tapered internal erosion rim | Resolved by shared-domain evolution. OI-03–OI-05/OI-29 preserve internal-boundary equivalence and absence of seam water rims. The true modeled outer rim is a different boundary. |
| #12 — cross-area lake authority | Resolved by shared topology/global IDs/surfaces, including natural checks. Preserve boundary/corner identity and physical surface consistency under OI-05/OI-16/OI-23/OI-29. |
| Later cell-field gaps | OI-08 supplies moisture/forest ecology; OI-09–OI-11/OI-13 supply settlement ownership/roads/land use. Temperature, rainfall and water metrics already have producers. |
| Later CI/visual evidence gaps | OI-01/OI-02; missing old local reports do not reverse delivered features or imply all previous checks failed. |
| “Not defects” records | Identical empty ocean archives can be valid; preserve semantic correctness rather than requiring every file hash to differ. The historical pinned-seam smoothness measurements are dated, not current production terrain requirements. |

| Original build-plan step | Detailed remaining work |
|---|---|
| 0–3 | Delivered foundation; preserve it through relevant regression checks. |
| 4 — continent full | Human allocation/naming OI-09/OI-11; remaining physical findings OI-02–OI-05; statistical acceptance OI-29. Climate/drainage/validation already exist. |
| 5 — area stages | OI-08–OI-14; delivered shared physical fields/basic cover remain credited. |
| 6 — complete blocks/vocabulary | OI-06/OI-07/OI-16–OI-19/OI-24; convergence alone is not visual or gameplay acceptance. |
| 7 — society | OI-13/OI-15/OI-17: one upstream shell/layout authority and downstream interior/social binding. |
| 8 — attributes/POIs | OI-20, with persistence OI-23 and gameplay OI-27. |
| 9 — full export | OI-12/OI-14/OI-22/OI-23; retain existing PNG/JSON and quality streaming. |
| 10 — facade/CLI/Docker | OI-25/OI-28/OI-30; delivered APIs/tooling are not rebuilt. |
| 11 — serve | OI-26 and actual consumer integration OI-27. |
| 12 — validation/performance | OI-01/OI-02/OI-28/OI-29 and each item's specific completion gate. |

| Companion roadmap step | Detailed queue coverage |
|---|---|
| 1 — visual target | OI-02/OI-12/OI-21 |
| 2 — scale/coordinates | OI-06 |
| 3 — scene/ownership | OI-06/OI-07/OI-23 |
| 4 — starter art | OI-21 |
| 5 — authored proof | OI-21/OI-27 |
| 6 — world-to-site inputs | OI-08–OI-11/OI-15 |
| 7 — shared site layout | OI-13/OI-14 |
| 8 — tactical terrain/water | OI-16 |
| 9 — interiors | OI-17 |
| 10 — WFC | OI-18/OI-19 |
| 11 — furniture/decor | OI-20 |
| 12 — rich rendering | OI-12/OI-14/OI-22 |
| 13 — versioned persistence | OI-07/OI-23 |
| 14 — gameplay | OI-27 |
| 15 — coverage | OI-24 |
| 16 — streaming/performance | OI-25/OI-26/OI-28 |
| 17 — multi-seed acceptance | OI-02/OI-29 |
| 18 — content expansion | OI-31 |


## Historical C06 snapshot — 2026-09-08

The snapshot below preserves the original observations and measurements. Its
present-tense statements refer to September 8. Linked feature-local reports and
images are not present in this checkout; current implementation and verification
status are recorded above.

The area-water-terrain implementation is installed. Candidate06 corrects the
altitude-dependent detail amplitude that repeatedly manufactured shallow basins
on gentle high terrain. Detail now follows surrounding regional height differences,
using the existing bounded sampler and noise. The annual water rules, saved format
and renderer are unchanged.

The reported seed436342 changes from 2,653 to 186 lakes; fixed 50 km squares with
at least 50 lake anchors fall from 18 to zero. Its former densest square changes
from 158 to 3. These are measurements, not production quotas. The 16K comparison
visibly removes the repeated pond patches. Total wet area changes from 5,028.39 to
2,558.01 km²; substantial regional lakes remain.

All five frozen worlds, 38 selected JSONs and 238 exports pass, including repeated
bytes, unchanged saved worlds, exact annual balances and shared wet-boundary
identity/surface checks. The combined workspace, focused repairs and fresh
approved C06 golden comparison cover 590 passing tests. The terrain correction
and its deterministic baseline are committed as `ed4875d`. Parallel ravines, angular shorelines and large
rectangular regional basins remain open; the overall feature is not marked done.
Evidence: [C06 comparison](features/2026-09-07-area-water-terrain-realism/reports/lake-district-correction--data-comparison--REPORT.md)
and [maps](features/2026-09-07-area-water-terrain-realism/output/current/lake-district-correction--gallery-c06.md).

| Item | Current evidence and disposition |
|---|---|
| Terrain shape and incision (old #4) | Candidate04 evolves one physical rectangle, applies implicit downstream-first incision, removes the unresolved two-cell detail octave and uses a radial coast mask. A frozen-input replay reproduced the numerical pit defect exactly and the implicit update removed it while controlled physical bowls survived. The radial mask removes a verified planar coarse flank; current default and MICRO images still show regular parallel drainage. The large rectangular seed42 lake follows an existing coarse basin. No target direction percentage or arbitrary river-count reduction is used; overall visual acceptance remains open. |
| Lazy loading (old #8) | Implemented: manifest-only `World::load`, requested area/archive caches, uncached owned area reads for exports. Actual missing/corrupt/sparse-file fixtures verify later I/O failures and admission checks. |
| Lake thresholds (old #10) | Canonical generation uses fine-grid depressions and an annual water-support calculation, including positive-depth physical storage, rainfall, runoff and evaporation. The historical 100-cell/2 m rule no longer controls final world lakes. |
| Cross-area lake authority (old #12) | Shared fine topology, global IDs, physical surfaces and copied feature records replace nearest-coarse-basin/max-surface reconciliation. Constructed boundary/corner controls and all 38 selected natural record comparisons pass. |
| Tactical noise and fallback (old #2, #3, #7) | Remain open. Existing 24-tile WFC, permissive adjacency and 64-cell sampling stride are unchanged. Detailed assets, movement/collision geometry, NPCs and server/browser transport require later work. |
| Erosion rim (old #11) | Candidate02 seed436342 area3,10 had 50.1% wet rim cells versus 6.2% wet interior. Production now evolves terrain across publication boundaries without pins or taper. Candidate04 gives 4.89% wet rim versus 4.95% interior; candidate05 gives 6.51% versus 6.45%. Across all 513 candidate05 default areas, every one of 21,772 adjacent pairs wet on both sides has matching IDs/surfaces; candidate06 checks 20,712 such pairs with no mismatch. The continuous water square is absent in the current preview. Only the true modeled outer rim remains fixed. See `features/2026-09-07-area-water-terrain-realism/reports/terrain-correction--visual-default436342-c05.md`. |
| Statistical calibration (old #9) | Existing drainage, cross-tile, terrain and determinism suites run. This work adds controlled physical/annual/resource/geometry checks and a frozen natural panel; it does not supply the full proposed Horton/Hack/rank-size/sinuosity/farmland calibration suite. |
| Cell producers | Temperature, rainfall, wetness, water and terrain metrics now have producers. Cell.moisture, full vegetation, human geography, roads, buildings and society remain deferred. |

Accepted physical/rendering limits are the 100 m terrain lattice, quantized pond footprints, thin channels rendered with physical area coverage, coarse angular shorelines, sequential world preparation/composition, and a representative static annual balance. Snow storage, groundwater, seasons and dynamic floods are absent. Full area JSON is intentionally explicit and can be tens of megabytes; no browser parse or network-loading budget has been demonstrated. See the current [models](02-models.md), [testing](06-testing.md), [operations](07-operations.md) and [export behavior](logic/04-export.md).

## Historical inventory — through 2026-08-27

The entries below preserve the diagnosis and measurements that motivated later work. The current table above supersedes their implementation-status claims.

## Blocking realism

| # | Item | Evidence | Owner |
| --- | --- | --- | --- |
| ~~1~~ | **Closed by feature 03** (2026-08-27): entering rivers seed each tile from the continent drainage tree. Measured max area-cell catchment at default size 50,070 km² against the old one-tile ceiling of 2,621 km². | Max catchment anywhere = 2,207 km²; hard ceiling is one tile at 2,621 km². A UK-scale major basin is 9,385 km² — **3.6× larger than a whole tile**, so the continent cannot produce even one of the 22–30 systems a UK-sized landmass should have. | `continent/bundles.rs` (bundle has only edge heights), `area/water.rs` |
| 2 | **The WFC tactical layer is salt-and-pepper noise.** Converges, satisfies adjacency, deterministic — and has no spatial structure, because `may_adjoin` lets anything within one "wetness rank" touch, so nearly every tile is compatible with nearly every other and collapse degenerates to uniform random choice. | `export --block 3,7,192,256` on any world. | `arda-core/src/tiles.rs::may_adjoin`, `arda-gen/src/block/wfc.rs` |
| 3 | **The relaxed-fallback ladder is unreachable.** Every tile is adjacency-compatible with itself, so any non-empty constraint set tiles trivially; only an empty set fires the fallback. `logic/03` specifies a ladder that cannot trigger in practice. | Test `an_impossible_constraint_set_falls_back_to_a_marked_relaxed_fill` has to pass an empty set. | `arda-gen/src/block/wfc.rs` |
| 4 | **Value noise is anisotropic.** Its gradients favour the square lattice's axes, so steepest descent does too and rivers tend to straight runs. | ~17% of flow directions diagonal against an isotropic ~50%, measured on raw relief before any erosion. | `arda-gen/src/noise.rs` |

On (4): **the recorded diagnosis was wrong, and is corrected here.** A
measured sweep (2026-08-27) found the diagonal-flow share is 23.2%
post-erosion / 38.1% pre-erosion, not the "~17%" recorded — and zeroing the
value-noise detail term entirely barely moves it (21.3%), which proves the
detail noise is **not** the dominant source. The axis bias comes from
`coarse_height`'s own separable smoothstep-bilinear sample of the 1 km grid,
which is what every area cell's regional trend is built on.

Attempts, all measured: per-octave domain rotation (the previously "untried
alternative") moves the share to 23.9% — the floor-then-lattice-lookup
reconstructs an axis-aligned staircase, defeating the rotation; rotation plus
per-octave offset, 23.8%; domain-warping the detail term alone, 23.4–24.0%.
Warping `coarse_height`'s sample position as well reaches 28.6% and visibly
reduces the combs at area zoom — **but it was rejected on the render**: at
default size it speckles the coastline, scatters noise-like micro-lakes
through the interior, and weakens the trunk hierarchy. That is the same class
of regression that killed the first attempt, so it was reverted rather than
shipped.

What the evidence now points at: the combs are strongest on smooth mountain
flanks, where a near-planar slope sends every cell the same way and no
convergence forms. That is a *terrain-shape* problem (too little fine-scale
valley structure for erosion to organise), not purely a noise-isotropy one.
A real fix likely needs the erosion budget or the relief construction
revisited, which is a feature with a spike, not a constant to tune.

## Stand-ins awaiting a producer

| # | Item | Current behaviour | Unblocked by |
| --- | --- | --- | --- |
| ~~5~~ | **Closed by feature 03** (2026-08-27): `Cell.rainfall` is written from the bundle's 1 km rainfall patch; discharge is `Σupstream rain × 125/788,400` L/s plus entering rivers, and channels initiate at 40 L/s. The artifact's "3 km² ≈ 40 L/s" equivalence is now emergent rather than assumed. | — | — |
| ~~6~~ | **Closed by feature 02** (2026-08-26): `overview.bin` carries real 18 B/cell records (relief, climate, drainage) and `continent/objects.bin` carries rivers, at format 3. | — | — |
| 7 | Blocks are materialised on a **64-cell stride**, not one per land cell. | `mockup/02` specifies per-land-cell. | Build-order step 6 |
| 8 | `World::load` reads **every** area and block eagerly. | `logic/05` specifies lazy access with an O(accessed) cache. | Build-order step 10 |

## Calibration held open

| # | Item | Note |
| --- | --- | --- |
| 9 | **No statistical validation suite.** No Horton, Hack, rank-size, sinuosity, or farmland gate exists. | Erosion and lake constants are calibrated only against the artifact's two stated equilibrium anchors (1 km² → 9% slope, 100 km² → 1%) and against Earth's hypsometric curve — not against network statistics. Build-order step 12. |
| 10 | **Lake thresholds** (100 cells, 2 m) were derived from basin distributions measured on **pre-erosion** relief. | Erosion reshapes that distribution; re-derive when (9) lands. |
| 11 | **Tile-edge taper band.** Area erosion ramps to zero over 32 cells at the pinned rim, so the 35° repose rule is not enforced there (measured tan×1000 of 1,324 inside the band against exactly 700 at full strength). | Structural: per-tile erosion must freeze tile edges for neighbours to agree byte-for-byte. Seamless tiled erosion needs a global pass or a proven-decay overlap scheme. |
| 12 | A basin straddling a tile seam: **mechanism now exact, precondition unobserved** (2026-08-27). The continent tier emits lake identity — `ContinentHydrology.basin_surface` gives every 1 km cell inside a filled depression that depression's single surface — and a near-rim area basin takes its lake surface from a nearest-cell (never interpolated) lookup of that constant. Two fragments of one depression therefore agree **exactly**, verified at 0 mm on a constructed two-tile case against the real `compose` path. | Residues, both unobserved: no fixture has yet produced a straddling basin whose cells see a continent depression at all (0 in a 4,280 seed/seam sweep), so the exact path is proven on constructed input rather than natural data; and a fragment whose rim abuts two *different* depressions takes a max of two constants, which is span-dependent again. Where the continent tier sees no depression (sub-km pits invisible at 1 km) the old bilinear rule still applies, pinned at 723 mm on the synthetic case. |

## Groomed but not built

- **Feature 02** — continent climate and hydrology: **built** (2026-08-26,
  commits c2c0d2a..9c48e00; spike S1 discharged by the recorded Horton
  measurements). (1), (5), (12) are now unblocked, not closed.
- **Feature 03** — climate-driven refinement: **built** (2026-08-27,
  commits 78bbff5..8be0a0a). Closed (1) and (5); improved (12); also
  landed the step-10 bundle payload, the step-9 river gate, and
  order-banded river rendering. Five review rounds to dry.

**Next, in the order the evidence suggests:** (4) the value-noise
anisotropy is now the most visible remaining river defect — at area
zoom, streams still run as straight parallel combs along the lattice
axes, which is what "rivers look like fjords" describes at close range.
Cross-tile continuity and hierarchy are fixed; the *shape* of an
individual stream is not. The recorded fix (isotropic gradient noise)
was tried once and reverted for blocky coastlines, so it needs its own
feature rather than an in-place patch.

## Historical unbuilt-step inventory — August checkpoint

This table preserves the August checkpoint, not the current work queue. Its
default-temperature/rainfall claim and blanket pending export/facade stages
were superseded by later implementation. Use the current status above and the
reconciled states in `implementation.md` for remaining work.

| Step | Scope |
| --- | --- |
| 4 | Continent full — **climate, hydrology objects, and S1 done** (feature 02); remainder: human geography, naming, validation stats. |
| 5 | Area stages 3–7: climate, vegetation, settlement, land use, roads. Also `Cell.temperature`, `rainfall`, `moisture`, `forest_density`, `road`, `built_by`, all still at `Default`. |
| 6 | Blocks full: 200+ tile vocabulary. Spike S2 first — note S2 was scoped to *convergence*, and (2) above shows convergence was never the risk. |
| 7 | Society: realms, buildings, NPC sheets. |
| 8 | Tile attribute table + POI layer. |
| 9–12 | Export full, facade/CLI/docker polish, `arda serve`, statistical gates. |

## Not defects

Recorded so they are not re-investigated: the pinned tile seam is **smoother**
than the interior (p90 2,009 mm at the seam against 7,400 mm ten cells in,
land-to-land), and two all-ocean tiles legitimately share identical
`objects.bin` and block-archive hashes.

### Latest bounded mountain-source trial

A single crest-following source trial adds two saddle-defined summits that survive the unchanged160-step kernel, retaining259m and197m prominence. Rim/ocean cells remain exact and no new closed-basin cells appear. Root inspection of the actual Atlas comparison still finds smooth elongated rises and repetitive comb-like gullies, so this trial is rejected for production adoption. [Matched evidence](features/2026-09-23-terrain-corrections/evidence/crest-segment-source/README.md), [root visual assessment](features/2026-09-23-terrain-corrections/evidence/crest-segment-source/root-visual-assessment.md).
