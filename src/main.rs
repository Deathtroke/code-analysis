use lsp_types::request::{Initialize, Shutdown, DocumentSymbolRequest};
use lsp_types::*;
use lsp_types::notification::{DidOpenTextDocument, Initialized, Exit};
use lsp_types::notification::Notification as LspNotification;
use lsp_types::request::Request as LspRequest;

mod parser;
mod lang_server;

fn main() {
    let project_path = "/Users/hannes.boerner/Downloads/criu-criu-dev".to_string();
    let input = r#"{@fanotify_resolve_remap}"#;
    let mut parser = parser::parser::new(project_path);

    let functions = parser.parse(input);
    println!("{:?}", functions);

    println!("{}", parser.graph_to_DOT())
}


#[cfg(test)]
mod grammar_test;
mod parser_test;