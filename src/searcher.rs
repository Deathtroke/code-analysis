use crate::lang_server::LanguageServer;
use crate::lang_server;
use lsp_types::{DocumentSymbolResponse, SymbolKind};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::prelude::*;
use log::{Level, log};
use serde_json::Value;
use crate::analyzer::FilterName;

pub trait LSPServer {
    fn restart(&mut self,);
    fn search_connection_filter(
        &mut self,
        parent_name: String,
        child_name: String,
    ) -> HashSet<(String, String)>;
    fn find_func_name(
        &mut self,
        filter: Vec<HashMap<FilterName, Regex>>,
    ) -> HashSet<FunctionNode>;
    fn search_child_single_document_filter(
        &mut self,
        func_filter: Regex,
        child_filter: HashMap<String, String>,
        document_name: &str,
    ) -> HashSet<(String, String)>;
    fn search_parent_single_document_filter(
        &mut self,
        func_filter: Regex,
        parent_filter: HashMap<String, String>,
        document_name: &str,
    ) -> HashSet<(String, String)>;
    fn find_link(&mut self, parent_name: String, child_name: String, document_name: &str) -> bool;
    fn find_functions_in_doc(&mut self, func_filter: Regex, ident: Option<String>, document_name: &str)
        -> HashSet<String>;
    fn close(&mut self);
}

pub struct ClangdServer {
    pub lang_server: Box<dyn LanguageServer>,
    pub files_in_project: Vec<String>,
    pub project_path: String,
    pub index_map: HashMap<String, Vec<String>>,
    use_call_hierarchy_outgoing: bool,
    clangd_path: String,
}

pub struct FunctionNode {
    pub function_name: String,
    pub document: String,
    pub match_strategy: Box<dyn MatchFunctionEdge>
}

impl Hash for FunctionNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.function_name.hash(state);
        self.document.hash(state);
    }
}

impl PartialEq for FunctionNode {
    fn eq(&self, other: &Self) -> bool {
        self.document == other.document && self.function_name == other.function_name
    }
}

impl Eq for FunctionNode {}

impl Clone for FunctionNode {
    fn clone(&self) -> Self {
        match self.match_strategy.get_implementation().as_str() {
            "ForcedEdge" => {
                let strategy = ForcedNode { function_name: self.function_name.clone(), document: self.document.clone() };
                FunctionNode{
                    function_name: self.function_name.clone(),
                    document: self.document.clone(),
                    match_strategy: Box::new(strategy),
                }

            }
            "ParentChildEdge" => {
                let strategy = ParentChildNode { function_name: self.function_name.clone(), document: self.document.clone() };
                FunctionNode{
                    function_name: self.function_name.clone(),
                    document: self.document.clone(),
                    match_strategy: Box::new(strategy),
                }
            }
            _ => {unimplemented!()}
        }
    }
}

pub trait MatchFunctionEdge {
    fn do_match(&mut self, match_target: FunctionNode, lsp_server: &mut Box<dyn LSPServer>) -> bool;
    fn get_implementation(&self) -> String;
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct ForcedNode {
    pub function_name: String,
    pub document: String,
}

pub struct ParentChildNode {
    pub function_name: String,
    pub document: String,
}

impl MatchFunctionEdge for ForcedNode {
    fn do_match(&mut self, match_target: FunctionNode, lsp_server: &mut Box<dyn LSPServer>) -> bool {
        #[allow(dead_code)]
        if false { drop(match_target); drop(lsp_server); unimplemented!()}
        true
    }
    fn get_implementation(&self) -> String {
        "ForcedEdge".to_string()
    }

}

impl MatchFunctionEdge for ParentChildNode {
    fn do_match(&mut self, match_target: FunctionNode, lsp_server: &mut Box<dyn LSPServer>) -> bool {
        lsp_server.find_link(self.function_name.clone(), match_target.function_name, self.document.as_str())
    }
    fn get_implementation(&self) -> String{
        "ParentChildEdge".to_string()
    }

}

impl ClangdServer {
    pub fn new(project_path: String, clangd_path: String) -> Box<dyn LSPServer> {
        let mut lsp_server = Self {
            lang_server: lang_server::LanguageServerLauncher::new()
                .server(clangd_path.to_owned())
                .project(project_path.to_owned())
                .launch()
                .expect("Failed to spawn clangd"),
            files_in_project: vec![],
            project_path,
            index_map: HashMap::new(),
            use_call_hierarchy_outgoing: true,
            clangd_path
        };
        lsp_server.files_in_project = lsp_server.get_all_files_in_project();
        let res = lsp_server.lang_server.initialize();
        if res.is_err() {
            log!(Level::Error,"LSP server didn't initialize: {:?}", res.err());
        }
        Box::new(lsp_server)
    }

