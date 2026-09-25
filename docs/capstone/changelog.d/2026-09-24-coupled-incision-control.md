## terrain-corrections/coupled-incision-control@2026-09-24

- **Defect:** old uplift-then-exponential incision gives a timestep-dependent fixed-route equilibrium. Implement an isolated opt-in receiver-first backward-Euler correction with unchanged source, mesh, material, diffusion and timestep.
- **Checks:**32 offline scratch tests reported,7 independent exact-rational scalar cases, separate Sol review, exact old-mode400-step replay, and one completed candidate with saved-field source/bed/hull/ledger/final-diffusion checks. Actual control renders reproduce the prior files exactly.
- **Result:** native geometric depressions shrink substantially, but actual Atlas retains taller folded faces and repeated gullies. Peak4856→7672m. Retain the numerical correction for research; reject this terrain's production appearance adoption.
- **Scope:** no production code or original saved-world change, no new source/parameter sweep and no full-world completion claim. Evidence: features/2026-09-23-terrain-corrections/evidence/coupled-incision-control/. Full reference objective remains open.
