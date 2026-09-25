# Direction-consistent flow proposal

Implemented an isolated derivative-only donor selector and temporary
face-tangent constraints to address the measured flow-reversal cycle. Physical
residuals remain unchanged and constraints are reselected for each proposal.
Nine candidate tests pass, including signed donor limits, release and exact
ordinary-CG parity where branch handling is unnecessary.

The bounded local check reproduces the prior rejected candidate exactly, then
accepts a smaller step through the existing trust-radius update. Independent
replay passes. Map error decreases slightly while raw momentum increases
slightly; neither terminal flow gate passes. A separately bounded continuation
completed from the exact accepted state and next radius, stopping at its
fifty-step double plateau after 53 trials/345.02 seconds. Independent checks
cover all 51 states and 53 decisions. The reversing-face constraint releases,
but raw momentum and map gates still fail.

No terrain update, new render or production adoption is claimed. Current status
and the feature plan record the remaining validation.
[Evidence](../features/2026-09-23-terrain-corrections/evidence/flow-branch-aware/CONTROL-DESIGN.md).
