use std::fs::File;
use std::io::BufReader;
use anyhow::Result;
use serde::Deserialize;

/// Deserialized token from `.json` token file for IDEs highlighting
#[derive(Debug, Deserialize)]
pub struct RawToken {
    pub token: String,
    pub slice: String,
    pub span: Span,
}

#[derive(Debug, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
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
        load_ftml_tokens("tests/tokens/include.json").unwrap();
        assert!(true);
    }
}
