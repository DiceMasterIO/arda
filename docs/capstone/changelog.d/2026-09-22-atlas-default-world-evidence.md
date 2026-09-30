## 2026-09-22 - progress: default-world Atlas inspection
key: progress/2026-09-22-atlas-visual-fidelity@default-world-evidence

- Existing seed-42 configured 500×1000 km generation completed once: 171 areas, exit 0, 1,510.31 seconds, peak RSS 1,848,684 KiB. No restart, reroll or generator modification.
- Retained world: 523 files, 1,913,421,396 bytes. All 171 area cell hashes match the pre-render inventory after gallery export. This is not a before/after claim for every other file.
- Production renderer source `9a16883`: real 3880×8192 overview and six 8192×8192 area views. Selection uses altitude, slope, lake/river cell counts and balanced coastline; a mixed lake-margin view supplements the retained all-lake maximum.
- World maximum 3,801.251 m establishes coverage of palette ranges missing from the earlier small fixture. Independent Sol inspection and root inspection find repeated grooves, stepped shores, thin rivers and a large angular interior lake. No final visual acceptance is claimed.
- The alpine image contains no channel at 255; broad pale relief is not evidence of RGB clipping. Across internal default-world coast edges, 9,679 of 45,792 touch a zero-height sea sample; inter-area edges are excluded from this diagnostic.
- A second scratch shoreline prototype uses categorical land/sea samples and rounds corners without zero-height fallback. Its small-world 8K crop changes 920 pixels; broad steps remain. Production source is unchanged by either shore prototype.
- A globally saturated deep-water palette was tested against fresh production renders and rejected: it reduces rim contrast but leaves straight terraces and loses deep-water colour variation. The current palette remains unchanged.
- `open-items.md`: records the completed default-world gallery and updates OI-02, OI-05 and OI-12; terrain/shoreline scope remains a pending user decision. The broader visual objective remains incomplete.
