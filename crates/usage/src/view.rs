use std::time::Duration;

use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, Weekday};

use crate::app::Work;
use crate::bead::Bead;
use crate::record::{Bucket, Day, Limit, Record};

const MODEL_ROWS: usize = 4;

#[derive(Debug, PartialEq)]
pub struct Dashboard {
    pub title: String,
    pub plan: String,
    pub limits: Vec<Meter>,
    pub days: Vec<DayRow>,
    pub today: String,
    pub models: Vec<ModelRow>,
    pub updated: String,
}

#[derive(Debug, PartialEq)]
pub struct Meter {
    pub title: String,
    pub fraction: f64,
    pub resets_in: Option<Duration>,
}

#[derive(Debug, PartialEq)]
pub struct DayRow {
    pub label: String,
    pub tokens: u64,
    pub today: bool,
}

#[derive(Debug, PartialEq)]
pub struct ModelRow {
    pub name: String,
    pub tokens: u64,
}

#[derive(Debug, PartialEq)]
pub enum BeadPage {
    Waiting,
    Idle,
    Active {
        place: String,
        title: String,
        meta: String,
        description: String,
        elapsed: String,
        page: usize,
    },
}

pub fn bead_page(work: &Work, since_push: Duration) -> BeadPage {
    match work {
        Work::Unknown => BeadPage::Waiting,
        Work::Idle { .. } => BeadPage::Idle,
        Work::Active {
            bead, now, page, ..
        } => {
            let at: DateTime<FixedOffset> =
                *now + chrono::TimeDelta::from_std(since_push).unwrap_or(chrono::TimeDelta::MAX);
            active_page(bead, at, *page)
        }
    }
}

pub fn active_page(bead: &Bead, at: DateTime<FixedOffset>, page: usize) -> BeadPage {
    let mut meta: Vec<String> = vec![format!("P{}", bead.priority), bead.issue_type.clone()];
    meta.extend(bead.assignee.clone());
    BeadPage::Active {
        place: format!("{} \u{b7} {}", bead.project, bead.id),
        title: bead.title.clone(),
        meta: meta.join(" \u{b7} "),
        description: bead.description.clone(),
        elapsed: format_duration((at - bead.started).to_std().unwrap_or(Duration::ZERO)),
        page,
    }
}

pub fn waiting() -> Dashboard {
    Dashboard {
        title: String::from("Claude Code"),
        plan: String::from("waiting for devbox"),
        limits: vec![],
        days: vec![],
        today: String::new(),
        models: vec![],
        updated: String::new(),
    }
}

pub fn dashboard(record: &Record, now: DateTime<FixedOffset>) -> Dashboard {
    let mut models: Vec<ModelRow> = record
        .model_usage
        .iter()
        .map(|(id, bucket): (&String, &Bucket)| ModelRow {
            name: friendly_model_name(id),
            tokens: bucket.total(),
        })
        .collect();
    models.sort_by_key(|m: &ModelRow| std::cmp::Reverse(m.tokens));
    models.truncate(MODEL_ROWS);
    Dashboard {
        title: if record.name.is_empty() {
            String::from("Claude Code")
        } else {
            record.name.clone()
        },
        plan: plan(record),
        limits: record
            .limits
            .iter()
            .filter(|l: &&Limit| l.percent >= 0.0)
            .map(|l: &Limit| meter(l, now))
            .collect(),
        days: record
            .recent_days
            .iter()
            .map(|d: &Day| day_row(d, now.date_naive()))
            .collect(),
        today: format!(
            "{} prompts \u{b7} {} sessions",
            record.today_prompts, record.today_sessions
        ),
        models,
        updated: record
            .updated_at
            .as_deref()
            .and_then(|t: &str| DateTime::parse_from_rfc3339(t).ok())
            .map_or(String::from("never updated"), |t: DateTime<FixedOffset>| {
                format!("updated {}", t.with_timezone(now.offset()).format("%H:%M"))
            }),
    }
}

fn plan(record: &Record) -> String {
    if !record.usage_status_text.is_empty() {
        return record.usage_status_text.clone();
    }
    let mut letters: std::str::Chars<'_> = record.tier_label.chars();
    match letters.next() {
        Some(first) => first.to_uppercase().chain(letters).collect(),
        None => String::from("Subscription"),
    }
}

