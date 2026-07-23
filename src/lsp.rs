use crate::{
    HighlightData, IdeHighlightingOutput, RawToken, build_ide_highlighting, get_ftml_tokens,
    sanitize_unused_input,
};
use anyhow::{Result as AnyResult, anyhow, bail};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::{
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, InitializeParams, InitializeResult,
    InitializedParams, MessageType, SemanticToken, SemanticTokenModifier, SemanticTokenType,
    SemanticTokens, SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
    SemanticTokensParams, SemanticTokensResult, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, Url,
};
use tower_lsp::{Client, LanguageServer, LspService, Server};

const TOKEN_TYPE_KEYWORD: u32 = 0;
const TOKEN_TYPE_ATTRIBUTE: u32 = 1;
const TOKEN_TYPE_PUNCTUATION: u32 = 2;
const TOKEN_TYPE_STRING: u32 = 3;
const TOKEN_TYPE_COMMENT: u32 = 4;
const TOKEN_TYPE_TEXT: u32 = 5;

pub fn semantic_tokens_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::KEYWORD,  // keyword
            SemanticTokenType::PROPERTY, // attribute
            SemanticTokenType::OPERATOR, // punctuation
            SemanticTokenType::STRING,   // string
            SemanticTokenType::COMMENT,  // comment
            SemanticTokenType::VARIABLE, // text
        ],
        token_modifiers: vec![SemanticTokenModifier::DECLARATION],
    }
}

pub fn source_to_semantic_tokens(source_text: &str) -> AnyResult<Vec<SemanticToken>> {
    let raw_tokens: Vec<RawToken> =
        serde_json::from_str(&get_ftml_tokens(source_text)?.tokens_json)?;
    let sanitized_tokens = sanitize_unused_input(raw_tokens);
    let highlighting = build_ide_highlighting(&sanitized_tokens);

    ide_ir_to_semantic_tokens(&highlighting)
}

pub fn ide_ir_to_semantic_tokens(
    highlighting: &IdeHighlightingOutput,
) -> AnyResult<Vec<SemanticToken>> {
    let absolute_tokens = parse_ir_entries(&highlighting.data)?;
    let mut semantic_tokens = Vec::with_capacity(absolute_tokens.len());
    let mut previous_line = 0u32;
    let mut previous_start = 0u32;

    for (line, start, length, kind) in absolute_tokens {
        let token_type = kind_to_token_type(&kind)?;
        let delta_line = line.saturating_sub(previous_line);
        let delta_start = if delta_line == 0 {
            start.saturating_sub(previous_start)
        } else {
            start
        };

        semantic_tokens.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset: 0,
        });

        previous_line = line;
        previous_start = start;
    }

    Ok(semantic_tokens)
}

fn parse_ir_entries(data: &[HighlightData]) -> AnyResult<Vec<(u32, u32, u32, String)>> {
    if !data.len().is_multiple_of(4) {
        bail!("Highlight IR data must contain groups of 4 values");
    }

    let mut entries = Vec::with_capacity(data.len() / 4);

    for chunk in data.chunks_exact(4) {
        let line = parse_usize_as_u32(&chunk[0], "line")?;
        let start = parse_usize_as_u32(&chunk[1], "start")?;
        let length = parse_usize_as_u32(&chunk[2], "length")?;
        let kind = match &chunk[3] {
            HighlightData::Kind(kind) => kind.clone(),
            HighlightData::Number(_) => bail!("Expected highlighting kind string"),
        };

        entries.push((line, start, length, kind));
    }

    Ok(entries)
}

fn parse_usize_as_u32(value: &HighlightData, field: &str) -> AnyResult<u32> {
    let value = match value {
        HighlightData::Number(value) => *value,
        HighlightData::Kind(_) => bail!("Expected numeric {field} in highlighting IR"),
    };

    u32::try_from(value).map_err(|_| anyhow!("Value for {field} exceeds u32"))
}

fn kind_to_token_type(kind: &str) -> AnyResult<u32> {
    match kind {
        "keyword" => Ok(TOKEN_TYPE_KEYWORD),
        "attribute" => Ok(TOKEN_TYPE_ATTRIBUTE),
        "punctuation" => Ok(TOKEN_TYPE_PUNCTUATION),
        "string" => Ok(TOKEN_TYPE_STRING),
        "comment" => Ok(TOKEN_TYPE_COMMENT),
        "text" => Ok(TOKEN_TYPE_TEXT),
        _ => bail!("Unknown highlighting kind: {kind}"),
    }
}

pub struct Backend {
    client: Client,
    semantic_tokens_by_uri: RwLock<HashMap<Url, Vec<SemanticToken>>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            semantic_tokens_by_uri: RwLock::new(HashMap::new()),
        }
    }

    async fn update_semantic_tokens(&self, uri: Url, source_text: &str) {
        match source_to_semantic_tokens(source_text) {
            Ok(tokens) => {
                self.semantic_tokens_by_uri
                    .write()
                    .await
                    .insert(uri, tokens);
            }
            Err(error) => {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!("Failed computing semantic tokens: {error}"),
                    )
                    .await;
            }
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> LspResult<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                semantic_tokens_provider: Some(
                    tower_lsp::lsp_types::SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            work_done_progress_options: Default::default(),
                            legend: semantic_tokens_legend(),
                            range: None,
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                        },
                    ),
                ),
                ..ServerCapabilities::default()
            },
            ..InitializeResult::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(
                MessageType::INFO,
                "FTML LSP semantic token server initialized.",
            )
            .await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let source_text = params.text_document.text;

        self.update_semantic_tokens(uri, &source_text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(last_change) = params.content_changes.last() {
            self.update_semantic_tokens(uri, &last_change.text).await;
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> LspResult<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let tokens = self.semantic_tokens_by_uri.read().await;
        let Some(cached_tokens) = tokens.get(&uri) else {
            return Ok(None);
        };

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: cached_tokens.clone(),
        })))
    }
}

pub async fn run_lsp_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(Backend::new);

    Server::new(stdin, stdout, socket).serve(service).await;
}
