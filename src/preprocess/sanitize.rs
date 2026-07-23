use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

/// Deserialized token from `.json` token file for IDEs highlighting
#[derive(Debug, Deserialize, Serialize)]
pub struct RawToken {
    pub token: String,
    pub slice: String,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// Remove unused tokens from the input.
pub fn sanitize_unused_input(tokens: Vec<RawToken>) -> Vec<RawToken> {
    tokens
        .into_iter()
        .filter(|token| token.token != "input-start" && token.token != "input-end")
        .collect()
}

/// Load and deserialize `.json` token file for IDEs highlighting
pub fn load_ftml_tokens(path: &str) -> Result<Vec<RawToken>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // deserialize tokens
    let tokens: Vec<RawToken> = serde_json::from_reader(reader)?;

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_ftml_function_works() {
        let tokens = load_ftml_tokens("tests/tokens/include.json").unwrap();

        assert!(!tokens.is_empty());
    }

    #[test]
    fn output_load_ftml_function() {
        let token = load_ftml_tokens("tests/tokens/include.json").unwrap();
        println!("{:#?}", token);
    }
}
