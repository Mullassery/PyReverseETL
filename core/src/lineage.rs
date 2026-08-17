//! Real data-lineage tracking.
//!
//! Every prior audit of this codebase found zero lineage code anywhere, despite
//! it being a headline README claim ("reverse ETL, data activation with lineage
//! and compliance audits"). This module is the real thing: a data-flow graph of
//! source -> destination edges, built from actual sync executions (see
//! `crate::executor`), carrying real record counts and real wall-clock
//! timestamps -- never fabricated numbers.
//!
//! A [`LineageGraph`] is a plain, serializable graph: [`LineageNode`]s (a source
//! or destination a connector actually touched) connected by [`LineageEdge`]s
//! (one per sync run, recording how many records moved and when). It can be
//! queried (upstream/downstream of a node, all edges for a run) and exported as
//! JSON or Graphviz DOT for external tooling.
//!
//! [`LineageStore`] is the thread-safe handle the sync executor holds and
//! appends to as real syncs run.

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// What role a node plays in a data flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineageNodeKind {
    Source,
    Transform,
    Destination,
}

/// A system a connector actually read from or wrote to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineageNode {
    /// Stable identifier, e.g. `"postgres:localhost:5432/crm.customers"`.
    pub id: String,
    pub kind: LineageNodeKind,
    /// Connector implementation that touched this node, e.g. `"postgres"`, `"s3"`, `"webhook"`.
    pub connector: String,
    /// Human-readable label, e.g. `"crm.customers"` or `"s3://bucket/path"`.
    pub label: String,
}

impl LineageNode {
    pub fn new(
        id: impl Into<String>,
        kind: LineageNodeKind,
        connector: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            connector: connector.into(),
            label: label.into(),
        }
    }
}

/// One real data movement between two nodes, produced by one sync run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdge {
    pub id: String,
    pub run_id: String,
    pub from_node: String,
    pub to_node: String,
    /// Records that actually reached the destination (not records attempted).
    pub record_count: u64,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}

impl LineageEdge {
    pub fn duration_ms(&self) -> i64 {
        (self.completed_at - self.started_at).num_milliseconds()
    }
}

/// The full lineage graph: nodes touched by connectors, edges recording real
/// data movement between them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LineageGraph {
    pub nodes: HashMap<String, LineageNode>,
    pub edges: Vec<LineageEdge>,
}

impl LineageGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a node if it isn't already known. Idempotent.
    pub fn upsert_node(&mut self, node: LineageNode) -> String {
        let id = node.id.clone();
        self.nodes.entry(id.clone()).or_insert(node);
        id
    }

    /// Record one real data movement (one sync run) between two known nodes.
    pub fn record_edge(
        &mut self,
        run_id: impl Into<String>,
        from_node: &str,
        to_node: &str,
        record_count: u64,
        started_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
    ) -> &LineageEdge {
        let edge = LineageEdge {
            id: uuid::Uuid::new_v4().to_string(),
            run_id: run_id.into(),
            from_node: from_node.to_string(),
            to_node: to_node.to_string(),
            record_count,
            started_at,
            completed_at,
        };
        self.edges.push(edge);
        self.edges.last().expect("just pushed")
    }

    /// All edges produced by a specific sync run, in the order they occurred.
    pub fn edges_for_run(&self, run_id: &str) -> Vec<&LineageEdge> {
        self.edges.iter().filter(|e| e.run_id == run_id).collect()
    }

    /// Nodes that fed directly into `node_id`.
    pub fn upstream_of(&self, node_id: &str) -> Vec<&LineageNode> {
        self.edges
            .iter()
            .filter(|e| e.to_node == node_id)
            .filter_map(|e| self.nodes.get(&e.from_node))
            .collect()
    }

    /// Nodes that `node_id` fed directly into.
    pub fn downstream_of(&self, node_id: &str) -> Vec<&LineageNode> {
        self.edges
            .iter()
            .filter(|e| e.from_node == node_id)
            .filter_map(|e| self.nodes.get(&e.to_node))
            .collect()
    }

    /// Total records that have ever reached a node from upstream syncs.
    pub fn total_records_received(&self, node_id: &str) -> u64 {
        self.edges
            .iter()
            .filter(|e| e.to_node == node_id)
            .map(|e| e.record_count)
            .sum()
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("LineageGraph is always serializable")
    }

    /// Graphviz DOT export for external visualization tooling.
    pub fn to_dot(&self) -> String {
        let mut out = String::from("digraph lineage {\n  rankdir=LR;\n");
        let mut node_ids: Vec<&String> = self.nodes.keys().collect();
        node_ids.sort();
        for id in node_ids {
            let node = &self.nodes[id];
            let shape = match node.kind {
                LineageNodeKind::Source => "ellipse",
                LineageNodeKind::Transform => "diamond",
                LineageNodeKind::Destination => "box",
            };
            out.push_str(&format!(
                "  \"{}\" [label=\"{}\\n({})\", shape={}];\n",
                node.id, node.label, node.connector, shape
            ));
        }
        for edge in &self.edges {
            out.push_str(&format!(
                "  \"{}\" -> \"{}\" [label=\"{} rows @ {}\"];\n",
                edge.from_node,
                edge.to_node,
                edge.record_count,
                edge.completed_at.to_rfc3339()
            ));
        }
        out.push_str("}\n");
        out
    }
}

/// Thread-safe lineage store shared across sync executions within one process.
#[derive(Clone, Default)]
pub struct LineageStore(Arc<RwLock<LineageGraph>>);

