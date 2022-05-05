use lsp_types::request::{Initialize, Shutdown, DocumentSymbolRequest};
use lsp_types::*;
use lsp_types::notification::{DidOpenTextDocument, Initialized, Exit};
use lsp_types::notification::Notification as LspNotification;
use lsp_types::request::Request as LspRequest;

mod parser;
mod searcher;

fn main() {
    let mut lang_server = searcher::LanguageServerLauncher::new()
        .server("/usr/bin/clangd".to_owned())
        .project("/Users/hannes.boerner/Downloads/criu-criu-dev".to_owned())
        //.languages(language_list)
        .launch()
        .expect("Failed to spawn clangd");

    println!("test");

    lang_server.initialize();

    let document = lang_server.document_open("/criu/fsnotify.c").unwrap();
    println!("{:?}", document);
    let doc_symbol = lang_server.document_symbol(&document).unwrap();

    match doc_symbol.clone() {
        Some(DocumentSymbolResponse::Flat(_)) => {
            println!("unsupported symbols found");
        },
        Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
            for symbol in doc_symbols {
                println!("{:?}", symbol);
            }
        },
        None => {
            println!("no symbols found");
        }
    }

    println!("{:?}", doc_symbol);

    println!("{:?}", doc_symbol);

    lang_server.shutdown();
    lang_server.exit();

}


#[cfg(test)]
mod grammar_test;
mod parser_test;