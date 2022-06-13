use serde::ser::{SerializeSeq, SerializeStruct};
use serde::{Serialize, Serializer};
use std::collections::HashSet;
use petgraph::graph::{NodeIndex, NodeIndices};

pub struct Graph {
    pub pet_graph: petgraph::Graph<String, ()>,
}

impl Graph {
    pub fn graph_to_tuple(&self) -> HashSet<(String, String)>{
        let mut result :HashSet<(String, String)> = HashSet::new();
        for edge in self.pet_graph.raw_edges() {
            let mut target = "".to_string();
            if self.pet_graph.node_weight(edge.target()).is_some(){
                target = self.pet_graph.node_weight(edge.target()).unwrap().to_owned();
            }
            result.insert((self.pet_graph.node_weight(edge.source()).unwrap().to_owned(), target));
        }
        result
    }

    pub fn add_node(&mut self, node_name: String) -> NodeIndex {
        let mut result_index = NodeIndex::new(0);
        let mut node_exists = false;
        for node in self.pet_graph.node_indices(){
            if self.pet_graph.node_weight(node).unwrap().to_owned() == node_name {
                result_index = node;
                node_exists = true;
            }
        }
        if !node_exists {
            result_index = self.pet_graph.add_node(node_name);
        }
        result_index
    }

    pub fn graph_to_dot(&mut self) -> String {
        format!("{:?}",  petgraph::dot::Dot::new(&self.pet_graph.clone()))
    }
}
/*
impl TryFrom<Graph> for tabbycat::Graph {
    type Error = anyhow::Error;

    fn try_from(g: Graph) -> Result<Self, Self::Error> {
        let mut stmts = tabbycat::StmtList::new();

        for edge in &g.edges {
            if edge.node_to.is_some() {
                stmts = stmts.add_edge(
                    tabbycat::Edge::head_node(tabbycat::Identity::id(edge.node_from.as_str())?, None)
                        .arrow_to_node(tabbycat::Identity::id(edge.node_to.as_ref().unwrap().as_str())?, None),
                );
            } else {
                stmts = stmts.add_edge(
                    tabbycat::Edge::head_node(tabbycat::Identity::id(edge.node_from.as_str())?, None)
                );
            }
        }

        tabbycat::GraphBuilder::default()
            .graph_type(tabbycat::GraphType::DiGraph)
            .strict(false)
            .id(tabbycat::Identity::id("G").unwrap())
            .stmts(stmts)
            .build()
            .map_err(anyhow::Error::msg)
    }
}*/