    pub fn restart_server(&mut self){
        let shutdown_res = self.lang_server.exit();
        if shutdown_res.is_err() {
            log!(Level::Error, "{:?}", shutdown_res.err());
        }
        let new_lsp = lang_server::LanguageServerLauncher::new()
            .server(self.clangd_path.to_owned())
            .project(self.project_path.to_owned())
            .launch()
            .expect("Failed to spawn clangd");
        self.lang_server = new_lsp;

        let init_res = self.lang_server.initialize();
        if init_res.is_err() {
            eprintln!("{:?}", init_res.err());
        }
    }

    pub fn get_all_files_in_project(&mut self) -> Vec<String> {
        let files: Vec<String>;
        let path_to_index = self.project_path.clone() + "/.cache/clangd/index";
        let index_dir  = fs::read_dir(path_to_index.clone());

        if index_dir.is_ok() {
            let mut index_file_names: Vec<String> = vec![];
            for file in index_dir.unwrap() {
                let mut file_str = file.as_ref().unwrap().path().to_str().unwrap().to_owned();
                file_str = file_str.replace(&(path_to_index.clone() + "/"), "");
                file_str = file_str[..file_str.find(".").unwrap()].to_owned();
                index_file_names.push(file_str);
            }
            files = self.get_files_in_dir(self.project_path.clone(), self.project_path.clone(), Some(index_file_names.clone()));

            self.index_map = self.check_index_file(files.clone(), self.clangd_path.clone());
        } else {
            files = self.get_files_in_dir(self.project_path.clone(), self.project_path.clone(), None);

        }
        files
    }

    fn check_index_file(&mut self, files: Vec<String>, clangd_path: String) -> HashMap<String, Vec<String>> {
        let mut index_map : HashMap<String, Vec<String>> = HashMap::new();

        let path = self.project_path.clone() + "/.cache/index.json";
        let mut needs_indexing = false;
        let file =  File::open(&path);
        if file.is_err() {
            needs_indexing = true;
        } else {
            let mut s = String::new();
            match file.unwrap().read_to_string(&mut s) {
                Err(why) => panic!("could not read: {}", why),
                Ok(_) => {}
            }

            let json = serde_json::from_str::<Value>(s.as_str());
            if json.is_err() {
                needs_indexing = true;
            } else {
                let json_map = json.unwrap().as_object().unwrap().to_owned();
                if json_map.len() == files.len() {
                    for file in json_map{
                        if files.contains(&file.0) {
                            let mut functions: Vec<String> = vec![];
                            let symbols = file.1.as_array().unwrap().to_owned();
                            for symbol in symbols {
                                functions.push(symbol.to_string().replace("\"", ""));
                            }
                            index_map.insert(file.0, functions.clone());
                        } else {
                            needs_indexing = true;
                        }
                    }
                } else {
                    needs_indexing = true;
                }
            }

        }
        if needs_indexing {
            let mut i = 0;
            let mut i_total = 0;
            eprintln!("start indexing, there should be a message displaying the progress every coupe of seconds, please restart the program if the messages stop unexpectedly");
            for file in files.clone() {
                i += 1; i_total += 1;
                let mut functions:Vec<String> = vec![];

                if i >= 10 {
                    //break;
                    i = 0;
                    eprintln!("indexing project, please wait ({}/{})", i_total, files.clone().len());
                    self.restart_server();
                }
                let document_res = self.lang_server.document_open(file.as_str());
                if document_res.is_ok() {
                    let document = document_res.unwrap();
                    let doc_symbol = self.lang_server.document_symbol(&document);
                    if doc_symbol.is_ok() {
                        match doc_symbol.unwrap() {
                            Some(DocumentSymbolResponse::Flat(_)) => {
                                log!(Level::Warn ,"unsupported symbols found");
                            }
                            Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                                for symbol in doc_symbols {
                                    if symbol.kind == SymbolKind::FUNCTION {
                                        functions.push(symbol.name);
                                    }
                                }
                            }
                            None => {
                                log!(Level::Warn, "no symbols found");
                            }
                        }
                    }
                }
                index_map.insert(file, functions);
            }

            let new_json = serde_json::to_string(&index_map).unwrap();
            let mut file_ref = File::create(path).expect("create failed");
            file_ref.write_all(new_json.as_bytes()).expect("write failed");
        }
        index_map
    }

