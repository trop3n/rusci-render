pub mod svg;
pub mod obj;
pub mod text;
pub mod image;
pub mod gif;
pub mod gpla;
pub mod audio;
pub mod lua;
pub mod file_parser;

pub use file_parser::{FileType, ParseResult, parse_file, parse_file_typed, default_shapes};
