use chrono::{DateTime, FixedOffset};
use eink::epdc::Refresh;
use serde::Deserialize;

use crate::app::Feed;
use crate::record::Record;

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Push {
    Usage {
        now: String,
        flash: bool,
        record: Record,
    },
}

#[derive(Debug, PartialEq)]
pub enum Message {
    Usage(Feed, Refresh),
}

pub fn encode_usage(now: DateTime<FixedOffset>, flash: bool, record: &str) -> String {
    format!(
        r#"{{"kind":"usage","now":"{}","flash":{flash},"record":{record}}}"#,
        now.to_rfc3339()
    )
}

pub fn decode(text: &str) -> Result<Message, String> {
    let push: Push =
        serde_json::from_str(text).map_err(|e: serde_json::Error| format!("push: {e}"))?;
    match push {
        Push::Usage { now, flash, record } => {
            let refresh: Refresh = if flash {
                Refresh::Flash
            } else {
                Refresh::Quiet
            };
            Ok(Message::Usage(
                Feed {
                    record,
                    now: time(&now)?,
                },
                refresh,
            ))
        }
    }
}

fn time(text: &str) -> Result<DateTime<FixedOffset>, String> {
    DateTime::parse_from_rfc3339(text).map_err(|e: chrono::ParseError| format!("now {text}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(text).unwrap()
    }

    fn usage(message: Message) -> (Feed, Refresh) {
        match message {
            Message::Usage(feed, refresh) => (feed, refresh),
        }
    }

    #[test]
    fn what_push_encodes_serve_decodes() {
        let (feed, refresh): (Feed, Refresh) = usage(
            decode(&encode_usage(
                at("2026-10-25T20:00:00-04:00"),
                true,
                r#"{"tierLabel":"Max 5x"}"#,
            ))
            .unwrap(),
        );
        assert_eq!(
            (feed.record.tier_label.as_str(), feed.now, refresh),
            ("Max 5x", at("2026-10-25T20:00:00-04:00"), Refresh::Flash)
        );
    }

    #[test]
    fn a_push_without_flash_refreshes_quietly() {
        let (_, refresh): (Feed, Refresh) =
            usage(decode(&encode_usage(at("2026-10-25T20:00:00-04:00"), false, "{}")).unwrap());
        assert_eq!(refresh, Refresh::Quiet);
    }

    #[test]
    fn a_push_with_a_bad_time_or_no_record_is_refused() {
        let bad_time: Result<Message, String> =
            decode(r#"{"kind":"usage","now":"soon","flash":false,"record":{}}"#);
        let no_record: Result<Message, String> =
            decode(r#"{"kind":"usage","now":"2026-10-25T20:00:00-04:00","flash":false}"#);
        assert!(
            bad_time.is_err_and(|e: String| e.starts_with("now soon"))
                && no_record.is_err_and(|e: String| e.starts_with("push:"))
        );
    }

    #[test]
    fn a_push_without_a_kind_or_with_an_unknown_kind_is_refused() {
        let untagged: Result<Message, String> =
            decode(r#"{"now":"2026-10-25T20:00:00-04:00","flash":false,"record":{}}"#);
        let unknown: Result<Message, String> = decode(
            r#"{"kind":"weather","now":"2026-10-25T20:00:00-04:00","flash":false,"record":{}}"#,
        );
        assert!(
            untagged.is_err_and(|e: String| e.starts_with("push:"))
                && unknown.is_err_and(|e: String| e.starts_with("push:"))
        );
    }
}
