//! This module serves for getting parser AST by using `ftml` library

use anyhow::Result;
use ftml::prelude::ParseError;
use ftml::render::html::HtmlRender;
use serde::Serialize;
use std::borrow::Cow;

#[derive(Serialize)]
pub struct FtmlParseOutput {
    pub html: String,
    pub ast_json: String,
    pub warnings: Vec<ParseError>,
}

/// A function to get parser AST by using `ftml` library
pub fn get_ftml_ast(
    source_text: &str,
    language: &str,
    page_title: &str,
) -> Result<FtmlParseOutput> {
    let settings = ftml::settings::WikitextSettings::from_mode(
        ftml::settings::WikitextMode::Page,
        ftml::layout::Layout::Wikidot,
    );

    let site_name = get_site_name(language);
    let site = site_name.as_str();

    let page_info = ftml::data::PageInfo {
        page: Cow::Borrowed("page"),
        category: None,
        site: Cow::Borrowed(site),
        title: Cow::Borrowed(page_title),
        alt_title: None,
        score: ftml::data::ScoreValue::Integer(0),
        tags: vec![],
        language: Cow::Borrowed(language),
    };

    let mut wikitext = source_text.to_string();

    ftml::preprocess(&mut wikitext);

    let tokens = ftml::tokenize(&wikitext);

    let result = ftml::parse(&tokens, &page_info, &settings);

    let (tree, warnings) = result.into();

    let ast_json = serde_json::to_string_pretty(&tree)?;

    use ftml::render::Render;

    let html_output = HtmlRender.render(&tree, &page_info, &settings);

    Ok(FtmlParseOutput {
        html: html_output.body.to_string(),
        ast_json,
        warnings,
    })
}

/// A function to get site name correctly (I don't think this is necessary when implementing highlighter)
fn get_site_name(language: &str) -> String {
    if language == "en" {
        let site = &language;
        site.to_string()
    } else {
        let site = format!("scp-{language}");
        site
    }
}
