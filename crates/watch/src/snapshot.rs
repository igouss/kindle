use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

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
    pub updated_at: String,
}

#[derive(Deserialize)]
struct Audit {
    events: Vec<AuditEvent>,
}

#[derive(Deserialize)]
struct AuditEvent {
    event_type: String,
    new_value: Option<String>,
    timestamp: String,
}

#[derive(Deserialize)]
struct Shown {
    description: Option<String>,
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

pub fn started(audit: &str, issue: &Issue) -> Result<DateTime<FixedOffset>, String> {
    let audit: Audit =
        serde_json::from_str(audit).map_err(|e: serde_json::Error| format!("br audit log: {e}"))?;
    let moved: Option<&str> = audit
        .events
        .iter()
        .find(|e: &&AuditEvent| {
            e.event_type == "status_changed" && e.new_value.as_deref() == Some("in_progress")
        })
        .map(|e: &AuditEvent| e.timestamp.as_str());
    let text: &str = moved.unwrap_or(&issue.updated_at);
    DateTime::parse_from_rfc3339(text).map_err(|e: chrono::ParseError| format!("{text}: {e}"))
}

pub fn description(show: &str) -> Result<String, String> {
    let shown: Shown = match serde_json::from_str::<Vec<Shown>>(show) {
        Ok(mut list) if !list.is_empty() => list.swap_remove(0),
        Ok(_) | Err(_) => {
            serde_json::from_str(show).map_err(|e: serde_json::Error| format!("br show: {e}"))?
        }
    };
    Ok(shown.description.unwrap_or_default())
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

pub fn wire(snapshot: Option<&Snapshot>) -> String {
    serde_json::to_string(&snapshot).unwrap_or_else(|_| String::from("null"))
}

#[cfg(test)]
mod tests;
