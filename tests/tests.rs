use ftml_analyzer::{get_ftml_ast, get_ftml_tokens};
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
