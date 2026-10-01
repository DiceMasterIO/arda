//! Reading raw images (PNG with generator metadata, JPEG references).

use crate::error::{ImportError, ImportResult};
use arda_tactical::Rgba;
use std::path::Path;

/// A decoded raw image.
#[derive(Debug, Clone)]
pub struct RawImage {
    /// The pixels, straight alpha.
    pub rgba: Rgba,
    /// Whether the file carried an alpha channel with any transparency.
    pub has_alpha: bool,
    /// Generator metadata found in PNG text chunks.
    pub meta: GenMeta,
}

/// Generator metadata recovered from ComfyUI or A1111 PNG text chunks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GenMeta {
    /// The positive prompt.
    pub prompt: Option<String>,
    /// The sampler seed.
    pub seed: Option<u64>,
    /// The checkpoint or UNet name.
    pub model: Option<String>,
}

/// Reads a PNG or JPEG by extension.
///
/// # Errors
/// I/O and decode failures.
pub fn read(path: &Path) -> ImportResult<RawImage> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    match ext.as_deref() {
        Some("jpg" | "jpeg") => read_jpeg(path),
        _ => read_png(path),
    }
}

fn image_err(path: &Path, message: &impl ToString) -> ImportError {
    ImportError::Image {
        path: path.to_path_buf(),
        message: message.to_string(),
    }
}

fn read_png(path: &Path) -> ImportResult<RawImage> {
    let rgba = Rgba::read_png(path)?;
    let has_alpha = rgba.data.as_chunks::<4>().0.iter().any(|p| p[3] < 255);
    let meta = png_text(path).map(|t| parse_meta(&t)).unwrap_or_default();
    Ok(RawImage {
        rgba,
        has_alpha,
        meta,
    })
}

fn read_jpeg(path: &Path) -> ImportResult<RawImage> {
    let file = std::fs::File::open(path).map_err(crate::error::io(path))?;
    let mut dec = jpeg_decoder::Decoder::new(std::io::BufReader::new(file));
    let pixels = dec.decode().map_err(|e| image_err(path, &e))?;
    let info = dec
        .info()
        .ok_or_else(|| image_err(path, &"no JPEG header"))?;
    let data: Vec<u8> = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => pixels
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        jpeg_decoder::PixelFormat::L8 => pixels.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        other => return Err(image_err(path, &format!("unsupported JPEG {other:?}"))),
    };
    let rgba = Rgba {
        width: u32::from(info.width),
        height: u32::from(info.height),
        data,
    };
    Ok(RawImage {
        rgba,
        has_alpha: false,
        meta: GenMeta::default(),
    })
}

/// Every text chunk as `(keyword, text)`.
fn png_text(path: &Path) -> Option<Vec<(String, String)>> {
    let file = std::fs::File::open(path).ok()?;
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let reader = decoder.read_info().ok()?;
    let info = reader.info();
    let mut out: Vec<(String, String)> = info
        .uncompressed_latin1_text
        .iter()
        .map(|c| (c.keyword.clone(), c.text.clone()))
        .collect();
    out.extend(
        info.compressed_latin1_text
            .iter()
            .filter_map(|c| Some((c.keyword.clone(), c.get_text().ok()?))),
    );
    out.extend(
        info.utf8_text
            .iter()
            .filter_map(|c| Some((c.keyword.clone(), c.get_text().ok()?))),
    );
    Some(out)
}

/// Pulls prompt, seed and model out of ComfyUI's `prompt` graph or
/// A1111's `parameters` text.
#[must_use]
pub fn parse_meta(chunks: &[(String, String)]) -> GenMeta {
    let mut meta = GenMeta::default();
    if let Some((_, json)) = chunks.iter().find(|(k, _)| k == "prompt") {
        if let Ok(graph) = serde_json::from_str::<serde_json::Value>(json) {
            comfy_graph(&graph, &mut meta);
        }
    }
    if let Some((_, text)) = chunks.iter().find(|(k, _)| k == "parameters") {
        let mut lines = text.lines();
        meta.prompt = meta.prompt.or_else(|| lines.next().map(str::to_string));
        for part in text.split(',').map(str::trim) {
            if let Some(v) = part.strip_prefix("Seed: ") {
                meta.seed = meta.seed.or_else(|| v.parse().ok());
            }
            if let Some(v) = part.strip_prefix("Model: ") {
                meta.model = meta.model.clone().or_else(|| Some(v.to_string()));
            }
        }
    }
    meta
}

/// Walks a ComfyUI API graph: `{node_id: {class_type, inputs}}`. Node ids
/// are visited in sorted order so the result is deterministic.
fn comfy_graph(graph: &serde_json::Value, meta: &mut GenMeta) {
    let Some(nodes) = graph.as_object() else {
        return;
    };
    let mut entries: Vec<(&String, &serde_json::Value)> = nodes.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let mut longest = String::new();
    for (_, node) in entries {
        let class = node["class_type"].as_str().unwrap_or("");
        let inputs = &node["inputs"];
        for key in ["seed", "noise_seed"] {
            if meta.seed.is_none() {
                meta.seed = inputs[key].as_u64();
            }
        }
        for key in ["ckpt_name", "unet_name"] {
            if meta.model.is_none() {
                meta.model = inputs[key].as_str().map(str::to_string);
            }
        }
        if class.starts_with("CLIPTextEncode") {
            for key in ["text", "clip_l", "t5xxl"] {
                if let Some(t) = inputs[key].as_str() {
                    if t.len() > longest.len() {
                        longest = t.to_string();
                    }
                }
            }
        }
    }
    if !longest.is_empty() {
        meta.prompt = Some(longest);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comfy_prompt_graph_yields_prompt_seed_and_model() {
        let graph = r#"{
            "3": {"class_type": "KSampler", "inputs": {"seed": 812734, "steps": 4}},
            "4": {"class_type": "CheckpointLoaderSimple", "inputs": {"ckpt_name": "flux1-schnell.safetensors"}},
            "6": {"class_type": "CLIPTextEncode", "inputs": {"text": "top-down iron anvil, flat lighting"}},
            "7": {"class_type": "CLIPTextEncode", "inputs": {"text": "shadow"}}
        }"#;
        let meta = parse_meta(&[("prompt".into(), graph.into())]);
        assert_eq!(meta.seed, Some(812_734));
        assert_eq!(meta.model.as_deref(), Some("flux1-schnell.safetensors"));
        assert_eq!(
            meta.prompt.as_deref(),
            Some("top-down iron anvil, flat lighting")
        );
    }

    #[test]
    fn a1111_parameters_are_read() {
        let text =
            "a barrel, top-down\nNegative prompt: shadow\nSteps: 20, Seed: 42, Model: sdxl_base";
        let meta = parse_meta(&[("parameters".into(), text.into())]);
        assert_eq!(meta.seed, Some(42));
        assert_eq!(meta.model.as_deref(), Some("sdxl_base"));
        assert_eq!(meta.prompt.as_deref(), Some("a barrel, top-down"));
    }
}
