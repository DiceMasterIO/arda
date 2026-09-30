## 2026-09-22 - progress: Atlas terrain and shoreline diagnosis
key: progress/2026-09-22-atlas-visual-fidelity@terrain-stage-diagnosis

- `open-items.md`: updates OI-03 and OI-04 from direct saved-data comparisons; both remain open.
- Seed 42, 200×300 km: regenerated continent attempt 0 matches all 60,000 saved regional heights. Aligned valleys in area (2,0) are present before area evolution; final evolution deepens them. A second scratch probe captures the pre-erosion field and localizes appearance to the 25 continent-erosion iterations; 11 consecutive northward receivers explain one reproduced straight incision segment.
- The ocean contour diagnostic changes 4,894 of 1,048,576 pixels in a 6.4 km crop. It demonstrates smoother inferred geometry and the change to categorical land/water ownership; it is not a production implementation or topology guarantee.
- A diagnostic inventory measures every area in the retained world and selects representative altitude, slope, lake, river and mixed-coast views by recorded rules. Land count and maximum height are cross-checked against the independent JSON export for area (2,0).
- Production source and saved generation remain unchanged in this diagnostic increment. The broader visual objective remains active; no completion marker is written.
