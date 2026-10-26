use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use crate::epic::Epic;

#[derive(Deserialize)]
struct Listed {
    issues: Vec<Issue>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Issue {
    pub id: String,
    pub title: String,
    pub priority: u8,
    pub issue_type: String,
    pub assignee: Option<String>,
}

#[derive(Deserialize)]
struct Audit {
    events: Vec<AuditEvent>,
}

#[derive(Deserialize)]
struct AuditEvent {
    id: i64,
    event_type: String,
    actor: String,
    new_value: Option<String>,
    timestamp: String,
}

#[derive(Deserialize)]
struct Configured {
    value: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub project: String,
    pub issue: Issue,
    pub started: DateTime<FixedOffset>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Snapshot {
    pub project: String,
    pub id: String,
    pub title: String,
    pub priority: u8,
    pub issue_type: String,
    pub assignee: Option<String>,
    pub started: String,
    pub description: String,
}

pub fn in_progress(list: &str) -> Result<Vec<Issue>, String> {
    serde_json::from_str::<Listed>(list)
        .map(|listed: Listed| listed.issues)
        .map_err(|e: serde_json::Error| format!("br list: {e}"))
}

pub fn actor(config: &str, user: Option<&str>) -> Result<String, String> {
    let configured: Configured = serde_json::from_str(config)
        .map_err(|e: serde_json::Error| format!("br config get actor: {e}"))?;
    let named = |name: &str| -> Option<String> {
        Some(name.trim().to_owned()).filter(|name: &String| !name.is_empty())
    };
    Ok(configured
        .value
        .as_deref()
        .and_then(named)
        .or_else(|| user.and_then(named))
        .unwrap_or_else(|| String::from("unknown")))
}

pub fn started(audit: &str, me: &str) -> Result<Option<DateTime<FixedOffset>>, String> {
    let audit: Audit =
        serde_json::from_str(audit).map_err(|e: serde_json::Error| format!("br audit log: {e}"))?;
    let latest = |kind: &str| -> Option<&AuditEvent> {
        audit
            .events
            .iter()
            .filter(|e: &&AuditEvent| e.event_type == kind)
            .max_by_key(|e: &&AuditEvent| e.id)
    };
    let moved: Option<&AuditEvent> = match latest("status_changed") {
        Some(change) => {
            Some(change).filter(|e: &&AuditEvent| e.new_value.as_deref() == Some("in_progress"))
        }
        None => latest("created"),
    };
    moved
        .filter(|e: &&AuditEvent| e.actor == me)
        .map(|e: &AuditEvent| {
            DateTime::parse_from_rfc3339(&e.timestamp)
                .map_err(|err: chrono::ParseError| format!("{}: {err}", e.timestamp))
        })
        .transpose()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Payload {
    pub bead: Option<Snapshot>,
    pub epic: Option<Epic>,
}

pub fn newest(candidates: &[Candidate]) -> Option<&Candidate> {
    candidates.iter().max_by(|a: &&Candidate, b: &&Candidate| {
        a.started
            .cmp(&b.started)
            .then_with(|| b.issue.id.cmp(&a.issue.id))
    })
}

pub fn snapshot(candidate: &Candidate, description: String) -> Snapshot {
    Snapshot {
        project: candidate.project.clone(),
        id: candidate.issue.id.clone(),
        title: candidate.issue.title.clone(),
        priority: candidate.issue.priority,
        issue_type: candidate.issue.issue_type.clone(),
        assignee: candidate.issue.assignee.clone(),
        started: candidate.started.to_rfc3339(),
        description,
    }
}

pub fn wire(payload: &Payload) -> String {
    serde_json::to_string(payload).unwrap_or_default()
}

#[cfg(test)]
mod tests;
