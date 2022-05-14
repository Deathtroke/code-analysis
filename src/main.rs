use lsp_types::request::{Initialize, Shutdown, DocumentSymbolRequest};
use lsp_types::*;
use lsp_types::notification::{DidOpenTextDocument, Initialized, Exit};
use lsp_types::notification::Notification as LspNotification;
use lsp_types::request::Request as LspRequest;

mod parser;
mod lang_server;

fn main() {/*
    let input = r#"parent of "INIT_LIST_HEAD""#;
    let mut parser = parser::parser::new();
    let functions = parser.parse(input);
    println!("{:?}", functions);*/
}


#[cfg(test)]
mod grammar_test;
mod parser_test;