mod br;
mod epic;
mod show;
mod snapshot;
mod workspaces;

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use chrono::{DateTime, FixedOffset};
use inotify::{Inotify, WatchDescriptor, WatchMask, Watches};

use epic::Epic;
use show::Shown;
use snapshot::{Candidate, Issue, Payload, Snapshot};

const SETTLE: Duration = Duration::from_millis(500);
const RESCAN: Duration = Duration::from_secs(60);
const HEARTBEAT: Duration = Duration::from_secs(300);
const USAGE: &str = "usage: kindle-watch [--once] [--root DIR] [--kindle HOST] [--remote PATH]";

struct Options {
    once: bool,
    root: PathBuf,
    kindle: String,
    remote: String,
}

fn options(args: &[String]) -> Result<Options, String> {
    let home: String = std::env::var("HOME").map_err(|_| String::from("HOME is not set"))?;
    let mut options: Options = Options {
        once: false,
        root: Path::new(&home).join("Projects"),
        kindle: String::from("kindle"),
        remote: String::from("/mnt/us/kindle-dev/usage"),
    };
    let mut rest: std::slice::Iter<'_, String> = args.iter();
    while let Some(arg) = rest.next() {
        let mut value = || rest.next().cloned().ok_or_else(|| String::from(USAGE));
        match arg.as_str() {
            "--once" => options.once = true,
            "--root" => options.root = PathBuf::from(value()?),
            "--kindle" => options.kindle = value()?,
            "--remote" => options.remote = value()?,
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    Ok(options)
}

fn candidates(workspace: &Path) -> Result<Vec<Candidate>, String> {
    let issues: Vec<Issue> =
        snapshot::in_progress(&br::run(workspace, &["list", "--status", "in_progress"])?)?;
    if issues.is_empty() {
        return Ok(vec![]);
    }
    let me: String = snapshot::actor(
        &br::run(workspace, &["config", "get", "actor"])?,
        std::env::var("USER").ok().as_deref(),
    )?;
    issues
        .into_iter()
        .map(|issue: Issue| {
            let audit: String = br::run(workspace, &["audit", "log", &issue.id])?;
            Ok(
                snapshot::started(&audit, &me)?.map(|started: DateTime<FixedOffset>| Candidate {
                    project: workspaces::project(workspace),
                    started,
                    issue,
                }),
            )
        })
        .filter_map(Result::transpose)
        .collect()
}

fn chosen(known: &BTreeMap<PathBuf, Vec<Candidate>>) -> Option<(PathBuf, Candidate)> {
    let all: Vec<(PathBuf, Candidate)> = known
        .iter()
        .flat_map(|(path, list): (&PathBuf, &Vec<Candidate>)| {
            list.iter().map(|c: &Candidate| (path.clone(), c.clone()))
        })
        .collect();
    let flat: Vec<Candidate> = all
        .iter()
        .map(|(_, c): &(PathBuf, Candidate)| c.clone())
        .collect();
    let newest: &Candidate = snapshot::newest(&flat)?;
    all.into_iter()
        .find(|(_, c): &(PathBuf, Candidate)| c == newest)
}

fn shown(workspace: &Path, id: &str) -> Result<Shown, String> {
    show::parse(&br::run(workspace, &["show", id])?)
}

fn epic(workspace: &Path, root: &str) -> Result<Epic, String> {
    let head: Shown = shown(workspace, root)?;
    let children: Vec<Shown> = epic::children(&head)
        .iter()
        .map(|member: &epic::Member| shown(workspace, &member.id))
        .collect::<Result<Vec<Shown>, String>>()?;
    Ok(epic::assemble(&head, &children))
}

fn push(options: &Options, payload: &Payload) -> Result<String, String> {
    let now: String = chrono::Local::now().fixed_offset().to_rfc3339();
    let mut child: std::process::Child = Command::new("ssh")
        .args([
            "-o",
            "BatchMode=yes",
            "-o",
            "LogLevel=ERROR",
            "-o",
            "ConnectTimeout=5",
        ])
        .arg(&options.kindle)
        .args([options.remote.as_str(), "bead", "--now", now.as_str()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e: std::io::Error| format!("ssh: {e}"))?;
    child
        .stdin
        .take()
        .ok_or_else(|| String::from("ssh: no stdin"))?
        .write_all(snapshot::wire(payload).as_bytes())
        .map_err(|e: std::io::Error| format!("ssh: {e}"))?;
    let output: std::process::Output = child
        .wait_with_output()
        .map_err(|e: std::io::Error| format!("ssh: {e}"))?;
    let said: String = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if output.status.success() {
        Ok(said)
    } else {
        Err(format!(
            "push: {} {}",
            said,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

struct Watcher {
    options: Options,
    watches: Watches,
    watched: BTreeMap<WatchDescriptor, PathBuf>,
    known: BTreeMap<PathBuf, Vec<Candidate>>,
    pushed: Option<Payload>,
    pushed_at: Instant,
    root: Option<(PathBuf, String)>,
}

impl Watcher {
    fn rescan(&mut self) -> Vec<PathBuf> {
        let mut added: Vec<PathBuf> = vec![];
        for workspace in workspaces::find(&self.options.root) {
            if self.known.contains_key(&workspace) {
                continue;
            }
            match self.watches.add(
                workspace.join(".beads"),
                WatchMask::MODIFY | WatchMask::MOVED_TO,
            ) {
                Ok(descriptor) => {
                    self.watched.insert(descriptor, workspace.clone());
                    self.known.insert(workspace.clone(), vec![]);
                    added.push(workspace);
                }
                Err(e) => eprintln!("watch {}: {e}", workspace.display()),
            }
        }
        added
    }

    fn refresh(&mut self, workspaces: &[PathBuf]) {
        for workspace in workspaces {
            match candidates(workspace) {
                Ok(list) => {
                    eprintln!(
                        "queried {}: {} in progress",
                        workspace.display(),
                        list.len()
                    );
                    self.known.insert(workspace.clone(), list);
                }
                Err(fault) => eprintln!("{fault}"),
            }
        }
    }

    fn payload(&mut self) -> Result<Payload, String> {
        let Some((workspace, candidate)) = chosen(&self.known) else {
            let epic: Option<Epic> = match &self.root {
                Some((workspace, root)) => Some(epic(workspace, root)?),
                None => None,
            };
            return Ok(Payload { bead: None, epic });
        };
        let head: Shown = shown(&workspace, &candidate.issue.id)?;
        let root: String = epic::root(&head);
        let epic: Epic = epic(&workspace, &root)?;
        self.root = Some((workspace, root));
        let bead: Snapshot = snapshot::snapshot(&candidate, head.description.unwrap_or_default());
        Ok(Payload {
            bead: Some(bead),
            epic: Some(epic),
        })
    }

    fn publish(&mut self, heartbeat: bool) {
        let next: Payload = match self.payload() {
            Ok(next) => next,
            Err(fault) => return eprintln!("{fault}"),
        };
        if !heartbeat && self.pushed.as_ref() == Some(&next) {
            return;
        }
        match push(&self.options, &next) {
            Ok(said) => {
                let id: &str = next
                    .bead
                    .as_ref()
                    .map_or("nothing", |s: &Snapshot| s.id.as_str());
                eprintln!("pushed {id}: {said}");
                self.pushed = Some(next);
                self.pushed_at = Instant::now();
            }
            Err(fault) => eprintln!("{fault}"),
        }
    }
}

fn spawn_events(mut inotify: Inotify, sender: Sender<WatchDescriptor>) {
    std::thread::spawn(move || {
        let mut buffer: [u8; 4096] = [0; 4096];
        loop {
            let events: inotify::Events<'_> = match inotify.read_events_blocking(&mut buffer) {
                Ok(events) => events,
                Err(e) => return eprintln!("inotify: {e}"),
            };
            for event in events {
                if !event.name.is_some_and(workspaces::carries_state) {
                    continue;
                }
                if sender.send(event.wd).is_err() {
                    return;
                }
            }
        }
    });
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let options: Options = options(&args)?;
    let inotify: Inotify = Inotify::init().map_err(|e: std::io::Error| format!("inotify: {e}"))?;
    let once: bool = options.once;
    let mut watcher: Watcher = Watcher {
        options,
        watches: inotify.watches(),
        watched: BTreeMap::new(),
        known: BTreeMap::new(),
        pushed: None,
        pushed_at: Instant::now(),
        root: None,
    };
    let all: Vec<PathBuf> = watcher.rescan();
    eprintln!(
        "watching {} workspaces under {}",
        all.len(),
        watcher.options.root.display()
    );
    watcher.refresh(&all);
    watcher.publish(false);
    if once {
        return Ok(());
    }
    let (sender, changes): (Sender<WatchDescriptor>, Receiver<WatchDescriptor>) = mpsc::channel();
    spawn_events(inotify, sender);
    let mut dirty: Vec<PathBuf> = vec![];
    let mut scanned_at: Instant = Instant::now();
    loop {
        let wait: Duration = if dirty.is_empty() { RESCAN } else { SETTLE };
        match changes.recv_timeout(wait) {
            Ok(descriptor) => {
                if let Some(workspace) = watcher.watched.get(&descriptor)
                    && !dirty.contains(workspace)
                {
                    dirty.push(workspace.clone());
                }
                continue;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Err(String::from("inotify stopped")),
        }
        let mut changed: bool = !dirty.is_empty();
        watcher.refresh(&std::mem::take(&mut dirty));
        if scanned_at.elapsed() >= RESCAN {
            scanned_at = Instant::now();
            let added: Vec<PathBuf> = watcher.rescan();
            changed |= !added.is_empty();
            watcher.refresh(&added);
        }
        let heartbeat: bool = watcher.pushed_at.elapsed() >= HEARTBEAT;
        if changed || heartbeat {
            watcher.publish(heartbeat);
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(fault) => {
            eprintln!("kindle-watch: {fault}");
            ExitCode::FAILURE
        }
    }
}
