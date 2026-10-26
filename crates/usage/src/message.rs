use chrono::{DateTime, FixedOffset};
use eink::epdc::Refresh;
use serde::Deserialize;

use crate::app::{BeadFeed, Feed};
use crate::bead::{Payload, Wire};
use crate::record::Record;

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Push {
    Usage {
        now: String,
        flash: bool,
        record: Record,
    },
    Bead {
        now: String,
        payload: Payload,
    },
}

#[derive(Debug, PartialEq)]
pub enum Message {
    Usage(Feed, Refresh),
    Bead(BeadFeed),
}

pub fn encode_usage(now: DateTime<FixedOffset>, flash: bool, record: &str) -> String {
    format!(
        r#"{{"kind":"usage","now":"{}","flash":{flash},"record":{record}}}"#,
        now.to_rfc3339()
    )
}

pub fn encode_bead(now: DateTime<FixedOffset>, payload: &str) -> String {
    format!(
        r#"{{"kind":"bead","now":"{}","payload":{payload}}}"#,
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
        Push::Bead { now, payload } => Ok(Message::Bead(BeadFeed {
            bead: payload.bead.map(Wire::bead).transpose()?,
            epic: payload.epic,
            now: time(&now)?,
        })),
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
            Message::Bead(_) => panic!("expected a usage push"),
        }
    }

    fn payload(bead: &str) -> String {
        format!(r#"{{"bead":{bead},"epic":null}}"#)
    }

    fn bead(message: Message) -> BeadFeed {
        match message {
            Message::Bead(feed) => feed,
            Message::Usage(..) => panic!("expected a bead push"),
        }
    }

    const SNAPSHOT: &str = r###"{"project":"kindle","id":"kindle-bxx.4","title":"bead push","priority":1,"issue_type":"task","assignee":"elendal","started":"2026-10-26T09:00:00-04:00","description":"## What\n\nText."}"###;

    #[test]
    fn a_bead_push_round_trips_every_field() {
        let feed: BeadFeed = bead(
            decode(&encode_bead(
                at("2026-10-26T09:30:00-04:00"),
                &payload(SNAPSHOT),
            ))
            .unwrap(),
        );
        let got: crate::bead::Bead = feed.bead.unwrap();
        assert_eq!(
            (
                feed.now,
                got.project.as_str(),
                got.id.as_str(),
                got.title.as_str(),
                got.priority,
                got.issue_type.as_str(),
                got.assignee.as_deref(),
                got.started,
                got.description.as_str()
            ),
            (
                at("2026-10-26T09:30:00-04:00"),
                "kindle",
                "kindle-bxx.4",
                "bead push",
                1,
                "task",
                Some("elendal"),
                at("2026-10-26T09:00:00-04:00"),
                "## What\n\nText."
            )
        );
    }

    #[test]
    fn what_the_watcher_sends_decodes_to_the_bead_and_epic_it_describes() {
        let wire: &str = include_str!("../../watch/src/snapshot/wire.json").trim_end();
        let feed: BeadFeed =
            bead(decode(&encode_bead(at("2026-10-26T09:40:00-04:00"), wire)).unwrap());
        let got: crate::bead::Bead = feed.bead.unwrap();
        let epic: crate::bead::Epic = feed.epic.unwrap();
        assert_eq!(
            (
                got.project.as_str(),
                got.id.as_str(),
                got.started,
                got.description.as_str(),
                epic.id.as_str(),
                epic.beads
                    .iter()
                    .map(|m: &crate::bead::Member| (m.id.as_str(), m.status))
                    .collect::<Vec<_>>(),
                epic.blocks
            ),
            (
                "kindle",
                "kindle-bxx.6",
                at("2026-10-26T09:35:01-04:00"),
                "## What\n\nThe PC pushes the active bead.",
                "kindle-bxx",
                vec![
                    ("kindle-bxx.6", crate::bead::Status::InProgress),
                    ("kindle-bxx.5", crate::bead::Status::Open)
                ],
                vec![[String::from("kindle-bxx.6"), String::from("kindle-bxx.5")]]
            )
        );
    }

    #[test]
    fn a_status_the_kindle_does_not_know_is_other_and_not_a_refusal() {
        let epic: &str = r#"{"bead":null,"epic":{"id":"e","title":"t","beads":[{"id":"e.1","title":"a","status":"tombstone"}],"blocks":[]}}"#;
        let feed: BeadFeed =
            bead(decode(&encode_bead(at("2026-10-26T09:40:00-04:00"), epic)).unwrap());
        assert_eq!(
            feed.epic.map(|e: crate::bead::Epic| e.beads[0].status),
            Some(crate::bead::Status::Other)
        );
    }

    #[test]
    fn a_bead_push_of_null_says_nothing_is_in_progress() {
        let feed: BeadFeed = bead(
            decode(&encode_bead(
                at("2026-10-26T09:30:00-04:00"),
                &payload("null"),
            ))
            .unwrap(),
        );
        assert_eq!(feed.bead, None);
    }

    #[test]
    fn a_bead_with_a_bad_start_or_a_missing_field_is_refused() {
        let bad_start: String = SNAPSHOT.replace("2026-10-26T09:00:00-04:00", "this morning");
        let no_title: String = SNAPSHOT.replace(r#""title":"bead push","#, "");
        let refused: [Result<Message, String>; 2] = [
            decode(&encode_bead(
                at("2026-10-26T09:30:00-04:00"),
                &payload(&bad_start),
            )),
            decode(&encode_bead(
                at("2026-10-26T09:30:00-04:00"),
                &payload(&no_title),
            )),
        ];
        assert!(
            refused[0]
                .as_ref()
                .is_err_and(|e: &String| e.starts_with("started this morning"))
                && refused[1]
                    .as_ref()
                    .is_err_and(|e: &String| e.starts_with("push:"))
        );
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
