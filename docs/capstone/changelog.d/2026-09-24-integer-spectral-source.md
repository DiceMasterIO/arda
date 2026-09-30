# Deterministic preferred terrain source

- Ported the selected procedural source into repository-owned MT19937 phase generation, Q60 cosine tables and integer Fourier filtering; added reproducible Decimal table generation and an executable diagnostic consumer.
- Matched native square sources across three seeds and through 1024² within 1 mm, with byte-identical repeated Rust outputs and actual Arda render comparisons.
- Checked a declared rectangular physical-frequency extension, explicit resource admission and invalid input handling; passed nine targeted tests, generation all-target Clippy and workspace formatting/compilation.
- Normal world integration, physical-domain/coast composition, 64-bit world-seed mapping, fine-field persistence, drainage and final 32K delivery remain open. The saved original world is preserved.
