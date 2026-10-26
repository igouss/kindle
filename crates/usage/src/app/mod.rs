use std::time::Duration;

use chrono::{DateTime, FixedOffset, TimeDelta};
use eink::canvas::{Point, Port};
use eink::epdc::Refresh;

use crate::bead::{Bead, Epic};
use crate::menu::{self, Direction, Hit, Target};
use crate::record::Record;
use crate::settings::Settings;

pub const LIGHT_STEP: u32 = 3;
pub const DEFAULT_LEVEL: u32 = 12;
pub const FLASH_EVERY: u32 = 6;

#[derive(Debug, PartialEq)]
pub struct Feed {
    pub record: Record,
    pub now: DateTime<FixedOffset>,
}

impl Feed {
    pub fn now_after(&self, elapsed: Duration) -> DateTime<FixedOffset> {
        self.now + TimeDelta::from_std(elapsed).unwrap_or(TimeDelta::MAX)
    }
}

#[derive(Debug, PartialEq)]
pub struct BeadFeed {
    pub bead: Option<Bead>,
    pub epic: Option<Epic>,
    pub now: DateTime<FixedOffset>,
}

#[derive(Debug, PartialEq)]
pub enum Work {
    Unknown,
    Idle {
        now: DateTime<FixedOffset>,
    },
    Active {
        bead: Bead,
        now: DateTime<FixedOffset>,
        page: usize,
        pages: usize,
        quiet: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Bead,
    Graph,
    Usage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Menu {
    Closed,
    Open,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Light {
    pub level: u32,
    pub on: bool,
    pub max: u32,
}

impl Light {
    pub fn intensity(self) -> u32 {
        if self.on { self.level } else { 0 }
    }

    fn stepped(self, up: bool) -> Light {
        let level: u32 = if up {
            self.level.saturating_add(LIGHT_STEP)
        } else {
            self.level.saturating_sub(LIGHT_STEP)
        };
        Light {
            level: level.clamp(1, self.max.max(1)),
            on: true,
            max: self.max,
        }
    }
}

pub fn initial_light(saved: Settings, current: u32, max: u32) -> Light {
    let fallback: u32 = if current > 0 { current } else { DEFAULT_LEVEL };
    Light {
        level: saved.level.unwrap_or(fallback).clamp(1, max.max(1)),
        on: saved.on.unwrap_or(current > 0),
        max,
    }
}

#[derive(Debug, PartialEq)]
pub struct State {
    pub view: View,
    pub menu: Menu,
    pub port: Port,
    pub light: Light,
    pub feed: Option<Feed>,
    pub work: Work,
    pub epic: Option<Epic>,
}

#[derive(Debug, PartialEq)]
pub enum Event {
    Tap(Point),
    Pushed(Feed, Refresh),
    BeadPushed(BeadFeed, usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Draw(Refresh),
    Light(u32),
    Save,
    Exit,
}

pub fn summary(state: &State) -> String {
    let side: &str = match state.port {
        Port::Left => "left",
        Port::Right => "right",
    };
    format!("drawn, USB {side}, light {}", state.light.intensity())
}

pub fn step(state: State, event: Event) -> (State, Vec<Effect>) {
    match event {
        Event::Pushed(feed, refresh) => {
            let effects: Vec<Effect> = shown(&state, View::Usage, refresh);
            (
                State {
                    feed: Some(feed),
                    ..state
                },
                effects,
            )
        }
        Event::BeadPushed(feed, pages) => sighted(state, feed, pages.max(1)),
        Event::Tap(point) => {
            let hit: Hit = menu::hit(point, state.menu);
            tap(state, hit)
        }
    }
}

fn shown(state: &State, view: View, refresh: Refresh) -> Vec<Effect> {
    if state.view == view {
        vec![Effect::Draw(refresh)]
    } else {
        vec![]
    }
}

fn work_shown(state: &State) -> Vec<Effect> {
    match state.view {
        View::Bead | View::Graph => vec![Effect::Draw(Refresh::Quiet)],
        View::Usage => vec![],
    }
}

fn sighted(state: State, feed: BeadFeed, pages: usize) -> (State, Vec<Effect>) {
    let BeadFeed { bead, epic, now } = feed;
    let state: State = State { epic, ..state };
    let kept: Option<(usize, u32)> = match (&state.work, &bead) {
        (
            Work::Active {
                bead: was,
                page,
                quiet,
                ..
            },
            Some(is),
        ) if was.id == is.id => Some((*page, *quiet)),
        (Work::Active { .. } | Work::Idle { .. } | Work::Unknown, _) => None,
    };
    match (bead, kept) {
        (Some(bead), Some((page, quiet))) => {
            let effects: Vec<Effect> = work_shown(&state);
            let work: Work = Work::Active {
                bead,
                now,
                page: page.min(pages - 1),
                pages,
                quiet,
            };
            (State { work, ..state }, effects)
        }
        (Some(bead), None) => (
            State {
                view: View::Bead,
                menu: Menu::Closed,
                work: Work::Active {
                    bead,
                    now,
                    page: 0,
                    pages,
                    quiet: 0,
                },
                ..state
            },
            vec![Effect::Draw(Refresh::Flash)],
        ),
        (None, _) if matches!(state.work, Work::Active { .. }) => (
            State {
                view: View::Graph,
                menu: Menu::Closed,
                work: Work::Idle { now },
                ..state
            },
            vec![Effect::Draw(Refresh::Flash)],
        ),
        (None, _) => {
            let effects: Vec<Effect> = work_shown(&state);
            (
                State {
                    work: Work::Idle { now },
                    ..state
                },
                effects,
            )
        }
    }
}

fn tap(state: State, hit: Hit) -> (State, Vec<Effect>) {
    let current: Light = state.light;
    match (state.menu, hit) {
        (Menu::Closed, Hit::Button) => (
            State {
                menu: Menu::Open,
                ..state
            },
            vec![Effect::Draw(Refresh::Quiet)],
        ),
        (Menu::Closed, Hit::Tab(view)) if view == state.view => (state, vec![]),
        (Menu::Closed, Hit::Tab(view)) => {
            (State { view, ..state }, vec![Effect::Draw(Refresh::Flash)])
        }
        (Menu::Closed, Hit::Page(direction)) => leaf(state, direction),
        (Menu::Closed, Hit::Target(_) | Hit::Panel | Hit::Outside) => (state, vec![]),
        (
            Menu::Open,
            Hit::Button | Hit::Tab(_) | Hit::Page(_) | Hit::Outside | Hit::Target(Target::Close),
        ) => (
            State {
                menu: Menu::Closed,
                ..state
            },
            vec![Effect::Draw(Refresh::Quiet)],
        ),
        (Menu::Open, Hit::Panel) => (state, vec![]),
        (Menu::Open, Hit::Target(Target::KindleUi)) => (state, vec![Effect::Exit]),
        (Menu::Open, Hit::Target(Target::PortLeft)) => turn(state, Port::Left),
        (Menu::Open, Hit::Target(Target::PortRight)) => turn(state, Port::Right),
        (Menu::Open, Hit::Target(Target::Dimmer)) => light(state, current.stepped(false)),
        (Menu::Open, Hit::Target(Target::Brighter)) => light(state, current.stepped(true)),
        (Menu::Open, Hit::Target(Target::LightSwitch)) => {
            let switched: Light = Light {
                on: !state.light.on,
                ..state.light
            };
            light(state, switched)
        }
    }
}

fn leaf(state: State, direction: Direction) -> (State, Vec<Effect>) {
    let State { work, view, .. } = state;
    match (view, work) {
        (
            View::Bead,
            Work::Active {
                bead,
                now,
                page,
                pages,
                quiet,
            },
        ) => {
            let next: usize = match direction {
                Direction::Back => page.saturating_sub(1),
                Direction::Forward => (page + 1).min(pages - 1),
            };
            let (quiet, effects): (u32, Vec<Effect>) = match (next == page, quiet + 1) {
                (true, _) => (quiet, vec![]),
                (false, turned) if turned >= FLASH_EVERY => (0, vec![Effect::Draw(Refresh::Flash)]),
                (false, turned) => (turned, vec![Effect::Draw(Refresh::Quiet)]),
            };
            let work: Work = Work::Active {
                bead,
                now,
                page: next,
                pages,
                quiet,
            };
            (State { work, ..state }, effects)
        }
        (View::Bead, work @ (Work::Unknown | Work::Idle { .. }))
        | (
            View::Graph | View::Usage,
            work @ (Work::Unknown | Work::Idle { .. } | Work::Active { .. }),
        ) => (State { work, ..state }, vec![]),
    }
}

fn turn(state: State, port: Port) -> (State, Vec<Effect>) {
    if state.port == port {
        return (state, vec![]);
    }
    (
        State { port, ..state },
        vec![Effect::Save, Effect::Draw(Refresh::Flash)],
    )
}

fn light(state: State, light: Light) -> (State, Vec<Effect>) {
    if state.light == light {
        return (state, vec![]);
    }
    let effects: Vec<Effect> = vec![
        Effect::Light(light.intensity()),
        Effect::Save,
        Effect::Draw(Refresh::Quiet),
    ];
    (State { light, ..state }, effects)
}

#[cfg(test)]
mod tests;
