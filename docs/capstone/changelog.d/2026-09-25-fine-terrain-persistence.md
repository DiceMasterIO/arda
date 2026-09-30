# Fine terrain storage and full world seed identity

Added bounded streaming fine-height persistence with versioned geometry, metadata/payload checksums and canonical integer queries. Both source-composition cases reopen and reproduce their four Arda renders byte-for-byte. Added the existing full 64-bit world seed/attempt RNG path for spectral generation while retaining the native 32-bit reference/calibration path. Core and focused generation checks pass.

Normal world admission, terrain authority, climate/drainage, manifest/export integration and final 32K delivery remain open. Evidence: `features/2026-09-23-terrain-corrections/evidence/fine-terrain-persistence/` and `world-spectral-seed/`.