    fn get_files_in_dir(&self, dir: String, project_path: String, index_file_name: Option<Vec<String>>) -> Vec<String> {
        let mut files: Vec<String> = Vec::new();

        let paths = fs::read_dir(dir.clone()).unwrap();

        for path in paths {
            let path_str = path.as_ref().unwrap().path().to_str().unwrap().to_string();
            if path.as_ref().unwrap().metadata().unwrap().is_dir() {
                let mut subfolder = self.get_files_in_dir(path_str, project_path.clone(), index_file_name.clone());
                files.append(&mut subfolder);
            } else {
                if index_file_name.clone().is_some() {
                    let mut name = path_str.replace(&(project_path.clone().as_str().to_owned() + "/"), "");
                    while name.find("/").is_some() {
                        name = name[(name.find("/").unwrap()+1)..].to_owned();
                    }
                    if name.find(".").is_some() {
                        name = name[..name.find(".").unwrap()].to_owned();

                        if index_file_name.clone().unwrap().contains(&name){
                            if path_str.ends_with(".cpp") || path_str.ends_with(".c") {
                                files.push(path_str.replace(&(project_path.clone().as_str().to_owned() + "/"), ""));
                            }
                        }
                    }

                } else {
                    if path_str.ends_with(".cpp") || path_str.ends_with(".c") {
                        files.push(path_str.replace(&(project_path.clone().as_str().to_owned() + "/"), ""));
                    }
                }
            }
        }
        files
    }
}

impl LSPServer for ClangdServer {
    fn restart(&mut self) {
        self.restart_server();
    }

    fn search_connection_filter(
        &mut self,
        parent_name: String,
        child_name: String,
    ) -> HashSet<(String, String)> {
        let mut connections: HashSet<(String, String)> = HashSet::new();

        if parent_name.as_str() == "" {
            for file_path in self.files_in_project.clone() {
                let path = self.project_path.clone() + "/" + file_path.as_str();
                let mut file = match File::open(&path) {
                    Err(why) => panic!("could not open: {}", why),
                    Ok(file) => file,
                };
                let mut s = String::new();
                match file.read_to_string(&mut s) {
                    Err(why) => panic!("could not read: {}", why),
                    Ok(_) => {}
                }

                let mut new_children = HashSet::new();
                if s.contains(&child_name.clone()) {
                    new_children = self.search_parent_single_document_filter(
                        Regex::new(child_name.as_str()).unwrap(),
                        HashMap::new(),
                        file_path.as_str(),
                    );
                }
                for child in new_children {
                    connections.insert(child);
                }
            }
        } else {
            let mut doc_name = String::new();
            for doc in self.index_map.clone() {
                for func in doc.1 {
                    if func == parent_name {
                        doc_name = doc.0.clone();
                    }
                }
            }
            if doc_name.clone() == "" {
                for file_path in self.files_in_project.clone() {
                    let path = self.project_path.clone() + "/" + file_path.as_str();
                    let mut file = match File::open(&path) {
                        Err(why) => panic!("could not open: {}", why),
                        Ok(file) => file,
                    };
                    let mut s = String::new();
                    match file.read_to_string(&mut s) {
                        Err(why) => panic!("could not read: {}", why),
                        Ok(_) => {}
                    }

                    let mut new_children = HashSet::new();
                    if s.contains(&parent_name.clone()) {
                        new_children = self.search_child_single_document_filter(
                            Regex::new(parent_name.as_str()).unwrap(),
                            HashMap::new(),
                            file_path.as_str(),
                        );
                    }
                    for child in new_children {
                        connections.insert(child);
                    }
                }
            } else {
                for file_path in self.files_in_project.clone() {
                    if file_path.contains(&doc_name.clone()) {
                        let mut new_children = HashSet::new();
                        new_children = self.search_child_single_document_filter(
                            Regex::new(parent_name.as_str()).unwrap(),
                            HashMap::new(),
                            file_path.as_str(),
                        );

                        for child in new_children {
                            connections.insert(child);
                        }
                    }
                }
            }
        }
        connections
    }

