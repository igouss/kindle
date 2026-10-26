mod app;
mod menu;
mod message;
mod record;
mod screen;
mod serve;
mod settings;
mod view;

use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::process::ExitCode;

use chrono::{DateTime, FixedOffset};

const USAGE: &str =
    "usage: usage serve\n       usage push --now <RFC 3339 time> [--flash] < claude.json";

#[derive(Debug, PartialEq)]
enum Command {
    Serve,
    Push {
        now: DateTime<FixedOffset>,
        flash: bool,
    },
}

fn command(args: &[String]) -> Result<Command, String> {
    match args.split_first() {
        Some((first, rest)) if first == "serve" && rest.is_empty() => Ok(Command::Serve),
        Some((first, rest)) if first == "push" => push_options(rest),
        Some((other, _)) => Err(format!("unknown command {other}\n{USAGE}")),
        None => Err(String::from(USAGE)),
    }
}

fn push_options(args: &[String]) -> Result<Command, String> {
    let mut now: Option<DateTime<FixedOffset>> = None;
    let mut flash: bool = false;
    let mut rest: std::slice::Iter<'_, String> = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--flash" => flash = true,
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
    Ok(Command::Push {
        now: now.ok_or(USAGE)?,
        flash,
    })
}

fn push(now: DateTime<FixedOffset>, flash: bool) -> Result<String, String> {
    let mut record: String = String::new();
    std::io::stdin()
        .read_to_string(&mut record)
        .map_err(|e: std::io::Error| format!("stdin: {e}"))?;
    record::parse(&record).map_err(|e: serde_json::Error| format!("usage record: {e}"))?;
    let message: String = message::encode_usage(now, flash, &record);
    let mut stream: UnixStream =
        UnixStream::connect(serve::SOCKET).map_err(|e: std::io::Error| {
            format!("{}: {e} (is `usage serve` running?)", serve::SOCKET)
        })?;
    stream
        .write_all(message.as_bytes())
        .map_err(|e: std::io::Error| format!("send: {e}"))?;
    stream
        .shutdown(Shutdown::Write)
        .map_err(|e: std::io::Error| format!("send: {e}"))?;
    let mut reply: String = String::new();
    stream
        .read_to_string(&mut reply)
        .map_err(|e: std::io::Error| format!("reply: {e}"))?;
    match reply.trim().strip_prefix("ok ") {
        Some(done) => Ok(done.to_owned()),
        None => Err(reply.trim().to_owned()),
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match command(&args)? {
        Command::Serve => serve::run(),
        Command::Push { now, flash } => push(now, flash).map(|done: String| println!("{done}")),
    }
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

    fn at(text: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(text).unwrap()
    }

    #[test]
    fn serve_takes_no_arguments() {
        assert_eq!(
            (
                command(&args(&["serve"])),
                command(&args(&["serve", "x"])).is_err()
            ),
            (Ok(Command::Serve), true)
        );
    }

    #[test]
    fn push_needs_now_and_may_flash() {
        let quiet: Result<Command, String> =
            command(&args(&["push", "--now", "2026-10-25T19:41:30-04:00"]));
        let flash: Result<Command, String> = command(&args(&[
            "push",
            "--flash",
            "--now",
            "2026-10-25T19:41:30-04:00",
        ]));
        let now: DateTime<FixedOffset> = at("2026-10-25T19:41:30-04:00");
        assert_eq!(
            (quiet, flash),
            (
                Ok(Command::Push { now, flash: false }),
                Ok(Command::Push { now, flash: true })
            )
        );
    }

    #[test]
    fn no_command_is_refused_with_the_usage() {
        assert_eq!(command(&[]), Err(String::from(USAGE)));
    }

    #[test]
    fn push_without_now_a_dangling_now_or_a_bad_time_is_refused() {
        let refused: [bool; 3] = [
            command(&args(&["push"])).is_err(),
            command(&args(&["push", "--now"])).is_err(),
            command(&args(&["push", "--now", "tomorrow"])).is_err(),
        ];
        assert_eq!(refused, [true, true, true]);
    }

    #[test]
    fn an_unknown_command_or_argument_is_refused() {
        let unknown: Result<Command, String> = command(&args(&["draw"]));
        let argument: Result<Command, String> = command(&args(&[
            "push",
            "--now",
            "2026-10-25T19:41:30-04:00",
            "--port",
        ]));
        assert!(
            unknown.is_err_and(|e: String| e.starts_with("unknown command draw"))
                && argument.is_err_and(|e: String| e.starts_with("unknown argument --port"))
        );
    }
}
