# Accepted fine terrain: production access and full 32K delivery

- **What**: preserve the user-approved recipe-2 terrain appearance and deliver a complete seed-42 500×1000 km world with normal Atlas exports.
- **Production**: `generate --terrain fine` and `generate_from_fine_source` orchestrate admitted source/world phases, five deterministic candidate gates and private staging. Legacy remains the default; the production MICRO command reproduces all 35 accepted files exactly.
- **Full output**: 15,522×32,768 PNG, 259,955,341 bytes, SHA-256 `b350076951b0148fa10a09be902ac909f94d001bf1f0c553317b782dc906a583`; actual overview and mountain close-up inspected.
- **Timing**: source 406.630 s, world 1,223.970 s, verification 2.574 s, render 193.293 s; sum 1,826.467 s (30m 26s). Single local release run; full peak RSS was not measured.
- **Verification**: all 44,826,624 exported heights and 500,000 continent heights equal the canonical field; receiving topology and saved tables validate; annual sources/sinks both 159,795,615,670,000 L. Additional small seeds 43/44 pass. Existing exact repeats, seam tests and scoped suites remain preserved.
- **Preservation**: all 523 original-world files, exact file set and 1,943,353,639 bytes unchanged.
- **Review**: final independent GPT-6 Sol delivery review finds no blocker; documented minor edge is an error returned for private-stage cleanup failure after successful world publication. No such error occurred in the recorded runs.
- **Limits**: broad uniform plains, straight river runs, dense small highland lakes, 100 m categorical water boundaries and rectangular rim segments remain. User acceptance ends further visual experiments; exact reference-image parity is not claimed. Complete-area export retains its existing partial-fringe omission.
- **Reference refresh**: current status, architecture, testing, operations, index and final feature checklist record the delivered state. Reproducible commands, checksums, runtimes, inspection images and audit are retained in `features/2026-09-23-terrain-corrections/evidence/accepted-look-qualification/`.
