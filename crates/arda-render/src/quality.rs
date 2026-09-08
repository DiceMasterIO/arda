//! Validated output resolution, independent of saved terrain resolution.

use crate::RenderError;

/// PNG edge length: an area's side or an overview's longest edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageQuality(u32);

impl ImageQuality {
    /// Smallest supported edge, in pixels.
    pub const MIN: u32 = 512;
    /// Largest supported edge, in pixels (32K).
    pub const MAX: u32 = 32_768;
    /// Default export quality, 8K.
    pub const DEFAULT: Self = Self(8192);

    /// Validates a requested edge length.
    ///
    /// # Errors
    /// Returns [`RenderError::InvalidImageQuality`] outside 512–32768 pixels.
    pub const fn new(pixels: u32) -> Result<Self, RenderError> {
        if pixels >= Self::MIN && pixels <= Self::MAX {
            Ok(Self(pixels))
        } else {
            Err(RenderError::InvalidImageQuality)
        }
    }

    /// Requested edge length in pixels.
    #[must_use]
    pub const fn pixels(self) -> u32 {
        self.0
    }

    /// Fits an overview to this long edge, preserving its area-grid ratio.
    ///
    /// The shorter edge is rounded to the nearest pixel.
    ///
    /// # Errors
    /// Returns [`RenderError::ExactOverviewDimensions`] for invalid area counts.
    pub fn overview_dimensions(
        self,
        areas_wide: i32,
        areas_high: i32,
    ) -> Result<(u32, u32), RenderError> {
        if !(1..=78).contains(&areas_wide) || !(1..=78).contains(&areas_high) {
            return Err(RenderError::ExactOverviewDimensions);
        }
        let width = u32::try_from(areas_wide).map_err(|_| RenderError::ExactOverviewDimensions)?;
        let height = u32::try_from(areas_high).map_err(|_| RenderError::ExactOverviewDimensions)?;
        let longest = width.max(height);
        Ok((
            (self.0 * width + longest / 2) / longest,
            (self.0 * height + longest / 2) / longest,
        ))
    }
}

impl Default for ImageQuality {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl std::str::FromStr for ImageQuality {
    type Err = RenderError;

    /// Accepts a pixel count or an integer K suffix, where 1K is 1024 pixels.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        let (number, factor) = match text.strip_suffix('k').or_else(|| text.strip_suffix('K')) {
            Some(number) => (number, 1024),
            None => (text, 1),
        };
        let pixels = number
            .parse::<u32>()
            .ok()
            .and_then(|n| n.checked_mul(factor))
            .ok_or(RenderError::InvalidImageQuality)?;
        Self::new(pixels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_parses_pixels_and_binary_k_with_an_8k_default() {
        assert_eq!(ImageQuality::default().pixels(), 8192);
        for (text, pixels) in [("512", 512), ("513", 513), ("8k", 8192), ("32K", 32768)] {
            assert_eq!(text.parse::<ImageQuality>().unwrap().pixels(), pixels);
        }
        for text in [
            "0",
            "511",
            "32769",
            "33k",
            "-512",
            "8.5k",
            "4294967295k",
            "high",
        ] {
            assert!(text.parse::<ImageQuality>().is_err(), "{text}");
        }
    }

    #[test]
    fn overview_keeps_the_long_edge_and_rounds_the_short_edge() {
        let quality = ImageQuality::new(32768).unwrap();
        assert_eq!(quality.overview_dimensions(1, 1).unwrap(), (32768, 32768));
        assert_eq!(quality.overview_dimensions(2, 4).unwrap(), (16384, 32768));
        assert_eq!(quality.overview_dimensions(19, 9).unwrap(), (32768, 15522));
        assert_eq!(
            ImageQuality::new(512)
                .unwrap()
                .overview_dimensions(1, 78)
                .unwrap(),
            (7, 512)
        );
        for (w, h) in [(0, 1), (1, -1), (79, 1)] {
            assert!(quality.overview_dimensions(w, h).is_err());
        }
    }
}
