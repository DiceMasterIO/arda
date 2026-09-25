## 2026-09-23 — Landform source diagnostics

key: map/landform-source-diagnostics@92689f8a8142

- Reproduce the current continent stages and all 500,000 saved 1 km heights in an isolated collision-source control. Reject the single warp-Jacobian concentration variant: principal crest widths remain 7/23 km, with one summit above the declared 150 m prominence threshold. No fine run or production adoption follows.
- Audit saved lowland/highland samples independently. Structural relief is provably disabled at at least 54.2% of sampled coastal 1 km nodes and 28.8% of mixed-river nodes. These bounds describe the sampled nodes, not full map areas. Fine relief also vanishes on locally flat regional input.
- Record separate hill and cliff limitations: steep-face relaxation has no bedrock/resistance distinction, and the rock material can only display slopes already present. Neutral hillshade and actual Atlas views are both required for visual acceptance.
- Inspect the authors' MIT peak/saddle terrain reconstruction source at `c85d6ca393339f04e09097d6cc151c896763d271`. Record its omitted erosion stage, floating-point geometry and dense reconstruction matrices as constraints on direct reuse. No external implementation is installed in Arda.
- Update current status and the active correction plan. Preserve all installed terrain/river improvements and immutable saved worlds. The full reference-quality objective remains open.
- Reject a second controlled source-envelope trial with separated anchors and reduced inter-anchor uplift. It does not meet the predeclared multiple-summit gate; no fine/full-world run or production change follows. Source weighting does not implement the paper's explicit dual ridge/river construction.
- Verify the independent summit-prominence diagnostic against 2,000 random fields and eight targeted cases. The actual saved 100 m highland crop also retains one interior summit over the declared 150 m threshold; finite-crop prominence remains conditional on missing outside paths.