fn meter(limit: &Limit, now: DateTime<FixedOffset>) -> Meter {
    let reset: Option<DateTime<FixedOffset>> = limit
        .resets_at
        .as_deref()
        .and_then(|t: &str| DateTime::parse_from_rfc3339(t).ok());
    let remaining: Option<Duration> =
        reset.and_then(|r: DateTime<FixedOffset>| (r - now).to_std().ok());
    let expired: bool = reset.is_some() && remaining.is_none_or(|d: Duration| d.is_zero());
    Meter {
        title: limit
            .title
            .clone()
            .filter(|t: &String| !t.is_empty())
            .unwrap_or_else(|| window_title(&limit.label)),
        fraction: if expired { 0.0 } else { limit.percent },
        resets_in: remaining.filter(|d: &Duration| !d.is_zero()),
    }
}

fn day_row(day: &Day, today: NaiveDate) -> DayRow {
    let date: Option<NaiveDate> = NaiveDate::parse_from_str(&day.date, "%Y-%m-%d").ok();
    let is_today: bool = date == Some(today);
    let label: String = match date {
        _ if is_today => String::from("Today"),
        Some(d) => weekday_name(d.weekday()).to_owned(),
        None => day.date.clone(),
    };
    DayRow {
        label,
        tokens: day.message_count,
        today: is_today,
    }
}

fn weekday_name(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    }
}

pub fn window_title(label: &str) -> String {
    let lower: String = label.to_lowercase();
    if lower.contains("month") {
        String::from("Monthly")
    } else if lower.contains("week") || lower.contains("7-day") {
        String::from("Weekly")
    } else if lower.contains("session") || lower.contains("hour") {
        String::from("Session")
    } else {
        let plain: &str = label.split('(').next().unwrap_or("").trim();
        if plain.is_empty() {
            String::from("Limit")
        } else {
            plain.to_owned()
        }
    }
}

pub fn format_tokens(n: u64) -> String {
    let value: f64 = n as f64;
    if value >= 1e9 {
        format!("{:.1}B", value / 1e9)
    } else if value >= 1e6 {
        format!("{:.1}M", value / 1e6)
    } else if value >= 1e3 {
        format!("{:.1}K", value / 1e3)
    } else {
        n.to_string()
    }
}

pub fn format_duration(remaining: Duration) -> String {
    let minutes: u64 = remaining.as_secs() / 60;
    let (hours, days): (u64, u64) = (minutes / 60, minutes / 1440);
    if days > 0 {
        format!("{days}d {}h", hours % 24)
    } else if hours > 0 {
        format!("{hours}h {}m", minutes % 60)
    } else {
        format!("{}m", minutes.max(1))
    }
}

pub fn friendly_model_name(id: &str) -> String {
    let name: &str = id.strip_prefix("claude-").unwrap_or(id);
    let mut words: Vec<String> = Vec::new();
    let mut version: Vec<&str> = Vec::new();
    for part in name.split('-').filter(|p: &&str| !p.is_empty()) {
        if part.len() == 8 && part.chars().all(|c: char| c.is_ascii_digit()) {
            continue;
        }
        if part.starts_with(|c: char| c.is_ascii_digit()) {
            version.push(part);
            continue;
        }
        if !version.is_empty() {
            words.push(version.join("."));
            version.clear();
        }
        words.push(word_case(part));
    }
    if !version.is_empty() {
        words.push(version.join("."));
    }
    if words.is_empty() {
        String::from("Unknown")
    } else {
        words.join(" ")
    }
}

