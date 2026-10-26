use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Deserialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Record {
    pub name: String,
    pub tier_label: String,
    pub usage_status_text: String,
    pub limits: Vec<Limit>,
    pub recent_days: Vec<Day>,
    pub model_usage: BTreeMap<String, Bucket>,
    pub today_prompts: u64,
    pub today_sessions: u64,
    pub updated_at: Option<String>,
}

#[derive(Deserialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Limit {
    pub label: String,
    pub title: Option<String>,
    pub percent: f64,
    pub resets_at: Option<String>,
}

#[derive(Deserialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Day {
    pub date: String,
    pub message_count: u64,
}

#[derive(Deserialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Bucket {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
}

impl Bucket {
    pub fn total(&self) -> u64 {
        self.input_tokens
            + self.output_tokens
            + self.cache_read_input_tokens
            + self.cache_creation_input_tokens
    }
}

pub fn parse(text: &str) -> Result<Record, serde_json::Error> {
    serde_json::from_str(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_object_is_an_empty_record() {
        assert_eq!(parse("{}").unwrap(), Record::default());
    }

    #[test]
    fn unknown_fields_are_ignored() {
        assert_eq!(
            parse(r#"{"schemaVersion":1,"activeDays":18}"#).unwrap(),
            Record::default()
        );
    }

    #[test]
    fn a_record_reads_its_camel_case_fields() {
        let record: Record = parse(
            r#"{"tierLabel":"Max 5x","todayPrompts":2521,"todaySessions":51,
                "limits":[{"label":"Session (5-hour)","percent":0.03,"resetsAt":"2026-10-26T04:10:00+00:00"}],
                "recentDays":[{"date":"2026-10-25","messageCount":7}],
                "modelUsage":{"claude-opus-5-5":{"inputTokens":1,"outputTokens":2,"cacheReadInputTokens":3,"cacheCreationInputTokens":4}}}"#,
        )
        .unwrap();
        let limit: &Limit = &record.limits[0];
        assert_eq!(
            (
                record.tier_label.as_str(),
                record.today_prompts,
                record.today_sessions,
                limit.percent,
                record.recent_days[0].message_count
            ),
            ("Max 5x", 2521, 51, 0.03, 7)
        );
        assert_eq!(record.model_usage["claude-opus-5-5"].total(), 10);
    }

    #[test]
    fn malformed_json_or_a_bare_number_is_refused() {
        assert!(parse("{\"limits\":").is_err() && parse("42").is_err());
    }
}
