# Native mesh smooth-normal comparison

- Completed a scratch-only vertex-normal lighting comparison on the original frozen native TIN, with identical heights, material, camera and occlusion.
- Independently verified all rendered vertex gradients and signed normal numerators; prior sampled/flat-face output bytes and halo masks replay exactly.
- Vertex normals remove flat-face triangles but soften terrain and leave dense branching landforms unchanged. Reject production adoption; the reference-quality terrain goal remains open.
- Production code and saved-world fields remain unchanged. Evidence: `features/2026-09-23-terrain-corrections/evidence/native-smooth-light-control/root-assessment.md`.
