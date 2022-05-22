use std::collections::HashSet;
use lsp_types::DocumentSymbolResponse;
use crate::lang_server;
use crate::lang_server::LanguageServer;


pub trait LSPInterface {
    fn search_parent(&mut self, search_target: String)  -> HashSet<String>;
    fn search_child(&mut self, search_target: String)  -> HashSet<String>;
    fn paren_child_exists(&mut self, parent: String, child: String) -> bool;

}

pub(crate) struct LSPServer {
    lang_server : Box<dyn LanguageServer>,
}


pub struct FunctionEdge {
    function_name: String,
}

pub trait MatchFunctionEdge {
    fn get_func_name(&mut self) -> String;
}

pub trait ForcedEdge : MatchFunctionEdge{
    fn do_match(&mut self, match_target: FunctionEdge) -> bool;
}

pub trait DefaultEdge: MatchFunctionEdge{
    fn do_match(&mut self, match_target: FunctionEdge) -> bool;
}

impl MatchFunctionEdge for FunctionEdge {
    fn get_func_name(&mut self) -> String {
        self.function_name.clone()
    }
}

impl ForcedEdge for FunctionEdge {
    fn do_match(&mut self, match_target: FunctionEdge) -> bool {
        true
    }
}

impl DefaultEdge for FunctionEdge {
    fn do_match(&mut self, match_target: FunctionEdge) -> bool {
        todo!()
    }


}

impl LSPServer {
    pub fn new(project_path: String) -> LSPServer {
        let mut lsp_server = LSPServer{
            lang_server: lang_server::LanguageServerLauncher::new()
                .server("/usr/bin/clangd".to_owned())
                .project(project_path.to_owned())
                //.languages(language_list)
                .launch()
                .expect("Failed to spawn clangd"),
        };
        lsp_server.lang_server.initialize();
        lsp_server
    }

    pub fn search_parent_single_document(&mut self, function_name: String, document_name: &str) -> Result<HashSet<String>,  lang_server::Error> {
        let mut result: Result<HashSet<String>, lang_server::Error> = Ok(HashSet::new());
        let document_res = self.lang_server.document_open(document_name);
        if document_res.is_ok(){
            let document = document_res.unwrap();

            let doc_symbol_res = self.lang_server.document_symbol(&document);
            println!("sym {:?}", doc_symbol_res);
            if doc_symbol_res.is_ok(){
                let doc_symbol = doc_symbol_res.unwrap();

                match doc_symbol {
                    Some(DocumentSymbolResponse::Flat(_)) => {
                        println!("unsupported symbols found");
                    },
                    Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                        let mut children = HashSet::new();
                        for symbol in doc_symbols {
                            if symbol.name == function_name {
                                let prep_call_hierarchy_res = self.lang_server.call_hierarchy_item(&document, symbol.range.start);
                                if prep_call_hierarchy_res.is_ok(){
                                    println!("x {:?}", prep_call_hierarchy_res);
                                    let call_hierarchy_items = prep_call_hierarchy_res.unwrap().unwrap();
                                    if call_hierarchy_items.len() > 0 {
                                        let call_hierarchy_item = call_hierarchy_items[0].clone();

                                        let incoming_calls = self.lang_server.call_hierarchy_item_incoming(call_hierarchy_item);
                                        for incoming_call in incoming_calls.unwrap().unwrap() {
                                            children.insert(incoming_call.from.name.to_string());
                                        }
                                    }
                                    break;
                                } else {
                                    result = Err(prep_call_hierarchy_res.err().unwrap());
                                    return result;
                                }
                            }
                        }
                        result = Ok(children);
                    },
                    None => {
                        println!("no symbols found");
                    }
                }
            } else {
                result = Err(doc_symbol_res.err().unwrap());
                return result;
            }
        } else {
            result = Err(document_res.err().unwrap());
            return result;
        }

        result
    }

    pub(crate) fn search_child_single_document(&mut self, function_name: String, document_name: &str) -> HashSet<String> {
        let mut result: HashSet<String> = HashSet::new();
        let document = self.lang_server.document_open(document_name).unwrap();

        let doc_symbol = self.lang_server.document_symbol(&document).unwrap();

        match doc_symbol {
            Some(DocumentSymbolResponse::Flat(_)) => {
                println!("unsupported symbols found");
            },
            Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                for symbol in doc_symbols {
                    if symbol.name == function_name {
                        let prep_call_hierarchy = self.lang_server.call_hierarchy_item(&document, symbol.range.start);
                        let outgoing_calls = self.lang_server.call_hierarchy_item_outgoing(prep_call_hierarchy.unwrap().unwrap()[0].clone());
                        for outgoing_call in outgoing_calls.unwrap().unwrap() {
                            result.insert(outgoing_call.to.name.to_string());
                        }
                        break;
                    }

                }
            },
            None => {
                println!("no symbols found");
            }
        }

        result
    }

}