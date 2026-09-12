//! Topology persistence — save/load entire topologies to/from SQLite.

use chrono::{DateTime, Utc};
use fabric_graph::model::{
    CapabilityRef, Edge, EdgeId, LinkMetrics, Node, NodeId, Topology, TopologyEpoch,
    TopologyMeta,
};
use fabric_graph::LocalityTier;
use rusqlite::params;

use crate::error::PersistError;
use crate::Persist;

impl Persist {
    /// Save a full topology (nodes + edges + meta) in a single transaction.
    ///
    /// This replaces any existing topology data. It is intended for
    /// save-the-world snapshots, not incremental updates.
    pub fn save_topology(&self, topo: &Topology) -> Result<(), PersistError> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;

            // Clear existing topology data.
            tx.execute_batch(
                "DELETE FROM topology_edges;
                 DELETE FROM topology_nodes;
                 DELETE FROM topology_meta;",
            )?;

            // Save epoch.
            tx.execute(
                "INSERT INTO topology_meta (key, value) VALUES ('epoch', ?1)",
                params![topo.epoch.0.to_string()],
            )?;

            // Save meta.
            tx.execute(
                "INSERT INTO topology_meta (key, value) VALUES ('name', ?1)",
                params![topo.meta.name],
            )?;
            if let Some(ref by) = topo.meta.created_by {
                tx.execute(
                    "INSERT INTO topology_meta (key, value) VALUES ('created_by', ?1)",
                    params![by],
                )?;
            }
            if let Some(ref ts) = topo.meta.created_at {
                tx.execute(
                    "INSERT INTO topology_meta (key, value) VALUES ('created_at', ?1)",
                    params![ts.to_rfc3339()],
                )?;
            }
            // Serialize annotations.
            if !topo.meta.annotations.is_empty() {
                let json = serde_json::to_string(&topo.meta.annotations)?;
                tx.execute(
                    "INSERT INTO topology_meta (key, value) VALUES ('annotations', ?1)",
                    params![json],
                )?;
            }

