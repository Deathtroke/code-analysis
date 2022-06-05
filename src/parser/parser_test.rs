use super::*;
use std::collections::HashSet;
use crate::searcher::{ForcedNode, LSPServer};

#[cfg(test)]
struct MockLSPServer;

impl MockLSPServer {
    fn new() -> Box<dyn LSPServer> {
        Box::new(MockLSPServer)
    }
}

impl LSPServer for MockLSPServer {
    fn search_parent(&mut self, search_target: String) -> HashSet<String> {
        if false {unimplemented!("{:?}", search_target)}
        let parents: HashSet<String> =
            HashSet::from(["parent1".to_string(), "parent2".to_string()]);

        parents
    }

    fn search_child(&mut self, search_target: String) -> HashSet<String> {
        if false {unimplemented!("{:?}", search_target)}
        let mut children: HashSet<String> =
            HashSet::from(["child1".to_string(), "child2".to_string()]);

        children
    }

    fn search_connection_filter(
        &mut self,
        parent_filter: HashMap<String, String>,
        child_filter: HashMap<String, String>,
    ) -> HashSet<(String, String)> {
        let mut result: HashSet<(String, String)> = HashSet::new();
        for parent in parent_filter.clone() {
            for child in child_filter.clone() {
                result.insert((parent.1.clone(), child.1.clone()));
            }
        }
        result
    }

    fn find_func_name(
        &mut self,
        filter: Vec<HashMap<FilterName, Regex>>,
    ) -> HashSet<FunctionNode> {
        let mut result: HashSet<FunctionNode> = HashSet::new();
        for f in filter {
            if f.contains_key(&FilterName::Function) {
                let forced = ForcedNode {
                    function_name: f.get(&FilterName::Function).unwrap().clone().as_str().to_string(),
                    document: "".to_string()
                };
                result.insert(FunctionNode {
                    function_name: forced.function_name.clone(),
                    document: forced.document.clone(),
                    match_strategy: Box::new(forced)
                });
            }
        }
        result
    }

    fn search_child_single_document_filter(
        &mut self,
        func_filter: Regex,
        child_filter: HashMap<String, String>,
        document_name: &str,
    ) -> HashSet<(String, String)> {
        if false {unimplemented!("{:?} {:?}", child_filter, document_name)}
        let mut children: HashSet<(String,String)> =
            HashSet::from([(func_filter.to_string(), "child1".to_string()), (func_filter.to_string(), "child2".to_string())]);

        children
    }

    fn search_parent_single_document_filter(
        &mut self,
        func_filter: Regex,
        parent_filter: HashMap<String, String>,
        document_name: &str,
    ) -> HashSet<(String, String)> {
        if false {unimplemented!("{:?} {:?}", parent_filter, document_name)}
        let mut parents: HashSet<(String,String)> =
            HashSet::from([("parent1".to_string(), func_filter.to_string()), ("parent2".to_string(), func_filter.to_string())]);

        parents
    }

    fn find_link(&mut self, parent_name: String, child_name: String, document_name: &str) -> bool {
        if false {unimplemented!("{:?}, {:?}, {:?}", parent_name, child_name, document_name)}

        true
    }

    fn find_functions_in_doc(
        &mut self,
        func_filter: Regex,
        document_name: &str,
    ) -> HashSet<String> {
        unimplemented!("{:?}, {:?}", func_filter, document_name)
    }
}

#[test]
fn test_parser_simple1() {
    let input = r#"{@func}"#;
    let mut parser = PestParser::new(MockLSPServer::new());
    assert!(Regex::new("parent[12]").unwrap().is_match(parser.parse(input).iter().nth(0).unwrap().function_name.clone().as_str()));
    assert!(Regex::new("parent[12]").unwrap().is_match(parser.parse(input).iter().nth(1).unwrap().function_name.clone().as_str()));

    let graph_output = HashSet::from([
        ("parent1".to_string(), Some("func".to_string())),
        ("parent2".to_string(), Some("func".to_string())),
    ]);
    assert_eq!(parser.graph.graph_to_tuple(), graph_output);
}

#[test]
fn test_parser_simple2() {
    let input = r#"@func {}"#;
    let mut parser = PestParser::new( MockLSPServer::new());

    assert_eq!(parser.parse(input).iter().nth(0).unwrap().function_name, "func".to_string());
    let graph_output = HashSet::from([
        ("func".to_string(), Some("child1".to_string())),
        ("func".to_string(), Some("child2".to_string())),
    ]);
    assert_eq!(parser.graph.graph_to_tuple(), graph_output);
}

#[test]
fn test_parser() {
    let input = r#"{{@func}}"#;
    let mut parser = PestParser::new( MockLSPServer::new());
    assert!(Regex::new("parent[12]").unwrap().is_match(parser.parse(input).iter().nth(0).unwrap().function_name.clone().as_str()));
    assert!(Regex::new("parent[12]").unwrap().is_match(parser.parse(input).iter().nth(1).unwrap().function_name.clone().as_str()));

    let graph_output = HashSet::from([
        ("parent1".to_string(), Some("func".to_string())),
        ("parent1".to_string(), Some("parent1".to_string())),
        ("parent1".to_string(), Some("parent2".to_string())),
        ("parent2".to_string(), Some("func".to_string())),
        ("parent2".to_string(), Some("parent1".to_string())),
        ("parent2".to_string(), Some("parent2".to_string())),
    ]);
    assert_eq!(parser.graph.graph_to_tuple(), graph_output);
    //let g : tabbycat::Graph = parser.graph.try_into().unwrap();
    //assert_eq!(g.to_string(), "This test is unusable")
}


#[test]
fn test_graph_only_node() {
    let input = r#"@foo"#;
    let mut parser = PestParser::new( MockLSPServer::new());
    assert_eq!(parser.parse(input).iter().nth(0).unwrap().function_name, "foo".to_string());

    let graph_output = HashSet::from([
        ("foo".to_string(), None),
    ]);
    assert_eq!(parser.graph.graph_to_tuple(), graph_output);
    //let g : tabbycat::Graph = parser.graph.try_into().unwrap();
    //assert_eq!(g.to_string(), "This test is unusable")
}