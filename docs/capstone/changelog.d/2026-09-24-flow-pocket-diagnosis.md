# Flow pockets and a nonlinear velocity-error control

Diagnosed opposing cell velocities that nearly cancel at shared faces, along with strongly unequal common/antisymmetric water and momentum sensitivity. Implemented an isolated implicit nonlinear-map objective with unchanged physical equations and depth-aware trust geometry; three small tests, measured derivative/local-step checks and independent review pass.

The fresh 128×128 control stops at 60 accepted steps/83 trials in 178.61 s without convergence. Maximum velocity-map error improves from the prior endpoint's 20.43 to 1.407 m/s, but raw scaled momentum error worsens from 90.90 to 1924.12 and errors become more widespread. All 61 saved states and 83 decisions independently reconcile. No terrain update, new render, production change or appearance acceptance.

Updated current status and feature plan. [Evidence](../features/2026-09-23-terrain-corrections/evidence/flow-pocket-diagnosis/root-assessment.md) retains the failed run, limitations and the next connection to an actual terrain test. The full reference-quality objective remains open.