            // Save nodes.
            for node in topo.nodes.values() {
                let tags_json = serde_json::to_string(&node.tags)?;
                let caps_json = serde_json::to_string(&node.capabilities)?;
                tx.execute(
                    "INSERT INTO topology_nodes (id, label, locality_tier, capabilities, tags, last_seen)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        node.id.0,
                        node.label,
                        tier_to_i32(node.locality_tier),
                        caps_json,
                        tags_json,
                        node.last_seen.to_rfc3339(),
                    ],
                )?;
            }

            // Save edges.
            for edge in topo.edges.values() {
                let metrics_json = serde_json::to_string(&edge.metrics)?;
                tx.execute(
                    "INSERT INTO topology_edges (id, from_node, to_node, locality_tier, metrics, up)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        edge.id.0,
                        edge.from.0,
                        edge.to.0,
                        tier_to_i32(edge.locality_tier),
                        metrics_json,
                        edge.up,
                    ],
                )?;
            }

            tx.commit()?;
            Ok(())
        })
    }

    /// Load a topology from the database.
    ///
    /// Returns `Ok(None)` if the database is empty (no topology saved yet).
    pub fn load_topology(&self) -> Result<Option<Topology>, PersistError> {
        self.with_conn(|conn| {
            // Check if we have any meta (epoch is always present).
            let count: i64 =
                conn.query_row("SELECT COUNT(*) FROM topology_meta", [], |r| r.get(0))?;
            if count == 0 {
                return Ok(None);
            }

            // Load meta.
            let mut meta = TopologyMeta::default();
            let mut epoch = TopologyEpoch::default();

            let mut stmt = conn.prepare("SELECT key, value FROM topology_meta")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (k, v) = row?;
                match k.as_str() {
                    "epoch" => {
                        if let Ok(e) = v.parse::<u64>() {
                            epoch = TopologyEpoch(e);
                        }
                    }
                    "name" => meta.name = v,
                    "created_by" => meta.created_by = Some(v),
                    "created_at" => {
                        meta.created_at = DateTime::parse_from_rfc3339(&v)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc));
                    }
                    "annotations" => {
                        if let Ok(a) = serde_json::from_str(&v) {
                            meta.annotations = a;
                        }
                    }
                    _ => {}
                }
            }

            // Load nodes.
            let mut stmt = conn.prepare(
                "SELECT id, label, locality_tier, capabilities, tags, last_seen FROM topology_nodes",
            )?;
            let nodes: Vec<Node> = stmt
                .query_map([], |row| {
                    let id: String = row.get(0)?;
                    let label: Option<String> = row.get(1)?;
                    let tier_i32: i32 = row.get(2)?;
                    let caps_json: String = row.get(3)?;
                    let tags_json: String = row.get(4)?;
                    let last_seen_str: String = row.get(5)?;

                    let tier = i32_to_tier(tier_i32).unwrap_or(LocalityTier::L7Wan);
                    let capabilities: Vec<CapabilityRef> =
                        serde_json::from_str(&caps_json).unwrap_or_default();
                    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
                    let last_seen = DateTime::parse_from_rfc3339(&last_seen_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());

                    Ok(Node {
                        id: NodeId(id),
                        label,
                        locality_tier: tier,
                        capabilities,
                        last_seen,
                        tags,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            // Load edges.
            let mut stmt = conn.prepare(
                "SELECT id, from_node, to_node, locality_tier, metrics, up FROM topology_edges",
            )?;
            let edges: Vec<Edge> = stmt
                .query_map([], |row| {
                    let id: String = row.get(0)?;
                    let from: String = row.get(1)?;
                    let to: String = row.get(2)?;
                    let tier_i32: i32 = row.get(3)?;
                    let metrics_json: String = row.get(4)?;
                    let up: bool = row.get(5)?;

                    let tier = i32_to_tier(tier_i32).unwrap_or(LocalityTier::L7Wan);
                    let metrics: Option<LinkMetrics> =
                        serde_json::from_str(&metrics_json).ok().flatten();

                    Ok(Edge {
                        id: EdgeId(id),
                        from: NodeId(from),
                        to: NodeId(to),
                        locality_tier: tier,
                        metrics,
                        up,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            // Build topology.
            let mut topo = Topology::new();
            topo.epoch = epoch;
            topo.meta = meta;
            for node in nodes {
                topo.nodes.insert(node.id.clone(), node);
            }
            for edge in edges {
                topo.edges.insert(edge.id.clone(), edge);
            }

            Ok(Some(topo))
        })
    }

    /// Increment the topology epoch.
    pub fn advance_epoch(&self) -> Result<TopologyEpoch, PersistError> {
        self.with_conn(|conn| {
            // Read current epoch value.
            let current: String = conn
                .query_row(
                    "SELECT value FROM topology_meta WHERE key = 'epoch'",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or_else(|_| "0".to_string());
            let current_epoch: u64 = current.parse().unwrap_or(0);
            let new_epoch = current_epoch + 1;

            // Update.
            conn.execute(
                "DELETE FROM topology_meta WHERE key = 'epoch'",
                [],
            )?;
            conn.execute(
                "INSERT INTO topology_meta (key, value) VALUES ('epoch', ?1)",
                params![new_epoch.to_string()],
            )?;
            Ok(TopologyEpoch(new_epoch))
        })
    }
}

/// Convert LocalityTier to i32 for SQLite storage.
fn tier_to_i32(tier: LocalityTier) -> i32 {
    tier.as_f64() as i32
}

/// Convert i32 back to LocalityTier.
fn i32_to_tier(i: i32) -> Option<LocalityTier> {
    LocalityTier::from_index(i as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fabric_graph::model::{Edge, Node};

    fn test_topology() -> Topology {
        let mut topo = Topology::new();
        topo.meta.name = "test-topo".into();
        let n1 = Node::new(NodeId("n1".into()), LocalityTier::L2CrossNumaShm)
            .with_label("node-1")
            .with_tag("gpu");
        let n2 = Node::new(NodeId("n2".into()), LocalityTier::L6Lan).with_label("node-2");
        topo.nodes.insert(n1.id.clone(), n1);
        topo.nodes.insert(n2.id.clone(), n2);
        topo.edges.insert(
            EdgeId("e1".into()),
            Edge::new(
                EdgeId("e1".into()),
                NodeId("n1".into()),
                NodeId("n2".into()),
                LocalityTier::L6Lan,
            ),
        );
        topo
    }

    #[test]
    fn save_and_load_topology_roundtrip() {
        let persist = Persist::open_memory().unwrap();
        let topo = test_topology();

        persist.save_topology(&topo).unwrap();
        let loaded = persist.load_topology().unwrap().unwrap();

        assert_eq!(loaded.nodes.len(), 2);
        assert_eq!(loaded.edges.len(), 1);
        assert_eq!(loaded.meta.name, "test-topo");
    }

    #[test]
    fn load_empty_returns_none() {
        let persist = Persist::open_memory().unwrap();
        assert!(persist.load_topology().unwrap().is_none());
    }

    #[test]
    fn save_replaces_existing_topology() {
        let persist = Persist::open_memory().unwrap();
        let topo1 = test_topology();
        persist.save_topology(&topo1).unwrap();

        let mut topo2 = Topology::new();
        topo2.meta.name = "replaced".into();
        let n = Node::new(NodeId("only".into()), LocalityTier::L0SameProcess);
        topo2.nodes.insert(n.id.clone(), n);
        persist.save_topology(&topo2).unwrap();

        let loaded = persist.load_topology().unwrap().unwrap();
        assert_eq!(loaded.nodes.len(), 1);
        assert_eq!(loaded.meta.name, "replaced");
    }

    #[test]
    fn advance_epoch_increments() {
        let persist = Persist::open_memory().unwrap();
        let e1 = persist.advance_epoch().unwrap();
        let e2 = persist.advance_epoch().unwrap();
        assert_eq!(e1.0 + 1, e2.0);
    }

    #[test]
    fn topology_meta_roundtrip() {
        use chrono::Utc;

        let persist = Persist::open_memory().unwrap();
        let mut topo = Topology::new();
        topo.meta.name = "full-meta-test".into();
        topo.meta.created_by = Some("test-agent".into());
        topo.meta.created_at = Some(Utc::now());
        topo.meta
            .annotations
            .insert("env".into(), "staging".into());
        persist.save_topology(&topo).unwrap();

        let loaded = persist.load_topology().unwrap().unwrap();
        assert_eq!(loaded.meta.name, "full-meta-test");
        assert_eq!(
            loaded.meta.created_by.as_deref(),
            Some("test-agent")
        );
        assert!(loaded.meta.created_at.is_some());
        assert_eq!(
            loaded.meta.annotations.get("env").map(|s| s.as_str()),
            Some("staging")
        );
    }
}
