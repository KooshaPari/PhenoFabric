//! Research record management and knowledge accumulation for Phenotype Fabric.
//!
//! The [`ResearchLedger`] stores experiment records, findings, and
//! knowledge across the fabric topology. It supports category/status
//! filtering, full-text search, confidence tracking, and aggregate stats.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A research category.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResearchCategory {
    Topology,
    Routing,
    Streaming,
    Security,
    Performance,
    Integration,
}

impl ResearchCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Topology => "topology",
            Self::Routing => "routing",
            Self::Streaming => "streaming",
            Self::Security => "security",
            Self::Performance => "performance",
            Self::Integration => "integration",
        }
    }
}

/// Lifecycle status of a research record.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResearchStatus {
    Planned,
    InProgress,
    Completed,
    Archived,
}

impl ResearchStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Archived => "archived",
        }
    }
}

/// A single evidence-backed finding within a research record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub summary: String,
    pub evidence: Vec<String>,
    /// Confidence in this finding, from 0.0 (speculative) to 1.0 (proven).
    pub confidence: f64,
    pub tags: Vec<String>,
}

/// A research record tracked in the ledger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResearchRecord {
    pub id: String,
    pub title: String,
    pub category: ResearchCategory,
    pub status: ResearchStatus,
    pub findings: Vec<Finding>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Aggregate statistics about the ledger contents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedgerStats {
    pub total_records: usize,
    pub by_status: HashMap<String, usize>,
    pub by_category: HashMap<String, usize>,
    pub total_findings: usize,
    pub avg_confidence: f64,
}

// ---------------------------------------------------------------------------
// Ledger
// ---------------------------------------------------------------------------

/// The research ledger — stores and queries research records.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResearchLedger {
    records: Vec<ResearchRecord>,
}

impl ResearchLedger {
    /// Create an empty ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a new record and return a reference to it.
    pub fn add_record(&mut self, record: ResearchRecord) -> &ResearchRecord {
        self.records.push(record);
        self.records.last().unwrap()
    }

    /// Look up a record by id.
    pub fn get_record(&self, id: &str) -> Option<&ResearchRecord> {
        self.records.iter().find(|r| r.id == id)
    }

    /// Return all records.
    pub fn list_records(&self) -> &[ResearchRecord] {
        &self.records
    }

    /// Filter records by category.
    pub fn list_by_category(&self, category: &ResearchCategory) -> Vec<&ResearchRecord> {
        self.records
            .iter()
            .filter(|r| &r.category == category)
            .collect()
    }

    /// Filter records by status.
    pub fn list_by_status(&self, status: &ResearchStatus) -> Vec<&ResearchRecord> {
        self.records
            .iter()
            .filter(|r| &r.status == status)
            .collect()
    }

    /// Update the status of a record. Returns `true` if found.
    pub fn update_status(&mut self, id: &str, status: ResearchStatus) -> bool {
        if let Some(r) = self.records.iter_mut().find(|r| r.id == id) {
            r.status = status;
            true
        } else {
            false
        }
    }

    /// Append a finding to a record. Returns `true` if the record was found.
    pub fn add_finding(&mut self, record_id: &str, finding: Finding) -> bool {
        if let Some(r) = self.records.iter_mut().find(|r| r.id == record_id) {
            r.findings.push(finding);
            true
        } else {
            false
        }
    }

    /// Case-insensitive search across title, finding summaries, and tags.
    pub fn search(&self, query: &str) -> Vec<&ResearchRecord> {
        let q = query.to_lowercase();
        self.records
            .iter()
            .filter(|r| {
                r.title.to_lowercase().contains(&q)
                    || r.findings
                        .iter()
                        .any(|f| f.summary.to_lowercase().contains(&q))
                    || r.findings
                        .iter()
                        .any(|f| f.tags.iter().any(|t| t.to_lowercase().contains(&q)))
            })
            .collect()
    }