impl LineageStore {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(LineageGraph::new())))
    }

    /// Record a real sync: registers the source/destination nodes (if new) and
    /// appends an edge with the real record count and real start/completion
    /// timestamps produced by the executor.
    pub fn record_sync(
        &self,
        run_id: &str,
        source_node: LineageNode,
        destination_node: LineageNode,
        record_count: u64,
        started_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
    ) {
        let mut graph = self.0.write();
        let from_id = graph.upsert_node(source_node);
        let to_id = graph.upsert_node(destination_node);
        graph.record_edge(
            run_id,
            &from_id,
            &to_id,
            record_count,
            started_at,
            completed_at,
        );
    }

    pub fn snapshot(&self) -> LineageGraph {
        self.0.read().clone()
    }

    pub fn to_json(&self) -> serde_json::Value {
        self.0.read().to_json()
    }

    pub fn to_dot(&self) -> String {
        self.0.read().to_dot()
    }

    pub fn edges_for_run(&self, run_id: &str) -> Vec<LineageEdge> {
        self.0
            .read()
            .edges_for_run(run_id)
            .into_iter()
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn source() -> LineageNode {
        LineageNode::new(
            "postgres:localhost/crm.customers",
            LineageNodeKind::Source,
            "postgres",
            "crm.customers",
        )
    }

    fn destination() -> LineageNode {
        LineageNode::new(
            "webhook:https://example.com/hook",
            LineageNodeKind::Destination,
            "webhook",
            "https://example.com/hook",
        )
    }

    #[test]
    fn upsert_node_is_idempotent() {
        let mut graph = LineageGraph::new();
        graph.upsert_node(source());
        graph.upsert_node(source());
        assert_eq!(graph.nodes.len(), 1);
    }

    #[test]
    fn record_edge_captures_real_counts_and_timestamps() {
        let mut graph = LineageGraph::new();
        let from = graph.upsert_node(source());
        let to = graph.upsert_node(destination());

        let started = Utc::now();
        let completed = started + Duration::milliseconds(250);
        graph.record_edge("run-1", &from, &to, 42, started, completed);

        assert_eq!(graph.edges.len(), 1);
        let edge = &graph.edges[0];
        assert_eq!(edge.record_count, 42);
        assert_eq!(edge.duration_ms(), 250);
    }

    #[test]
    fn edges_for_run_filters_by_run_id() {
        let mut graph = LineageGraph::new();
        let from = graph.upsert_node(source());
        let to = graph.upsert_node(destination());
        let now = Utc::now();

        graph.record_edge("run-1", &from, &to, 10, now, now);
        graph.record_edge("run-2", &from, &to, 20, now, now);
        graph.record_edge("run-1", &from, &to, 5, now, now);

        let run_1_edges = graph.edges_for_run("run-1");
        assert_eq!(run_1_edges.len(), 2);
        assert_eq!(run_1_edges.iter().map(|e| e.record_count).sum::<u64>(), 15);
    }

    #[test]
    fn upstream_and_downstream_queries_are_real_graph_traversal() {
        let mut graph = LineageGraph::new();
        let from = graph.upsert_node(source());
        let to = graph.upsert_node(destination());
        let now = Utc::now();
        graph.record_edge("run-1", &from, &to, 10, now, now);

        let upstream = graph.upstream_of(&to);
        assert_eq!(upstream.len(), 1);
        assert_eq!(upstream[0].id, from);

        let downstream = graph.downstream_of(&from);
        assert_eq!(downstream.len(), 1);
        assert_eq!(downstream[0].id, to);
    }

    #[test]
    fn total_records_received_sums_all_inbound_edges() {
        let mut graph = LineageGraph::new();
        let from = graph.upsert_node(source());
        let to = graph.upsert_node(destination());
        let now = Utc::now();
        graph.record_edge("run-1", &from, &to, 10, now, now);
        graph.record_edge("run-2", &from, &to, 15, now, now);

        assert_eq!(graph.total_records_received(&to), 25);
    }

    #[test]
    fn json_export_round_trips() {
        let mut graph = LineageGraph::new();
        let from = graph.upsert_node(source());
        let to = graph.upsert_node(destination());
        let now = Utc::now();
        graph.record_edge("run-1", &from, &to, 7, now, now);

        let json = graph.to_json();
        let restored: LineageGraph = serde_json::from_value(json).unwrap();
        assert_eq!(restored.nodes.len(), 2);
        assert_eq!(restored.edges.len(), 1);
        assert_eq!(restored.edges[0].record_count, 7);
    }

    #[test]
    fn dot_export_contains_nodes_and_edges() {
        let mut graph = LineageGraph::new();
        let from = graph.upsert_node(source());
        let to = graph.upsert_node(destination());
        let now = Utc::now();
        graph.record_edge("run-1", &from, &to, 3, now, now);

        let dot = graph.to_dot();
        assert!(dot.starts_with("digraph lineage {"));
        assert!(dot.contains("crm.customers"));
        assert!(dot.contains("3 rows"));
    }

    #[test]
    fn lineage_store_is_shareable_and_accumulates_across_calls() {
        let store = LineageStore::new();
        let now = Utc::now();
        store.record_sync("run-1", source(), destination(), 100, now, now);
        store.record_sync("run-2", source(), destination(), 50, now, now);

        let snapshot = store.snapshot();
        assert_eq!(
            snapshot.nodes.len(),
            2,
            "source+destination registered once each"
        );
        assert_eq!(snapshot.edges.len(), 2, "one edge per sync run");
        assert_eq!(store.edges_for_run("run-1").len(), 1);
        assert_eq!(store.edges_for_run("run-1")[0].record_count, 100);
    }

    #[test]
    fn lineage_store_clones_share_the_same_underlying_graph() {
        let store = LineageStore::new();
        let clone = store.clone();
        let now = Utc::now();
        store.record_sync("run-1", source(), destination(), 1, now, now);

        assert_eq!(clone.snapshot().edges.len(), 1, "clone sees the same graph");
    }
}
