use lsp_types::notification::Notification as LspNotification;
use lsp_types::notification::{DidOpenTextDocument, Exit, Initialized};
use lsp_types::request::Request as LspRequest;
use lsp_types::request::{DocumentSymbolRequest, Initialize, Shutdown};
use lsp_types::*;

use structopt;
use structopt::StructOpt;

mod graph;
mod lang_server;
mod analyzer;
mod searcher;
mod ast_generator;

#[derive(StructOpt, Debug)]
#[structopt()]
pub struct Opt {
    #[structopt(short = "q", long = "query")]
    query: String,
    #[structopt(short = "o", long = "output-file")]
    output: Option<String>,
    #[structopt(short = "p", long = "project-path")]
    project_path: String,
    #[structopt(short = "l", long = "lsp-path", default_value = "/usr/bin/clangd")]
    lsp_path: String,
}

fn main() {
    if let Err(err) = try_main() {
        eprintln!("ERROR: {}", err);
        err.chain()
            .skip(1)
            .for_each(|cause| eprintln!("because: {}", cause));
        std::process::exit(1);
    }
}

fn try_main() -> anyhow::Result<()> {
    let opt = Opt::from_args();

    //let lsp_server: searcher::LSPServer = searcher::LSPServer::new(opt.project_path);
    let lsp_server = searcher::ClangdServer::new(opt.project_path.clone(), opt.lsp_path.clone());
    let mut parser = analyzer::Analyzer::new(lsp_server);

    parser.parse(opt.query.as_str());

    let mut out: Box<dyn std::io::Write> = if let Some(filename) = opt.output {
        Box::new(std::fs::File::create(filename)?)
    } else {
        Box::new(std::io::stdout())
    };

    //let g: tabbycat::Graph = analyzer.graph.try_into()?;
    let g = parser.graph.graph_to_dot();
    out.write(g.to_string().as_bytes())?;

    parser.close_lsp();

    Ok(())
}

#[cfg(test)]
mod grammar_test;
