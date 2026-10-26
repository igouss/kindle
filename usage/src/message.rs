use chrono::{DateTime, FixedOffset};
use eink::epdc::Refresh;
use serde::Deserialize;

use crate::app::Feed;
use crate::record::Record;

#[derive(Deserialize)]
struct Push {
    now: String,
    flash: bool,
    record: Record,
}

pub fn encode(now: DateTime<FixedOffset>, flash: bool, record: &str) -> String {
    format!(
        r#"{{"now":"{}","flash":{flash},"record":{record}}}"#,
        now.to_rfc3339()
    )
}

pub fn decode(text: &str) -> Result<(Feed, Refresh), String> {
    let push: Push =
        serde_json::from_str(text).map_err(|e: serde_json::Error| format!("push: {e}"))?;
    let now: DateTime<FixedOffset> = DateTime::parse_from_rfc3339(&push.now)
        .map_err(|e: chrono::ParseError| format!("now {}: {e}", push.now))?;
    let refresh: Refresh = if push.flash {
        Refresh::Flash
    } else {
        Refresh::Quiet
    };
    Ok((
        Feed {
            record: push.record,
            now,
        },
        refresh,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(text).unwrap()
    }

    #[test]
    fn what_push_encodes_serve_decodes() {
        let (feed, refresh): (Feed, Refresh) = decode(&encode(
            at("2026-10-25T20:00:00-04:00"),
            true,
            r#"{"tierLabel":"Max 5x"}"#,
        ))
        .unwrap();
        assert_eq!(
            (feed.record.tier_label.as_str(), feed.now, refresh),
            ("Max 5x", at("2026-10-25T20:00:00-04:00"), Refresh::Flash)
        );
    }

    #[test]
    fn a_push_without_flash_refreshes_quietly() {
        let (_, refresh): (Feed, Refresh) =
            decode(&encode(at("2026-10-25T20:00:00-04:00"), false, "{}")).unwrap();
        assert_eq!(refresh, Refresh::Quiet);
    }

    #[test]
    fn a_push_with_a_bad_time_or_no_record_is_refused() {
        let bad_time: Result<(Feed, Refresh), String> =
            decode(r#"{"now":"soon","flash":false,"record":{}}"#);
        let no_record: Result<(Feed, Refresh), String> =
            decode(r#"{"now":"2026-10-25T20:00:00-04:00","flash":false}"#);
        assert!(
            bad_time.is_err_and(|e: String| e.starts_with("now soon"))
                && no_record.is_err_and(|e: String| e.starts_with("push:"))
        );
    }
}
