use ftml_analyzer::{
    RawToken, build_ide_highlighting, get_ftml_ast, get_ftml_tokens, ide_ir_to_semantic_tokens,
    load_ftml_tokens, load_ide_highlighting, sanitize_unused_input,
};
use std::fs;
use std::path::Path;

/// Function to load `.ftml` source text
fn load_ftml_source_text<P: AsRef<Path>>(dir: P) -> Vec<(String, String)> {
    let mut source_text = Vec::new();

    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("ftml") {
            continue;
        }

        let name = path.file_stem().unwrap().to_string_lossy().to_string();

        let content = fs::read_to_string(&path).unwrap();

        source_text.push((name, content));
    }

    source_text
}

#[test]
fn test_ftml_ast_output() {
    let source_text = load_ftml_source_text("tests/ftml");

    for (name, source) in source_text {
        println!("Testing {name}");

        let result = get_ftml_ast(&source, "en", &name);

        fs::write(format!("tests/ast/{name}.json"), result.unwrap().ast_json)
            .expect("Failed writing AST JSON");
    }
}

#[test]
fn test_ftml_tokenize_output() {
    let source_text = load_ftml_source_text("tests/ftml");

    for (name, source) in source_text {
        println!("Testing {name}");

        let result = get_ftml_tokens(&source);

        fs::write(
            format!("tests/tokens/{name}.json"),
            result.unwrap().tokens_json,
        )
        .expect("Failed writing tokens JSON");
    }
}

#[test]
fn test_ftml_tokenize_output_with_sanitized_tokens() {
    let source_text = load_ftml_source_text("tests/ftml");
    fs::create_dir_all("tests/tokens_after_preprocess").expect("Failed creating output directory");

    for (name, source) in source_text {
        println!("Testing {name}");

        let raw_tokens: Vec<RawToken> =
            serde_json::from_str(&get_ftml_tokens(&source).unwrap().tokens_json)
                .expect("Failed to deserialize token output");

        let sanitized_tokens = sanitize_unused_input(raw_tokens);

        assert!(
            sanitized_tokens
                .iter()
                .all(|token| { token.token != "input-start" && token.token != "input-end" }),
            "Sanitized tokens for {name} still contain input-start/input-end markers"
        );

        fs::write(
            format!("tests/tokens_after_preprocess/{name}.json"),
            serde_json::to_string_pretty(&sanitized_tokens)
                .expect("Failed serializing sanitized tokens"),
        )
        .expect("Failed writing sanitized tokens JSON");
    }
}

#[test]
fn test_ide_highlighting_output_from_preprocessed_tokens() {
    fs::create_dir_all("tests/highlighting").expect("Failed creating output directory");

    for entry in fs::read_dir("tests/tokens_after_preprocess").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let path_str = path.to_string_lossy();
        let tokens = load_ftml_tokens(&path_str).expect("Failed loading preprocessed tokens");
        let highlighting = build_ide_highlighting(&tokens);

        if name == "div" {
            let output =
                serde_json::to_value(&highlighting).expect("Failed converting highlighting output");
            let data = output["data"]
                .as_array()
                .expect("Highlighting output has invalid `data` format");
            let expected_prefix = serde_json::json!([
                0,
                0,
                2,
                "punctuation",
                0,
                2,
                3,
                "keyword",
                0,
                6,
                5,
                "attribute"
            ]);

            assert_eq!(
                data.iter().take(12).cloned().collect::<Vec<_>>(),
                expected_prefix.as_array().unwrap().to_vec(),
                "Unexpected highlighting prefix in div fixture"
            );
        }

        fs::write(
            format!("tests/highlighting/{name}.json"),
            serde_json::to_string_pretty(&highlighting)
                .expect("Failed serializing highlighting output"),
        )
        .expect("Failed writing highlighting output");
    }
}

#[test]
fn test_lsp_semantic_tokens_from_ir() {
    fs::create_dir_all("tests/lsp").expect("Failed creating output directory");

    for entry in fs::read_dir("tests/highlighting").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let path_str = path.to_string_lossy();
        let highlighting =
            load_ide_highlighting(&path_str).expect("Failed loading highlighting IR");
        let semantic_tokens = ide_ir_to_semantic_tokens(&highlighting)
            .expect("Failed converting IR to LSP semantic tokens");

        if name == "div" {
            let expected_prefix = vec![(0, 0, 2, 2), (0, 2, 3, 0), (0, 4, 5, 1)];

            let actual_prefix: Vec<(u32, u32, u32, u32)> = semantic_tokens
                .iter()
                .take(3)
                .map(|token| {
                    (
                        token.delta_line,
                        token.delta_start,
                        token.length,
                        token.token_type,
                    )
                })
                .collect();

            assert_eq!(
                actual_prefix, expected_prefix,
                "Unexpected semantic token delta prefix in div fixture"
            );
        }

        fs::write(
            format!("tests/lsp/{name}.json"),
            serde_json::to_string_pretty(
                &semantic_tokens
                    .iter()
                    .map(|token| {
                        (
                            token.delta_line,
                            token.delta_start,
                            token.length,
                            token.token_type,
                            token.token_modifiers_bitset,
                        )
                    })
                    .collect::<Vec<_>>(),
            )
            .expect("Failed serializing semantic token output"),
        )
        .expect("Failed writing semantic token output");
    }
}