    fn find_func_name(
        &mut self,
        filter: Vec<HashMap<FilterName, Regex>>,
    ) -> HashSet<FunctionNode> {
        let mut func_names: HashSet<FunctionNode> = HashSet::new();
        for f in filter {
            let mut forced = false;
            if f.contains_key(&FilterName::Forced) {
                forced = true;
            }
            let mut only_ident = false;
            let mut ident = String::new();
            if f.contains_key(&FilterName::FunctionNameFromIdent) {
                ident = f.get(&FilterName::FunctionNameFromIdent).unwrap().to_string();
                only_ident = true;
            }

            let mut file_filter = Regex::new(".").unwrap();
            if f.contains_key(&FilterName::File) {
                let regex = f.get(&FilterName::File).unwrap();
                file_filter = regex.to_owned();
            }

            let mut function_filter = Regex::new(".").unwrap();
            if f.contains_key(&FilterName::Function) {
                let regex = f.get(&FilterName::Function).unwrap();
                function_filter = regex.to_owned();
            }
            for file_path in self.files_in_project.clone() {
                if file_filter.is_match(file_path.as_str()) {
                    let path = self.project_path.clone() + "/" + file_path.as_str();
                    let mut file = match File::open(&path) {
                        Err(why) => panic!("could not open: {}", why),
                        Ok(file) => file,
                    };
                    let mut s = String::new();
                    match file.read_to_string(&mut s) {
                        Err(why) => panic!("could not read: {}", why),
                        Ok(_) => {}
                    }

                    let need_lsp = (function_filter.is_match(s.as_str()) && !only_ident) || (s.contains(&ident.clone()) && only_ident);

                    if need_lsp {
                        let ident_opt :Option<String> = if only_ident {
                             Some(ident.clone())
                        } else {
                            None
                        };

                        let names =
                            self.find_functions_in_doc(function_filter.clone(), ident_opt, file_path.as_str());
                        for name in names {
                            if forced {
                                let node = ForcedNode {
                                    function_name: name.clone(),
                                    document: file_path.clone()
                                };
                                func_names.insert( FunctionNode{function_name: name.clone(), document: file_path.clone(), match_strategy: Box::new(node)});
                            } else {
                                let node = ParentChildNode {
                                    function_name: name.clone(),
                                    document: file_path.clone()
                                };
                                func_names.insert( FunctionNode{function_name: name.clone(), document: file_path.clone(), match_strategy: Box::new(node)});

                            }
                        }
                    }
                }
            }
            if func_names.len() == 0 && function_filter.as_str() != "." {
                let node = ParentChildNode {
                    function_name: function_filter.as_str().to_string(),
                    document: "not found".to_string()
                };
                func_names.insert( FunctionNode{function_name: node.function_name.clone(), document: node.document.clone(), match_strategy: Box::new(node)});

            }
        }


        func_names
    }
    fn search_child_single_document_filter(
        &mut self,
        func_filter: Regex,
        child_filter: HashMap<String, String>,
        document_name: &str,
    ) -> HashSet<(String, String)> {
        let mut result: HashSet<(String, String)> = HashSet::new();
        let document = self.lang_server.document_open(document_name).unwrap();

        let mut file_filter_c = Regex::new(".").unwrap(); //any
        if child_filter.contains_key("file") {
            file_filter_c = Regex::new(child_filter.get("file").unwrap().as_str()).unwrap();
        }
        let mut func_filter_c = Regex::new(".").unwrap(); //any
        if child_filter.contains_key("function") {
            func_filter_c = Regex::new(child_filter.get("function").unwrap().as_str()).unwrap();
        }

        let doc_symbol = self.lang_server.document_symbol(&document).unwrap();

        match doc_symbol {
            Some(DocumentSymbolResponse::Flat(token)) => {
                log!(Level::Warn ,"unsupported symbols found {:?}", token);
            }
            Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                for symbol in doc_symbols {
                    if symbol.kind == SymbolKind::FUNCTION {
                        let func_name = symbol.name;
                        let filter = Regex::new((func_filter.to_string() + "zzz").as_str()).unwrap();
                        let function_name_for_filter = func_name.clone() + "zzz";
                        if filter.is_match(function_name_for_filter.as_str()) {
                            let mut unsuccessful_response = false;

                            if self.use_call_hierarchy_outgoing {
                                let prep_call_hierarchy = self
                                    .lang_server
                                    .call_hierarchy_item(&document, symbol.range.start);
                                let call_hierarchy_array = prep_call_hierarchy.unwrap().unwrap();
                                for call_hierarchy_item in call_hierarchy_array {
                                    let outgoing_calls = self
                                        .lang_server
                                        .call_hierarchy_item_outgoing(call_hierarchy_item.clone());
                                    if outgoing_calls.is_ok() {
                                        unsuccessful_response = true;
                                        for outgoing_call in outgoing_calls.unwrap().unwrap() {
                                            unsuccessful_response = false;
                                            if func_filter_c.is_match(outgoing_call.to.name.as_str())
                                                && file_filter_c.is_match(outgoing_call.to.uri.as_str())
                                            {
                                                if outgoing_call.to.kind == SymbolKind::FUNCTION {
                                                    result.insert((
                                                        func_name.clone(),
                                                        outgoing_call.to.name.to_string(),
                                                    ));
                                                }
                                            }
                                        }
                                    } else {
                                        unsuccessful_response = true;
                                        self.use_call_hierarchy_outgoing = false;
                                    }
                                }
                            } else {
                                unsuccessful_response = true;
                            }
                            if unsuccessful_response {
                                let doc_text = document.text.clone();
                                let doc_lines: Vec<&str> = doc_text.split("\n").collect();
                                let start: usize = (symbol.range.start.line + 1) as usize;
                                let end: usize = symbol.range.end.line as usize;
                                if start < end {
                                    let function_data = doc_lines[start..end].concat();
                                    for file in self.index_map.clone() {
                                        if file_filter_c.is_match(file.0.as_str()){
                                            for function_name in file.1 {
                                                if func_filter_c.is_match(function_name.as_str()) {
                                                    let search_name = function_name.clone() + "(";
                                                    if function_data.contains(&search_name) {
                                                        result.insert((func_name.clone(), function_name.clone()));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                            }
                        }
                    }
                }
            }
            None => {
                log!(Level::Warn, "no symbols found");
            }
        }

        result
    }

    fn search_parent_single_document_filter(
        &mut self,
        func_filter: Regex,
        parent_filter: HashMap<String, String>,
        document_name: &str,
    ) -> HashSet<(String, String)> {
        let mut result: HashSet<(String, String)> = HashSet::new();
        let document = self.lang_server.document_open(document_name).unwrap();

        let mut file_filter_c = Regex::new(".").unwrap(); //any
        if parent_filter.contains_key("file") {
            file_filter_c = Regex::new(parent_filter.get("file").unwrap().as_str()).unwrap();
        }
        let mut func_filter_c = Regex::new(".").unwrap(); //any
        if parent_filter.contains_key("function") {
            func_filter_c = Regex::new(parent_filter.get("function").unwrap().as_str()).unwrap();
        }

        let doc_symbol = self.lang_server.document_symbol(&document).unwrap();

        match doc_symbol {
            Some(DocumentSymbolResponse::Flat(_)) => {
                log!(Level::Warn ,"unsupported symbols found");
            }
            Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                for symbol in doc_symbols {
                    if symbol.kind == SymbolKind::FUNCTION {
                        let func_name = symbol.name;
                        if func_filter.is_match(func_name.as_str()) {
                            let prep_call_hierarchy = self
                                .lang_server
                                .call_hierarchy_item(&document, symbol.range.start);
                            let call_hierarchy_array = prep_call_hierarchy.unwrap().unwrap();
                            for call_hierarchy_item in call_hierarchy_array {
                                let incoming_calls = self
                                    .lang_server
                                    .call_hierarchy_item_incoming(call_hierarchy_item.clone());
                                for incoming_call in incoming_calls.unwrap().unwrap() {
                                    if func_filter_c.is_match(incoming_call.from.name.as_str())
                                        && file_filter_c.is_match(incoming_call.from.uri.as_str())
                                    {
                                        if incoming_call.from.kind == SymbolKind::FUNCTION {
                                            result.insert((
                                                incoming_call.from.name.to_string(),
                                                func_name.clone(),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            None => {
                log!(Level::Warn, "no symbols found");
            }
        }

        result
    }

    fn find_link(&mut self, parent_name: String, child_name: String, document_name: &str) -> bool {
        let document_res = self.lang_server.document_open(document_name);
        if document_res.is_ok() {
            let document = document_res.unwrap();
            let doc_symbol = self.lang_server.document_symbol(&document).unwrap();

            match doc_symbol {
                Some(DocumentSymbolResponse::Flat(_)) => {
                    log!(Level::Warn ,"unsupported symbols found");
                }
                Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                    for symbol in doc_symbols {
                        if symbol.kind == SymbolKind::FUNCTION {
                            let func_name = symbol.name;
                            if child_name == func_name {
                                let prep_call_hierarchy = self
                                    .lang_server
                                    .call_hierarchy_item(&document, symbol.range.start);
                                let call_hierarchy_array = prep_call_hierarchy.unwrap().unwrap();
                                if call_hierarchy_array.len() > 0 {
                                    let incoming_calls =
                                        self.lang_server.call_hierarchy_item_incoming(
                                            call_hierarchy_array[0].clone(),
                                        );
                                    for incoming_call in incoming_calls.unwrap().unwrap() {
                                        if incoming_call.from.name.as_str() == parent_name {
                                            return true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                None => {
                    log!(Level::Warn, "no symbols found");
                }
            }
        }
        false
    }

    fn find_functions_in_doc(
        &mut self,
        func_filter: Regex,
        ident: Option<String>,
        document_name: &str,
    ) -> HashSet<String> {
        let mut result = HashSet::new();
        let document_res = self.lang_server.document_open(document_name);
        if document_res.is_ok() {
            let document = document_res.unwrap();

            let doc_symbol = self.lang_server.document_symbol(&document).unwrap();

            match doc_symbol {
                Some(DocumentSymbolResponse::Flat(_)) => {
                    log!(Level::Warn ,"unsupported symbols found");
                }
                Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                    for symbol in doc_symbols {
                        if symbol.kind == SymbolKind::FUNCTION {
                            let func_name = symbol.name;
                            if ident.clone().is_some() {
                                if func_name == ident.clone().unwrap() {
                                    result.insert(func_name.to_string());

                                }
                            } else {
                                if func_filter.is_match(func_name.as_str()) {
                                    result.insert(func_name.to_string());

                                }
                            }
                        }
                    }
                }
                None => {
                    log!(Level::Warn, "no symbols found");
                }
            }
        }
        result
    }

    fn close(&mut self){
        log!(Level::Info, "{:?}", self.lang_server.shutdown());
        log!(Level::Info, "{:?}", self.lang_server.exit());
    }
}
