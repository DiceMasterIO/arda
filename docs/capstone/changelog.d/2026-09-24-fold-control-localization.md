# Localized broad-face diagnosis

Mapped independently marked face boxes and cliff polylines to existing mesh constraints using the actual Atlas sample-center transform. Most marked lines track original ridges more closely than rivers; this is localization, not proof of a defective ridge model.

An exact original-mesh replay recovered pre-smoothed refined river heights omitted from earlier diagnostic metadata. The source uses these for initial Poisson interpolation while fixing river vertices to smoothed heights. The algebraic discrepancy has a crop median 0.488 m and exceeds 100 m at only 4 of 6,548 interior samples. It is not selected as the main broad-fold fix; corrected smoothing summaries remain close to previous values. No modified terrain, production code, or saved-world write occurred.

The user prefers the rugged preserved baseline over the smoother recipe. Status and plan now preserve that preference and distinguish the completed diagnostic from appearance acceptance. [Evidence](../features/2026-09-23-terrain-corrections/evidence/fold-control-localization/README.md).
