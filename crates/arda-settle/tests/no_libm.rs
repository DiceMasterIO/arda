//! Everything `arda settle` writes is in the byte-identical set (the
//! generation stamp digests every byte, `stats.json` included), so the
//! code behind it may use only the IEEE-754 operations every platform
//! rounds alike: `+ − × ÷` and `sqrt`. libm's `hypot`, `ln`, `exp`, the
//! trigonometric functions and `powi`/`powf` (lowered to libm or to a
//! multiplication chain at the compiler's choice) may round differently
//! between platforms (OI-52; review 3.3-15). The overlay renderer and the
//! synthetic test landscape write no `society/` file and are exempt.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::path::Path;

const LIBM: &[&str] = &[
    ".hypot(",
    ".powi(",
    ".powf(",
    ".exp(",
    ".exp2(",
    ".exp_m1(",
    ".ln(",
    ".ln_1p(",
    ".log(",
    ".log2(",
    ".log10(",
    ".sin(",
    ".cos(",
    ".tan(",
    ".asin(",
    ".acos(",
    ".atan(",
    ".atan2(",
    ".sinh(",
    ".cosh(",
    ".tanh(",
    ".cbrt(",
    ".sin_cos(",
];

fn scan(dir: &Path, hits: &mut Vec<String>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        if p.is_dir() {
            if name != "render" {
                scan(&p, hits);
            }
            continue;
        }
        if !name.ends_with(".rs") || name == "synthetic.rs" {
            continue;
        }
        let text = std::fs::read_to_string(&p).unwrap();
        // Unit tests may compare against libm.
        let code = text.split("#[cfg(test)]").next().unwrap_or("");
        for (k, line) in code.lines().enumerate() {
            let line = line.split("//").next().unwrap_or("");
            for f in LIBM {
                if line.contains(f) {
                    hits.push(format!("{}:{}: {}", p.display(), k + 1, line.trim()));
                }
            }
        }
    }
}

#[test]
fn settle_outputs_use_no_libm() {
    let mut hits = Vec::new();
    scan(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut hits,
    );
    assert!(
        hits.is_empty(),
        "libm in settle outputs:\n{}",
        hits.join("\n")
    );
}
