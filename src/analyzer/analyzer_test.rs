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
    fn restart(&mut self) {
        unimplemented!();
    }

    fn find_func_name(
        &mut self,
        filter: Vec<HashMap<FilterName, Regex>>,
    ) -> HashSet<FunctionNode> {
        let mut result: HashSet<FunctionNode> = HashSet::new();
        if filter.len() == 0 {
            let forced = ForcedNode {
                function_name: HashSet::from(["parent1".to_string(),"parent2".to_string(),"child1".to_string(), "child2".to_string()]),
            };
            result.insert(FunctionNode {
                function_name: forced.function_name.clone(),
                match_strategy: Box::new(forced)
            });
        }
        for f in filter {
            if f.contains_key(&FilterName::Function) {
                let forced = ForcedNode {
                    function_name: HashSet::from(["parent1".to_string(),"parent2".to_string()]),
                };
                result.insert(FunctionNode {
                    function_name: forced.function_name.clone(),
                    match_strategy: Box::new(forced)
                });
            }
            if f.contains_key(&FilterName::FunctionNameFromIdent) {
                let forced = ForcedNode {
                    function_name: HashSet::from([f.get(&FilterName::FunctionNameFromIdent).unwrap().clone().as_str().to_string()]),
                };
                result.insert(FunctionNode {
                    function_name: forced.function_name.clone(),
                    match_strategy: Box::new(forced)
                });
            }
        }
        result
    }

    fn find_link(&mut self, parent_name: HashSet<String>, child_name: HashSet<String>) -> HashSet<(String, String)> {
        let mut result: HashSet<(String, String)> = HashSet::new();
        for parent in parent_name.clone() {
            for child in child_name.clone() {
                result.insert((parent.clone(), child.clone()));
            }
        }
        result
    }

    fn close(&mut self) {
        unimplemented!()
    }
}

#[test]
fn test_parser_simple1() {
    let input = r#"{@func}"#;
    let mut parser = Analyzer::new(MockLSPServer::new());
    parser.parse(input);

    let graph_output = HashSet::from([
        ("parent1".to_string(), "func".to_string()),
        ("parent2".to_string(), "func".to_string()),
    ]);

    assert_eq!(parser.graph.graph_to_tuple(), graph_output);
}

#[test]
fn test_parser() {
    let input = r#"{{@func}}"#;
    let mut parser = Analyzer::new( MockLSPServer::new());

    parser.parse(input);

    let graph_output = HashSet::from([
        ("parent1".to_string(), "func".to_string()),
        ("parent2".to_string(), "func".to_string()),
        ("parent1".to_string(), "parent2".to_string()),
        ("parent2".to_string(), "parent1".to_string()),
    ]);
    assert_eq!(parser.graph.graph_to_tuple(), graph_output);
    //let g : tabbycat::Graph = analyzer.graph.try_into().unwrap();
    //assert_eq!(g.to_string(), "This test is unusable")
}


#[test]
fn test_graph_only_node() {
    let input = r#"@foo"#;
    let mut parser = Analyzer::new( MockLSPServer::new());

    parser.parse(input);

    assert_eq!(parser.graph.graph_to_tuple(), HashSet::new());
    for node in parser.graph.pet_graph.raw_nodes().to_owned(){
        assert_eq!(node.weight, "foo".to_string());
    }
    //let g : tabbycat::Graph = analyzer.graph.try_into().unwrap();
    //assert_eq!(g.to_string(), "This test is unusable")
}