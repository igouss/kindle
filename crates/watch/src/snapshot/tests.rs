use super::*;

fn issue(id: &str) -> Issue {
    Issue {
        id: id.to_owned(),
        title: String::from("watcher"),
        priority: 1,
        issue_type: String::from("task"),
        assignee: Some(String::from("elendal")),
        updated_at: String::from("2026-10-26T13:40:00Z"),
    }
}

fn at(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
}

fn candidate(id: &str, started: &str) -> Candidate {
    Candidate {
        project: String::from("kindle"),
        issue: issue(id),
        started: at(started),
    }
}

const LISTED: &str = r#"{"issues":[{"id":"kindle-bxx.6","title":"watcher","status":"in_progress","priority":1,"issue_type":"task","assignee":"elendal","created_at":"2026-10-26T12:00:00Z","updated_at":"2026-10-26T13:40:00Z","source_repo":"kindle"},{"id":"kindle-bxx.5","title":"graph","status":"in_progress","priority":1,"issue_type":"task","assignee":null,"created_at":"2026-10-26T12:00:00Z","updated_at":"2026-10-26T13:41:00Z"}],"total":2,"limit":0,"offset":0,"has_more":false}"#;

const AUDIT: &str = r#"{"issue_id":"kindle-bxx.6","events":[{"id":9,"event_type":"status_changed","actor":"elendal","timestamp":"2026-10-26T13:35:01.796670832Z","old_value":"open","new_value":"in_progress"},{"id":8,"event_type":"status_changed","actor":"elendal","timestamp":"2026-10-26T13:34:59.744668490Z","old_value":"in_progress","new_value":"open"},{"id":7,"event_type":"status_changed","actor":"elendal","timestamp":"2026-10-26T13:34:41.710041692Z","old_value":"open","new_value":"in_progress"},{"id":1,"event_type":"created","actor":"elendal","timestamp":"2026-10-26T12:00:00Z"}]}"#;

#[test]
fn nothing_in_progress_lists_no_issues() {
    assert_eq!(
        in_progress(r#"{"issues":[],"total":0,"limit":0,"offset":0,"has_more":false}"#),
        Ok(vec![])
    );
}

#[test]
fn every_listed_issue_is_read_with_its_assignee_or_none() {
    let issues: Vec<Issue> = in_progress(LISTED).unwrap();
    assert_eq!(
        issues
            .iter()
            .map(|i: &Issue| (i.id.as_str(), i.assignee.as_deref()))
            .collect::<Vec<_>>(),
        [("kindle-bxx.6", Some("elendal")), ("kindle-bxx.5", None)]
    );
}

#[test]
fn a_list_that_is_not_br_json_is_refused() {
    assert!(
        in_progress("Error: no beads workspace").is_err_and(|e: String| e.starts_with("br list:"))
    );
}

#[test]
fn work_started_at_the_latest_move_to_in_progress() {
    assert_eq!(
        started(AUDIT, &issue("kindle-bxx.6")),
        Ok(at("2026-10-26T13:35:01.796670832Z"))
    );
}

#[test]
fn without_a_move_to_in_progress_work_started_when_last_updated() {
    let created_in_progress: &str = r#"{"issue_id":"x","events":[{"id":1,"event_type":"created","actor":"a","timestamp":"2026-10-26T12:00:00Z"},{"id":2,"event_type":"status_changed","actor":"a","timestamp":"2026-10-26T12:30:00Z","old_value":"in_progress","new_value":"open"}]}"#;
    assert_eq!(
        started(created_in_progress, &issue("x")),
        Ok(at("2026-10-26T13:40:00Z"))
    );
}

#[test]
fn an_unreadable_audit_or_time_is_refused() {
    let bad_time: Issue = Issue {
        updated_at: String::from("soon"),
        ..issue("x")
    };
    assert!(
        started("{}", &issue("x")).is_err_and(|e: String| e.starts_with("br audit log:"))
            && started(r#"{"events":[]}"#, &bad_time).is_err_and(|e: String| e.starts_with("soon"))
    );
}

#[test]
fn the_description_comes_from_show_as_a_list_or_one_object_and_null_is_empty() {
    assert_eq!(
        (
            description(r#"[{"id":"x","description":"What"}]"#),
            description(r#"{"id":"x","description":"text"}"#),
            description(r#"[{"id":"x","description":null}]"#)
        ),
        (
            Ok(String::from("What")),
            Ok(String::from("text")),
            Ok(String::new())
        )
    );
}

#[test]
fn an_empty_or_unreadable_show_is_refused() {
    assert!(
        description("[]").is_err_and(|e: String| e.starts_with("br show:"))
            && description("nope").is_err_and(|e: String| e.starts_with("br show:"))
    );
}

#[test]
fn with_no_candidates_nothing_is_newest() {
    assert_eq!(newest(&[]), None);
}

#[test]
fn the_newest_is_the_latest_started_whatever_the_order() {
    let candidates: [Candidate; 3] = [
        candidate("a", "2026-10-26T09:00:00Z"),
        candidate("b", "2026-10-26T11:00:00Z"),
        candidate("c", "2026-10-26T10:00:00Z"),
    ];
    assert_eq!(
        newest(&candidates).map(|c: &Candidate| c.issue.id.as_str()),
        Some("b")
    );
}

#[test]
fn started_at_the_same_moment_the_first_id_wins() {
    let tied: [Candidate; 2] = [
        candidate("b", "2026-10-26T09:00:00Z"),
        candidate("a", "2026-10-26T09:00:00Z"),
    ];
    assert_eq!(
        newest(&tied).map(|c: &Candidate| c.issue.id.as_str()),
        Some("a")
    );
}

#[test]
fn the_wire_snapshot_is_exactly_what_the_kindle_decodes() {
    let chosen: Candidate = Candidate {
        project: String::from("kindle"),
        issue: issue("kindle-bxx.6"),
        started: at("2026-10-26T09:35:01-04:00"),
    };
    let sent: Snapshot = snapshot(
        &chosen,
        String::from("## What\n\nThe PC pushes the active bead."),
    );
    assert_eq!(wire(Some(&sent)), include_str!("wire.json").trim_end());
}

#[test]
fn nothing_in_progress_goes_on_the_wire_as_null() {
    assert_eq!(wire(None), "null");
}
