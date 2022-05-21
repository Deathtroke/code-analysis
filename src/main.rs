use lsp_types::request::{Initialize, Shutdown, DocumentSymbolRequest};
use lsp_types::*;
use lsp_types::notification::{DidOpenTextDocument, Initialized, Exit};
use lsp_types::notification::Notification as LspNotification;
use lsp_types::request::Request as LspRequest;
use serde::Serialize;
use tabbycat;

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
    if let Err(err) = try_main() {
        eprintln!("ERROR: {}", err);
        err.chain().skip(1).for_each(|cause| eprintln!("because: {}", cause));
        std::process::exit(1);
    }
}

fn try_main() -> anyhow::Result<()> {
    let opt = Opt::from_args();

    let mut parser = parser::parser::new(opt.project_path);


    let re = Regex::new(r".").unwrap();
    println!("regex test: {}", re.is_match("abcabc"));

    parser.parse(opt.query.as_str());

    let mut out : Box<dyn std::io::Write> = if let Some(filename) = opt.output {
        Box::new(std::fs::File::create(filename)?)
    } else {
        Box::new(std::io::stdout())
    };

    let g : tabbycat::Graph = parser.graph.try_into()?;
    out.write(g.to_string().as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod grammar_test;