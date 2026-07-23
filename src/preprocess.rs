mod highlight;
mod sanitize;

pub use highlight::{
    HighlightData, IdeHighlightingOutput, build_ide_highlighting, load_ide_highlighting,
};
pub use sanitize::{RawToken, Span, load_ftml_tokens, sanitize_unused_input};
