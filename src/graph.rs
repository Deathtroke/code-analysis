use serde::ser::{SerializeSeq, SerializeStruct};
use serde::{Serialize, Serializer};
use std::collections::HashSet;

pub struct Graph {
    pub edges: HashSet<Edge>,
}

#[derive(Eq, Hash, PartialEq)]
pub struct Edge {
    edge_properties: Option<String>,
    node_from: String,
    node_to: String,
}

impl Serialize for Edge {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("Edge", 3)?;
        s.serialize_field("edge_properties", &self.edge_properties)?;
        s.serialize_field("from_node", &self.node_from)?;
        s.serialize_field("to_node", &self.node_to)?;
        s.end()
    }
}

impl Serialize for Graph {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_seq(Some(self.edges.len()))?;
        for edge in &self.edges {
            s.serialize_element(&edge)?;
        }
        s.end()
    }
}

impl Graph {
    pub fn graph_to_tuple(&mut self) -> HashSet<(String, String)> {
        let mut tuples: HashSet<(String, String)> = HashSet::new();
        for edge in &self.edges {
            tuples.insert((edge.node_from.clone(), edge.node_to.clone()));
        }
        tuples
    }

    pub fn insert_edge(&mut self, option: Option<String>, from: String, to: String) {
        let edge = Edge {
            edge_properties: option,
            node_from: from,
            node_to: to,
        };
        self.edges.insert(edge);
    }
}

impl TryFrom<Graph> for tabbycat::Graph {
    type Error = anyhow::Error;

    fn try_from(g: Graph) -> Result<Self, Self::Error> {
        let mut stmts = tabbycat::StmtList::new();

        for edge in &g.edges {
            stmts = stmts.add_edge(
                tabbycat::Edge::head_node(tabbycat::Identity::id(edge.node_from.as_str())?, None)
                    .arrow_to_node(tabbycat::Identity::id(edge.node_to.as_str())?, None),
            );
        }

        tabbycat::GraphBuilder::default()
            .graph_type(tabbycat::GraphType::DiGraph)
            .strict(false)
            .id(tabbycat::Identity::id("G").unwrap())
            .stmts(stmts)
            .build()
            .map_err(anyhow::Error::msg)
    }
}
