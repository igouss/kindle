use std::time::Duration;

use chrono::{DateTime, FixedOffset, TimeDelta};
use eink::canvas::{Point, Port};
use eink::epdc::Refresh;

use crate::bead::Bead;
use crate::menu::{self, Hit, Target};
use crate::record::Record;
use crate::settings::Settings;

pub const LIGHT_STEP: u32 = 3;
pub const DEFAULT_LEVEL: u32 = 12;

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
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Bead,
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
}

#[derive(Debug, PartialEq)]
pub enum Event {
    Tap(Point),
    Pushed(Feed, Refresh),
    BeadPushed(BeadFeed),
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
        Event::BeadPushed(feed) => sighted(state, feed),
        Event::Tap(point) => tap(state, menu::hit(point)),
    }
}

fn shown(state: &State, view: View, refresh: Refresh) -> Vec<Effect> {
    if state.view == view {
        vec![Effect::Draw(refresh)]
    } else {
        vec![]
    }
}

fn sighted(state: State, feed: BeadFeed) -> (State, Vec<Effect>) {
    let BeadFeed { bead, now } = feed;
    let kept: Option<usize> = match (&state.work, &bead) {
        (
            Work::Active {
                bead: was, page, ..
            },
            Some(is),
        ) if was.id == is.id => Some(*page),
        (Work::Active { .. } | Work::Idle { .. } | Work::Unknown, _) => None,
    };
    match (bead, kept) {
        (Some(bead), Some(page)) => {
            let effects: Vec<Effect> = shown(&state, View::Bead, Refresh::Quiet);
            let work: Work = Work::Active { bead, now, page };
            (State { work, ..state }, effects)
        }
        (Some(bead), None) => (
            State {
                view: View::Bead,
                menu: Menu::Closed,
                work: Work::Active { bead, now, page: 0 },
                ..state
            },
            vec![Effect::Draw(Refresh::Flash)],
        ),
        (None, _) => {
            let effects: Vec<Effect> = shown(&state, View::Bead, Refresh::Quiet);
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
        (Menu::Closed, Hit::Target(_) | Hit::Panel | Hit::Outside) => (state, vec![]),
        (Menu::Open, Hit::Button | Hit::Tab(_) | Hit::Outside | Hit::Target(Target::Close)) => (
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
