## 2026-09-22 - groom: 2026-09-22-geographical-rendering-first-pass
key: groom/2026-09-22-geographical-rendering-first-pass@Q5

- `features/2026-09-22-geographical-rendering-first-pass/spec.md`: add per-output-pixel palette and lighting interpolation with separate class-filtered sampling.
- Read two-cell edge/corner context from eight saved neighbors; preserve physical water masks, Classic behavior and bounded row/band rendering.
- Use bilinear pixel-center weights on enlarged axes and existing source footprints on reduced axes, including mixed scaling.
- Interpolation improves presentation only; the saved geographical source remains 100 m.
