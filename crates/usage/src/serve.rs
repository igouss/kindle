use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant, SystemTime};

use eink::canvas::{Canvas, Geometry, Port};
use eink::epdc::{Epdc, Refresh};
use eink::touch::{Debounce, EVENT_SIZE, Frames, TouchDown, parse};

use crate::app::{self, Effect, Event, Light, Menu, State, View, Work};
use crate::message::{self, Message};
use crate::screen::{self, Faces, MenuView, Page};
use crate::settings::{self, Settings};
use crate::view::{self, BeadPage, Dashboard};

pub const SOCKET: &str = "/var/run/usage.sock";
const TOUCH: &str = "/dev/input/event1";
const SETTINGS: &str = "/mnt/us/kindle-dev/usage.settings";
const HANDED_BACK: &str = "/var/run/dashboard.off";
const QUIET: Duration = Duration::from_millis(400);

enum Inbound {
    Touch(TouchDown),
    Pushed(Box<Message>, Sender<String>),
    Broken(String),
}

struct Shell {
    epdc: Epdc,
    faces: Faces,
    marker: u32,
    pushed_at: Instant,
    bead_pushed_at: Instant,
}

pub fn run() -> Result<(), String> {
    let faces: Faces = Faces {
        regular: eink::text::Typeface::regular()?,
        bold: eink::text::Typeface::bold()?,
    };
    let epdc: Epdc = Epdc::open().map_err(|e: eink::epdc::Fault| e.to_string())?;
    let saved: Settings = settings::parse(&std::fs::read_to_string(SETTINGS).unwrap_or_default());
    let max: u32 = powerd("flMaxIntensity")?;
    let current: u32 = powerd("flIntensity")?;
    let light: Light = app::initial_light(saved, current, max);
    let mut state: State = State {
        view: View::Usage,
        menu: Menu::Closed,
        port: saved.port.unwrap_or(Port::Right),
        light,
        feed: None,
        work: Work::Unknown,
        epic: None,
    };
    let (sender, inbox): (Sender<Inbound>, Receiver<Inbound>) = mpsc::channel();
    spawn_touch(sender.clone());
    spawn_socket(sender)?;

    let mut shell: Shell = Shell {
        epdc,
        faces,
        marker: 1,
        pushed_at: Instant::now(),
        bead_pushed_at: Instant::now(),
    };
    let mut debounce: Debounce = Debounce::new(QUIET);
    apply(
        &mut shell,
        &state,
        &[
            Effect::Light(state.light.intensity()),
            Effect::Draw(Refresh::Flash),
        ],
    )?;
    for message in inbox {
        let (event, reply): (Event, Option<Sender<String>>) = match message {
            Inbound::Broken(fault) => return Err(fault),
            Inbound::Touch(touch) => {
                if !debounce.accept(touch) {
                    continue;
                }
                let page: Geometry = shell.epdc.geometry().landscape();
                (Event::Tap(state.port.to_page(touch.at, page)), None)
            }
            Inbound::Pushed(message, reply) => match *message {
                Message::Usage(feed, refresh) => {
                    shell.pushed_at = Instant::now();
                    (Event::Pushed(feed, refresh), Some(reply))
                }
                Message::Bead(feed) => {
                    shell.bead_pushed_at = Instant::now();
                    let pages: usize = feed.bead.as_ref().map_or(1, |bead: &crate::bead::Bead| {
                        screen::pages(&shell.faces, &view::active_page(bead, feed.now, 0))
                    });
                    (Event::BeadPushed(feed, pages), Some(reply))
                }
            },
        };
        let (next, effects): (State, Vec<Effect>) = app::step(state, event);
        state = next;
        let outcome: Result<Flow, String> = apply(&mut shell, &state, &effects);
        debounce.hold(since_epoch() + QUIET);
        if let Some(reply) = reply {
            let line: String = match &outcome {
                Ok(_) => format!("ok {}", app::summary(&state)),
                Err(fault) => format!("error {fault}"),
            };
            let _ = reply.send(line);
        }
        if outcome? == Flow::Leave {
            return hand_back();
        }
    }
    Err(String::from("every input closed"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flow {
    Stay,
    Leave,
}

fn hand_back() -> Result<(), String> {
    std::fs::write(HANDED_BACK, "").map_err(|e: std::io::Error| format!("{HANDED_BACK}: {e}"))?;
    set_powerd("preventScreenSaver", 0)?;
    let status: std::process::ExitStatus =
        Command::new("/sbin/start")
            .arg("lab126_gui")
            .status()
            .map_err(|e: std::io::Error| format!("start lab126_gui: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("start lab126_gui: {status}"))
    }
}

fn apply(shell: &mut Shell, state: &State, effects: &[Effect]) -> Result<Flow, String> {
    for effect in effects {
        match *effect {
            Effect::Light(intensity) => set_powerd("flIntensity", intensity)?,
            Effect::Save => save(state)?,
            Effect::Draw(refresh) => draw(shell, state, refresh)?,
            Effect::Exit => return Ok(Flow::Leave),
        }
    }
    Ok(Flow::Stay)
}

fn draw(shell: &mut Shell, state: &State, refresh: Refresh) -> Result<(), String> {
    let dashboard: Dashboard = match &state.feed {
        Some(feed) => view::dashboard(&feed.record, feed.now_after(shell.pushed_at.elapsed())),
        None => view::waiting(),
    };
    let bead: BeadPage = view::bead_page(&state.work, shell.bead_pushed_at.elapsed());
    let shown: Page<'_> = match state.view {
        View::Bead => Page::Bead(&bead),
        View::Usage => Page::Usage(&dashboard),
    };
    let menu: Option<MenuView> = match state.menu {
        Menu::Open => Some(MenuView {
            port: state.port,
            light: state.light,
        }),
        Menu::Closed => None,
    };
    let panel: Geometry = shell.epdc.geometry();
    let page: Canvas = screen::render(&shown, &shell.faces, panel.landscape(), menu);
    let canvas: Canvas = page.upright(state.port, panel.stride);
    shell.marker = shell.marker.wrapping_add(1).max(1);
    shell
        .epdc
        .present(&canvas, shell.marker, refresh)
        .map_err(|e: eink::epdc::Fault| e.to_string())?;
    Ok(())
}

fn save(state: &State) -> Result<(), String> {
    let text: String = settings::render(state.port, state.light.level, state.light.on);
    let staging: String = format!("{SETTINGS}.new");
    std::fs::write(&staging, text).map_err(|e: std::io::Error| format!("{staging}: {e}"))?;
    std::fs::rename(&staging, SETTINGS).map_err(|e: std::io::Error| format!("{SETTINGS}: {e}"))
}

fn since_epoch() -> Duration {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
}

fn powerd(property: &str) -> Result<u32, String> {
    let output: std::process::Output = Command::new("lipc-get-prop")
        .args(["com.lab126.powerd", property])
        .output()
        .map_err(|e: std::io::Error| format!("lipc-get-prop {property}: {e}"))?;
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u32>()
        .map_err(|e: std::num::ParseIntError| format!("lipc-get-prop {property}: {e}"))
}

fn set_powerd(property: &str, value: u32) -> Result<(), String> {
    let status: std::process::ExitStatus = Command::new("lipc-set-prop")
        .args(["com.lab126.powerd", property, &value.to_string()])
        .status()
        .map_err(|e: std::io::Error| format!("lipc-set-prop {property}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("lipc-set-prop {property} {value}: {status}"))
    }
}

fn spawn_touch(sender: Sender<Inbound>) {
    std::thread::spawn(move || {
        let fault: String = match touches(&sender) {
            Ok(()) => format!("{TOUCH} closed"),
            Err(e) => format!("{TOUCH}: {e}"),
        };
        let _ = sender.send(Inbound::Broken(fault));
    });
}

fn touches(sender: &Sender<Inbound>) -> std::io::Result<()> {
    let mut device: File = File::open(TOUCH)?;
    let mut frames: Frames = Frames::default();
    let mut bytes: [u8; EVENT_SIZE] = [0; EVENT_SIZE];
    loop {
        device.read_exact(&mut bytes)?;
        if let Some(touch) = frames.feed(parse(&bytes))
            && sender.send(Inbound::Touch(touch)).is_err()
        {
            return Ok(());
        }
    }
}

fn spawn_socket(sender: Sender<Inbound>) -> Result<(), String> {
    let _ = std::fs::remove_file(SOCKET);
    let listener: UnixListener =
        UnixListener::bind(SOCKET).map_err(|e: std::io::Error| format!("{SOCKET}: {e}"))?;
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            if serve_push(stream, &sender).is_err() {
                return;
            }
        }
    });
    Ok(())
}

fn serve_push(mut stream: UnixStream, sender: &Sender<Inbound>) -> Result<(), ()> {
    let mut text: String = String::new();
    let reply_line: String = match stream.read_to_string(&mut text) {
        Err(e) => format!("error reading push: {e}"),
        Ok(_) => match message::decode(&text) {
            Err(fault) => format!("error {fault}"),
            Ok(message) => {
                let (reply, answer): (Sender<String>, Receiver<String>) = mpsc::channel();
                sender
                    .send(Inbound::Pushed(Box::new(message), reply))
                    .map_err(|_| ())?;
                answer
                    .recv()
                    .unwrap_or_else(|_| String::from("error the dashboard stopped"))
            }
        },
    };
    let _ = writeln!(stream, "{reply_line}");
    Ok(())
}
