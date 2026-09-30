# Flow continuation and switching-face diagnosis

Completed and independently verified the bounded dogleg continuation: 500
additional accepted steps, 614 trials, 1463.77 seconds, still unqualified.
A tested projected-CG step improves a matched local comparison, but its bounded
60-step control also fails convergence after stalling at a reversing upwind
face. All retained fields and decisions in both runs verify independently.

Fixed-state directional checks show that the collapsed radius is not proof of
stationarity. A single local face-tangent comparison prevents that reversal but
still fails actual acceptance while another face switches. No erosion, new
terrain render or production adoption follows. Updated current status and the
feature plan to replace the stale running checkpoint with terminal evidence.

[Continuation result](../features/2026-09-23-terrain-corrections/evidence/flow-map-continuation/root-assessment.md)
and [Krylov result and limits](../features/2026-09-23-terrain-corrections/evidence/flow-krylov-control/root-assessment.md).
