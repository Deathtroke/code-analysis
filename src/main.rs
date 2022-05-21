use lsp_types::request::{Initialize, Shutdown, DocumentSymbolRequest};
use lsp_types::*;
use lsp_types::notification::{DidOpenTextDocument, Initialized, Exit};
use lsp_types::notification::Notification as LspNotification;
use lsp_types::request::Request as LspRequest;
use serde::Serialize;

use structopt;
use structopt::StructOpt;

mod parser;
mod lang_server;

use regex::Regex;

#[derive(StructOpt, Debug)]
#[structopt()]
pub struct Opt {
    #[structopt(short = "q", long = "query", default_value = r#"{@fanotify_resolve_remap}"#)]
    query: String,
    #[structopt(short = "o", long = "output-file")]
    output: Option<String>,
    #[structopt(short = "p", long = "project-path", default_value = "/Users/hannes.boerner/Documents/criu/criu")]
    project_path: String,
}

fn main() {
    let opt = Opt::from_args();

    let mut parser = parser::parser::new(opt.project_path);


    let re = Regex::new(r".").unwrap();
    println!("regex test: {}", re.is_match("abcabc"));

    parser.parse(opt.query.as_str());

    if opt.output.is_some() {
        parser.graph.graph_to_file(opt.output.unwrap());
    } else {
        println!("{}", parser.graph.graph_to_DOT());
    }
    println!("{:?}", serde_json::to_string(&parser.graph));
}


#[cfg(test)]
mod grammar_test;
mod parser_test;