    /// Compute aggregate statistics.
    pub fn stats(&self) -> LedgerStats {
        let mut by_status = HashMap::new();
        let mut by_category = HashMap::new();
        let mut total_findings = 0usize;
        let mut confidence_sum = 0f64;
        let mut confidence_count = 0usize;

        for r in &self.records {
            *by_status.entry(r.status.as_str().to_string()).or_insert(0) += 1;
            *by_category
                .entry(r.category.as_str().to_string())
                .or_insert(0) += 1;
            total_findings += r.findings.len();
            for f in &r.findings {
                confidence_sum += f.confidence;
                confidence_count += 1;
            }
        }

        LedgerStats {
            total_records: self.records.len(),
            by_status,
            by_category,
            total_findings,
            avg_confidence: if confidence_count > 0 {
                confidence_sum / confidence_count as f64
            } else {
                0.0
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_record(id: &str) -> ResearchRecord {
        ResearchRecord {
            id: id.to_string(),
            title: format!("Record {id}"),
            category: ResearchCategory::Topology,
            status: ResearchStatus::Planned,
            findings: vec![],
            created_at: 1000,
            updated_at: 1000,
        }
    }

    #[test]
    fn test_create_ledger() {
        let ledger = ResearchLedger::new();
        assert!(ledger.list_records().is_empty());
    }

    #[test]
    fn test_add_and_get_record() {
        let mut ledger = ResearchLedger::new();
        let record = sample_record("r1");
        ledger.add_record(record);
        assert_eq!(ledger.list_records().len(), 1);
        assert_eq!(ledger.get_record("r1").unwrap().title, "Record r1");
        assert!(ledger.get_record("nope").is_none());
    }

    #[test]
    fn test_list_by_category() {
        let mut ledger = ResearchLedger::new();
        ledger.add_record(sample_record("r1"));
        let mut r2 = sample_record("r2");
        r2.category = ResearchCategory::Streaming;
        ledger.add_record(r2);

        assert_eq!(
            ledger.list_by_category(&ResearchCategory::Topology).len(),
            1
        );
        assert_eq!(
            ledger.list_by_category(&ResearchCategory::Streaming).len(),
            1
        );
    }

    #[test]
    fn test_update_status() {
        let mut ledger = ResearchLedger::new();
        ledger.add_record(sample_record("r1"));
        assert!(ledger.update_status("r1", ResearchStatus::Completed));
        assert_eq!(
            ledger.get_record("r1").unwrap().status,
            ResearchStatus::Completed
        );
        assert!(!ledger.update_status("nope", ResearchStatus::Archived));
    }

    #[test]
    fn test_add_finding() {
        let mut ledger = ResearchLedger::new();
        ledger.add_record(sample_record("r1"));
        let finding = Finding {
            id: "f1".into(),
            summary: "Key insight".into(),
            evidence: vec!["test data".into()],
            confidence: 0.85,
            tags: vec!["performance".into()],
        };
        assert!(ledger.add_finding("r1", finding));
        assert_eq!(ledger.get_record("r1").unwrap().findings.len(), 1);
        assert!(!ledger.add_finding(
            "nope",
            Finding {
                id: "f2".into(),
                summary: "orphan".into(),
                evidence: vec![],
                confidence: 0.5,
                tags: vec![],
            }
        ));
    }

    #[test]
    fn test_search() {
        let mut ledger = ResearchLedger::new();
        let mut r = sample_record("r1");
        r.title = "Latency analysis".into();
        r.findings.push(Finding {
            id: "f1".into(),
            summary: "GPU pipelines reduce latency".into(),
            evidence: vec![],
            confidence: 0.9,
            tags: vec!["gpu".into()],
        });
        ledger.add_record(r);

        assert_eq!(ledger.search("latency").len(), 1);
        assert_eq!(ledger.search("GPU").len(), 1);
        assert_eq!(ledger.search("nonexistent").len(), 0);
    }

    #[test]
    fn test_stats() {
        let mut ledger = ResearchLedger::new();
        ledger.add_record(sample_record("r1"));
        let mut r2 = sample_record("r2");
        r2.status = ResearchStatus::Completed;
        r2.category = ResearchCategory::Security;
        r2.findings.push(Finding {
            id: "f1".into(),
            summary: "Found issue".into(),
            evidence: vec![],
            confidence: 0.7,
            tags: vec![],
        });
        ledger.add_record(r2);

        let stats = ledger.stats();
        assert_eq!(stats.total_records, 2);
        assert_eq!(stats.by_status.get("planned"), Some(&1));
        assert_eq!(stats.by_status.get("completed"), Some(&1));
        assert_eq!(stats.total_findings, 1);
        assert!((stats.avg_confidence - 0.7).abs() < f64::EPSILON);
    }
}
