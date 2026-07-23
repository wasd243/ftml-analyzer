use super::RawToken;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HighlightData {
    Number(usize),
    Kind(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdeHighlightingOutput {
    pub data: Vec<HighlightData>,
}

/// Build IDE highlighting data in the format:
/// [line, column, length, kind, ...]
pub fn build_ide_highlighting(tokens: &[RawToken]) -> IdeHighlightingOutput {
    let mut data = Vec::new();
    let mut line = 0usize;
    let mut column = 0usize;
    let mut inside_block = false;
    let mut inside_comment = false;
    let mut expect_block_keyword = false;

    for (index, token) in tokens.iter().enumerate() {
        let start_line = line;
        let start_column = column;
        let length = token.slice.chars().count();

        let kind = classify_token(
            tokens,
            index,
            inside_block,
            inside_comment,
            expect_block_keyword,
        );

        match token.token.as_str() {
            "left-block" | "left-block-end" => {
                inside_block = true;
                expect_block_keyword = true;
            }
            "right-block" => {
                inside_block = false;
                expect_block_keyword = false;
            }
            "left-comment" => inside_comment = true,
            "right-comment" => inside_comment = false,
            "identifier" if expect_block_keyword => expect_block_keyword = false,
            _ => {}
        }

        if let Some(kind) = kind {
            if length > 0 {
                data.push(HighlightData::Number(start_line));
                data.push(HighlightData::Number(start_column));
                data.push(HighlightData::Number(length));
                data.push(HighlightData::Kind(kind.to_string()));
            }
        }

        for ch in token.slice.chars() {
            if ch == '\n' {
                line += 1;
                column = 0;
            } else {
                column += 1;
            }
        }
    }

    IdeHighlightingOutput { data }
}

pub fn load_ide_highlighting(path: &str) -> Result<IdeHighlightingOutput> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let output: IdeHighlightingOutput = serde_json::from_reader(reader)?;

    Ok(output)
}

fn classify_token(
    tokens: &[RawToken],
    index: usize,
    inside_block: bool,
    inside_comment: bool,
    expect_block_keyword: bool,
) -> Option<&'static str> {
    let token = &tokens[index];

    if token.token == "whitespace"
        || token.token == "line-break"
        || token.token == "paragraph-break"
    {
        return None;
    }

    if inside_comment && token.token == "identifier" {
        return Some("comment");
    }

    match token.token.as_str() {
        "left-block" | "left-block-end" | "right-block" | "left-comment" | "right-comment"
        | "left-bracket" | "right-bracket" | "pipe" | "equals" | "colon" | "double-quote"
        | "bold" | "italics" | "underline" | "double-dash" | "superscript" | "subscript"
        | "raw" | "quote" | "triple-dash" => Some("punctuation"),
        "url" => Some("string"),
        "identifier" => {
            if expect_block_keyword {
                Some("keyword")
            } else if inside_block && next_non_whitespace_token(tokens, index) == Some("equals") {
                Some("attribute")
            } else {
                Some("text")
            }
        }
        _ => Some("text"),
    }
}

fn next_non_whitespace_token(tokens: &[RawToken], index: usize) -> Option<&str> {
    tokens
        .iter()
        .skip(index + 1)
        .find(|token| {
            token.token != "whitespace"
                && token.token != "line-break"
                && token.token != "paragraph-break"
        })
        .map(|token| token.token.as_str())
}
