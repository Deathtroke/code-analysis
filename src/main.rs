use lsp_types::request::{Initialize, Shutdown, DocumentSymbolRequest};
use lsp_types::*;
use lsp_types::notification::{DidOpenTextDocument, Initialized, Exit};
use lsp_types::notification::Notification as LspNotification;
use lsp_types::request::Request as LspRequest;

use structopt;
use structopt::StructOpt;

mod parser;
mod lang_server;

#[derive(StructOpt, Debug)]
#[structopt()]
pub struct Opt {
    #[structopt(short = "q", long = "query", default_value = r#"{@fanotify_resolve_remap}"#)]
    query: String,
    #[structopt(short = "o", long = "output-file", default_value = "")]
    output: String,
}

fn main() {
    let opt = Opt::from_args();

    let project_path = "/Users/hannes.boerner/Downloads/criu-criu-dev/criu".to_string();

    let mut parser = parser::parser::new(project_path);

    parser.parse(opt.query.as_str());

    println!("{}", parser.graph_to_DOT());
    if opt.output != ""
    {
        parser.graph_to_file(opt.output);
    }
}


#[cfg(test)]
mod grammar_test;
mod parser_test;