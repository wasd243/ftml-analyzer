mod get_ftml_ast;
mod lsp;
mod preprocess;

pub use crate::get_ftml_ast::{FtmlParseOutput, FtmlTokenizeOutput, get_ftml_ast, get_ftml_tokens};
pub use crate::lsp::{
    ide_ir_to_semantic_tokens, run_lsp_server, semantic_tokens_legend, source_to_semantic_tokens,
};
pub use crate::preprocess::{
    HighlightData, IdeHighlightingOutput, RawToken, Span, build_ide_highlighting, load_ftml_tokens,
    load_ide_highlighting, sanitize_unused_input,
};
