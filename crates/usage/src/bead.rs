use chrono::{DateTime, FixedOffset};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Wire {
    project: String,
    id: String,
    title: String,
    priority: u8,
    issue_type: String,
    assignee: Option<String>,
    started: String,
    description: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bead {
    pub project: String,
    pub id: String,
    pub title: String,
    pub priority: u8,
    pub issue_type: String,
    pub assignee: Option<String>,
    pub started: DateTime<FixedOffset>,
    pub description: String,
}

impl Wire {
    pub fn bead(self) -> Result<Bead, String> {
        let started: DateTime<FixedOffset> = DateTime::parse_from_rfc3339(&self.started)
            .map_err(|e: chrono::ParseError| format!("started {}: {e}", self.started))?;
        Ok(Bead {
            project: self.project,
            id: self.id,
            title: self.title,
            priority: self.priority,
            issue_type: self.issue_type,
            assignee: self.assignee,
            started,
            description: self.description,
        })
    }
}
