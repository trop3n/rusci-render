//! File parser dispatch — routes files to the appropriate parser by extension.

use osci_core::shape::Shape;
use osci_core::Point;

use crate::image::ImageConfig;

/// Supported file types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Svg,
    Obj,
    Text,
    Lua,
    Gpla,
    Gif,
    Image,
    Audio,
    Unknown,
}

impl FileType {
    /// Detect file type from extension string (case-insensitive).
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "svg" => FileType::Svg,
            "obj" => FileType::Obj,
            "txt" | "text" => FileType::Text,
            "lua" => FileType::Lua,
            "gpla" => FileType::Gpla,
            "gif" => FileType::Gif,
            "png" | "jpg" | "jpeg" | "bmp" | "tiff" | "tga" | "webp" => FileType::Image,
            "wav" | "aiff" | "ogg" | "flac" | "mp3" => FileType::Audio,
            _ => FileType::Unknown,
        }
    }

    /// Whether this file type produces per-sample output (vs. shape frames).
    pub fn is_sample_source(&self) -> bool {
        matches!(self, FileType::Lua | FileType::Audio)
    }

    /// Whether this file type supports animation (multiple frames).
    pub fn is_animated(&self) -> bool {
        matches!(self, FileType::Gpla | FileType::Gif)
    }
}

/// Result of parsing a file — either a single frame of shapes,
/// multiple animation frames, or audio sample data.
pub enum ParseResult {
    /// A single frame of shapes (SVG, OBJ, text, image).
    Shapes(Vec<Box<dyn Shape>>),
    /// Multiple animation frames (GPLA, GIF).
    AnimatedShapes {
        frames: Vec<Vec<Box<dyn Shape>>>,
        frame_rate: f64,
    },
    /// Audio sample data.
    Audio(crate::audio::AudioData),
    /// Lua script source (needs to be compiled separately).
    LuaScript(String),
}

/// Parse a file given its raw data and file extension.
///
/// The extension should not include the leading dot.
pub fn parse_file(data: &[u8], extension: &str) -> Result<ParseResult, String> {
    let file_type = FileType::from_extension(extension);
    parse_file_typed(data, file_type)
}

/// Parse a file given its raw data and known file type.
pub fn parse_file_typed(data: &[u8], file_type: FileType) -> Result<ParseResult, String> {
    match file_type {
        FileType::Svg => {
            let shapes = crate::svg::parse_svg(data)?;
            Ok(ParseResult::Shapes(shapes))
        }
        FileType::Obj => {
            let shapes = crate::obj::parse_obj(data)?;
            Ok(ParseResult::Shapes(shapes))
        }
        FileType::Text => {
            let text = std::str::from_utf8(data)
                .map_err(|e| format!("invalid UTF-8: {e}"))?;
            let config = crate::text::TextConfig::default();
            let shapes = crate::text::parse_text(text, &config)?;
            Ok(ParseResult::Shapes(shapes))
        }
        FileType::Lua => {
            let script = std::str::from_utf8(data)
                .map_err(|e| format!("invalid UTF-8: {e}"))?;
            Ok(ParseResult::LuaScript(script.to_string()))
        }
        FileType::Gpla => {
            let gpla = crate::gpla::parse_gpla(data)?;
            Ok(ParseResult::AnimatedShapes {
                frames: gpla.frames,
                frame_rate: gpla.frame_rate as f64,
            })
        }
        FileType::Gif => {
            let config = ImageConfig::default();
            let gif = crate::gif::parse_gif(data, &config)?;
            Ok(ParseResult::AnimatedShapes {
                frames: gif.frames,
                frame_rate: gif.frame_rate,
            })
        }
        FileType::Image => {
            let config = ImageConfig::default();
            let shapes = crate::image::parse_image(data, &config)?;
            Ok(ParseResult::Shapes(shapes))
        }
        FileType::Audio => {
            let audio = crate::audio::parse_audio(data)?;
            Ok(ParseResult::Audio(audio))
        }
        FileType::Unknown => {
            Err("unknown file type".to_string())
        }
    }
}

/// Generate a default shape (a square outline) for when no file is loaded.
pub fn default_shapes() -> Vec<Box<dyn Shape>> {
    use osci_core::shape::Line;
    vec![
        Box::new(Line::new_2d(-0.5, -0.5, 0.5, -0.5)),
        Box::new(Line::new_2d(0.5, -0.5, 0.5, 0.5)),
        Box::new(Line::new_2d(0.5, 0.5, -0.5, 0.5)),
        Box::new(Line::new_2d(-0.5, 0.5, -0.5, -0.5)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_type_detection() {
        assert_eq!(FileType::from_extension("svg"), FileType::Svg);
        assert_eq!(FileType::from_extension("SVG"), FileType::Svg);
        assert_eq!(FileType::from_extension("obj"), FileType::Obj);
        assert_eq!(FileType::from_extension("txt"), FileType::Text);
        assert_eq!(FileType::from_extension("lua"), FileType::Lua);
        assert_eq!(FileType::from_extension("gpla"), FileType::Gpla);
        assert_eq!(FileType::from_extension("gif"), FileType::Gif);
        assert_eq!(FileType::from_extension("png"), FileType::Image);
        assert_eq!(FileType::from_extension("jpg"), FileType::Image);
        assert_eq!(FileType::from_extension("wav"), FileType::Audio);
        assert_eq!(FileType::from_extension("mp3"), FileType::Audio);
        assert_eq!(FileType::from_extension("xyz"), FileType::Unknown);
    }

    #[test]
    fn test_sample_source() {
        assert!(FileType::Lua.is_sample_source());
        assert!(FileType::Audio.is_sample_source());
        assert!(!FileType::Svg.is_sample_source());
        assert!(!FileType::Obj.is_sample_source());
    }

    #[test]
    fn test_animated() {
        assert!(FileType::Gpla.is_animated());
        assert!(FileType::Gif.is_animated());
        assert!(!FileType::Svg.is_animated());
    }

    #[test]
    fn test_default_shapes() {
        let shapes = default_shapes();
        assert_eq!(shapes.len(), 4);
    }

    #[test]
    fn test_unknown_file_type_error() {
        let result = parse_file(b"data", "xyz");
        assert!(result.is_err());
    }
}
