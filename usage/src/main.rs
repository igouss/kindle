mod record;
mod screen;
mod view;

use std::io::Read;
use std::process::ExitCode;

use chrono::{DateTime, FixedOffset};
use eink::canvas::Canvas;
use eink::epdc::{Epdc, Refresh};
use eink::text::Typeface;

use record::Record;
use screen::Faces;
use view::Dashboard;

const USAGE: &str = "usage: usage --now <RFC 3339 time> [--flash] < claude.json";

struct Options {
    now: DateTime<FixedOffset>,
    refresh: Refresh,
}

fn options(args: &[String]) -> Result<Options, String> {
    let mut now: Option<DateTime<FixedOffset>> = None;
    let mut refresh: Refresh = Refresh::Quiet;
    let mut rest: std::slice::Iter<'_, String> = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--flash" => refresh = Refresh::Flash,
            "--now" => {
                let text: &String = rest.next().ok_or(USAGE)?;
                now = Some(
                    DateTime::parse_from_rfc3339(text)
                        .map_err(|e: chrono::ParseError| format!("--now {text}: {e}"))?,
                );
            }
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    Ok(Options {
        now: now.ok_or(USAGE)?,
        refresh,
    })
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let options: Options = options(&args)?;
    let mut input: String = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|e: std::io::Error| format!("stdin: {e}"))?;
    let record: Record =
        record::parse(&input).map_err(|e: serde_json::Error| format!("usage record: {e}"))?;
    let faces: Faces = Faces {
        regular: Typeface::regular()?,
        bold: Typeface::bold()?,
    };
    let epdc: Epdc = Epdc::open().map_err(|e: eink::epdc::Fault| e.to_string())?;
    let view: Dashboard = view::dashboard(&record, options.now);
    let canvas: Canvas = screen::render(&view, &faces, epdc.geometry());
    epdc.present(&canvas, std::process::id(), options.refresh)
        .map_err(|e: eink::epdc::Fault| e.to_string())?;
    println!(
        "{} \u{b7} {} \u{b7} {} limits",
        view.plan,
        view.updated,
        view.limits.len()
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(fault) => {
            eprintln!("usage: {fault}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s: &&str| s.to_string()).collect()
    }

    #[test]
    fn now_alone_refreshes_quietly() {
        let parsed: Options = options(&args(&["--now", "2026-10-25T19:41:30-04:00"])).unwrap();
        assert_eq!(
            (parsed.now.to_rfc3339(), parsed.refresh),
            (String::from("2026-10-25T19:41:30-04:00"), Refresh::Quiet)
        );
    }

    #[test]
    fn flash_asks_for_a_full_refresh() {
        assert_eq!(
            options(&args(&["--flash", "--now", "2026-10-25T19:41:30-04:00"]))
                .unwrap()
                .refresh,
            Refresh::Flash
        );
    }

    #[test]
    fn no_arguments_is_refused_with_the_usage() {
        assert_eq!(options(&[]).err(), Some(String::from(USAGE)));
    }

    #[test]
    fn a_dangling_now_or_a_bad_time_is_refused() {
        assert!(
            options(&args(&["--now"])).is_err() && options(&args(&["--now", "tomorrow"])).is_err()
        );
    }

    #[test]
    fn an_unknown_argument_is_refused() {
        assert!(
            options(&args(&["--now", "2026-10-25T19:41:30-04:00", "--fast"]))
                .err()
                .is_some_and(|e: String| e.starts_with("unknown argument --fast"))
        );
    }
}