fn word_case(word: &str) -> String {
    match word {
        "gpt" => String::from("GPT"),
        "deepseek" => String::from("DeepSeek"),
        _ => {
            let mut letters: std::str::Chars<'_> = word.chars();
            letters.next().map_or(String::new(), |first: char| {
                first.to_uppercase().chain(letters).collect()
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::parse;

    fn at(text: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(text).unwrap()
    }

    const NOW: &str = "2026-10-25T19:41:30-04:00";

    fn working(assignee: Option<&str>, page: usize) -> Work {
        Work::Active {
            bead: Bead {
                project: String::from("kindle"),
                id: String::from("kindle-bxx.4"),
                title: String::from("bead push"),
                priority: 1,
                issue_type: String::from("task"),
                assignee: assignee.map(str::to_owned),
                started: at("2026-10-26T09:00:00-04:00"),
                description: String::from("## What"),
            },
            now: at("2026-10-26T09:42:00-04:00"),
            page,
            pages: 4,
            quiet: 0,
        }
    }

    fn active(meta: &str, elapsed: &str, page: usize) -> BeadPage {
        BeadPage::Active {
            place: String::from("kindle \u{b7} kindle-bxx.4"),
            title: String::from("bead push"),
            meta: meta.to_owned(),
            description: String::from("## What"),
            elapsed: elapsed.to_owned(),
            page,
        }
    }

    #[test]
    fn an_active_bead_shows_where_it_is_its_title_metadata_text_and_page() {
        assert_eq!(
            bead_page(&working(Some("elendal"), 2), Duration::ZERO),
            active("P1 \u{b7} task \u{b7} elendal", "42m", 2)
        );
    }

    #[test]
    fn an_unassigned_bead_leaves_the_assignee_out_and_elapsed_time_runs_on() {
        assert_eq!(
            bead_page(&working(None, 0), Duration::from_secs(3600)),
            active("P1 \u{b7} task", "1h 42m", 0)
        );
    }

    #[test]
    fn before_any_bead_push_the_page_waits_and_after_an_empty_one_it_is_idle() {
        let idle: Work = Work::Idle {
            now: at("2026-10-26T09:42:00-04:00"),
        };
        assert_eq!(
            (
                bead_page(&Work::Unknown, Duration::ZERO),
                bead_page(&idle, Duration::ZERO)
            ),
            (BeadPage::Waiting, BeadPage::Idle)
        );
    }

    #[test]
    fn a_start_after_the_pushed_clock_shows_the_shortest_elapsed_time() {
        let bead: Bead = match working(None, 0) {
            Work::Active { bead, .. } => bead,
            Work::Unknown | Work::Idle { .. } => unreachable!(),
        };
        assert_eq!(
            active_page(&bead, at("2026-10-26T08:00:00-04:00"), 0),
            active("P1 \u{b7} task", "1m", 0)
        );
    }

    #[test]
    fn tokens_below_a_thousand_print_whole() {
        assert_eq!(
            (format_tokens(0), format_tokens(999)),
            (String::from("0"), String::from("999"))
        );
    }

    #[test]
    fn tokens_scale_to_k_m_and_b_with_one_decimal() {
        let scaled: [String; 3] = [
            format_tokens(1_000),
            format_tokens(601_409_290),
            format_tokens(3_092_349_207),
        ];
        assert_eq!(
            scaled,
            [
                String::from("1.0K"),
                String::from("601.4M"),
                String::from("3.1B")
            ]
        );
    }

    #[test]
    fn a_reset_under_a_minute_away_reads_one_minute() {
        assert_eq!(format_duration(Duration::from_secs(20)), "1m");
    }

    #[test]
    fn a_reset_hours_away_reads_hours_and_minutes() {
        assert_eq!(
            format_duration(Duration::from_secs(4 * 3600 + 28 * 60 + 30)),
            "4h 28m"
        );
    }

    #[test]
    fn a_reset_days_away_reads_days_and_hours() {
        assert_eq!(
            format_duration(Duration::from_secs(5 * 86400 + 5 * 3600 + 18 * 60)),
            "5d 5h"
        );
    }

    #[test]
    fn model_ids_lose_the_vendor_and_join_their_version() {
        let names: [String; 3] = [
            friendly_model_name("claude-opus-5-5"),
            friendly_model_name("claude-haiku-5-5-20261001"),
            friendly_model_name("gpt-5.6-sol"),
        ];
        assert_eq!(
            names,
            [
                String::from("Opus 5.5"),
                String::from("Haiku 5.5"),
                String::from("GPT 5.6 Sol")
            ]
        );
    }

    #[test]
    fn deepseek_keeps_its_capitals() {
        assert_eq!(friendly_model_name("deepseek-v3"), "DeepSeek V3");
    }

    #[test]
    fn an_empty_model_id_is_unknown() {
        assert_eq!(friendly_model_name("claude-"), "Unknown");
    }

    #[test]
    fn window_titles_follow_the_label() {
        let labels: [&str; 7] = [
            "session cap",
            "5-hour window",
            "weekly cap",
            "7-day window",
            "Monthly cap",
            "Opus (1M)",
            "(x)",
        ];
        let expected: [&str; 7] = [
            "Session", "Session", "Weekly", "Weekly", "Monthly", "Opus", "Limit",
        ];
        assert_eq!(labels.map(window_title), expected.map(String::from));
    }

    #[test]
    fn an_empty_record_reads_as_an_unnamed_subscription_with_nothing_to_show() {
        let view: Dashboard = dashboard(&Record::default(), at(NOW));
        let expected: Dashboard = Dashboard {
            title: String::from("Claude Code"),
            plan: String::from("Subscription"),
            limits: vec![],
            days: vec![],
            today: String::from("0 prompts \u{b7} 0 sessions"),
            models: vec![],
            updated: String::from("never updated"),
        };
        assert_eq!(view, expected);
    }

    #[test]
    fn a_status_text_replaces_the_plan_and_a_tier_is_capitalised() {
        let failing: Record = Record {
            usage_status_text: String::from("Sign in"),
            tier_label: String::from("pro"),
            ..Record::default()
        };
        let tiered: Record = Record {
            tier_label: String::from("pro"),
            ..Record::default()
        };
        assert_eq!(
            (
                dashboard(&failing, at(NOW)).plan,
                dashboard(&tiered, at(NOW)).plan
            ),
            (String::from("Sign in"), String::from("Pro"))
        );
    }

    #[test]
    fn a_live_limit_counts_down_to_its_reset_and_an_expired_one_reads_empty() {
        let record: Record = parse(
            r#"{"limits":[
                {"label":"Session (5-hour)","percent":0.03,"resetsAt":"2026-10-26T04:10:00+00:00"},
                {"label":"Weekly (7-day)","percent":0.9,"resetsAt":"2026-10-25T23:00:00+00:00"},
                {"label":"x","title":"Fable Weekly","percent":0.02},
                {"label":"unknown","percent":-1}]}"#,
        )
        .unwrap();
        let limits: Vec<Meter> = dashboard(&record, at(NOW)).limits;
        let expected: Vec<Meter> = vec![
            Meter {
                title: String::from("Session"),
                fraction: 0.03,
                resets_in: Some(Duration::from_secs(4 * 3600 + 28 * 60 + 30)),
            },
            Meter {
                title: String::from("Weekly"),
                fraction: 0.0,
                resets_in: None,
            },
            Meter {
                title: String::from("Fable Weekly"),
                fraction: 0.02,
                resets_in: None,
            },
        ];
        assert_eq!(limits, expected);
    }

    #[test]
    fn a_limit_resetting_exactly_now_reads_empty() {
        let record: Record = parse(r#"{"limits":[{"label":"Session","percent":0.5,"resetsAt":"2026-10-25T23:41:30+00:00"}]}"#).unwrap();
        assert_eq!(dashboard(&record, at(NOW)).limits[0].fraction, 0.0);
    }

    #[test]
    fn days_are_named_and_the_local_date_is_today() {
        let record: Record = parse(
            r#"{"recentDays":[{"date":"2026-10-24","messageCount":5},{"date":"2026-10-25","messageCount":9},{"date":"bad","messageCount":1}]}"#,
        )
        .unwrap();
        let expected: Vec<DayRow> = vec![
            DayRow {
                label: String::from("Sat"),
                tokens: 5,
                today: false,
            },
            DayRow {
                label: String::from("Today"),
                tokens: 9,
                today: true,
            },
            DayRow {
                label: String::from("bad"),
                tokens: 1,
                today: false,
            },
        ];
        assert_eq!(dashboard(&record, at(NOW)).days, expected);
    }

    #[test]
    fn today_follows_the_local_offset_not_utc() {
        let record: Record =
            parse(r#"{"recentDays":[{"date":"2026-10-25","messageCount":9}]}"#).unwrap();
        assert!(dashboard(&record, at("2026-10-25T23:30:00-04:00")).days[0].today);
    }

    #[test]
    fn the_four_heaviest_models_are_listed_heaviest_first() {
        let record: Record = parse(
            r#"{"modelUsage":{
                "claude-a":{"inputTokens":1},"claude-b":{"outputTokens":5},"claude-c":{"cacheReadInputTokens":3},
                "claude-d":{"cacheCreationInputTokens":4},"claude-e":{"inputTokens":2}}}"#,
        )
        .unwrap();
        let names: Vec<(String, u64)> = dashboard(&record, at(NOW))
            .models
            .into_iter()
            .map(|m: ModelRow| (m.name, m.tokens))
            .collect();
        assert_eq!(
            names,
            vec![
                (String::from("B"), 5),
                (String::from("D"), 4),
                (String::from("C"), 3),
                (String::from("E"), 2)
            ]
        );
    }

    #[test]
    fn the_update_time_is_shown_in_the_local_offset() {
        let record: Record = Record {
            updated_at: Some(String::from("2026-10-25T23:39:03.2+00:00")),
            ..Record::default()
        };
        assert_eq!(dashboard(&record, at(NOW)).updated, "updated 19:39");
    }

    #[test]
    fn before_the_first_push_the_page_says_it_is_waiting() {
        let page: Dashboard = waiting();
        assert_eq!(
            (
                page.plan.as_str(),
                page.limits.len(),
                page.days.len(),
                page.models.len()
            ),
            ("waiting for devbox", 0, 0, 0)
        );
    }

    #[test]
    fn todays_prompts_and_sessions_are_summed_up() {
        let record: Record = Record {
            today_prompts: 2521,
            today_sessions: 51,
            ..Record::default()
        };
        assert_eq!(
            dashboard(&record, at(NOW)).today,
            "2521 prompts \u{b7} 51 sessions"
        );
    }
}
