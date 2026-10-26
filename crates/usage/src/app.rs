use std::time::Duration;

use chrono::{DateTime, FixedOffset, TimeDelta};
use eink::canvas::{Point, Port};
use eink::epdc::Refresh;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Dashboard,
    Menu,
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
    pub mode: Mode,
    pub port: Port,
    pub light: Light,
    pub feed: Option<Feed>,
}

#[derive(Debug, PartialEq)]
pub enum Event {
    Tap(Point),
    Pushed(Feed, Refresh),
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
        Event::Pushed(feed, refresh) => (
            State {
                feed: Some(feed),
                ..state
            },
            vec![Effect::Draw(refresh)],
        ),
        Event::Tap(point) => tap(state, menu::hit(point)),
    }
}

fn tap(state: State, hit: Hit) -> (State, Vec<Effect>) {
    let current: Light = state.light;
    match (state.mode, hit) {
        (Mode::Dashboard, Hit::Button) => (
            State {
                mode: Mode::Menu,
                ..state
            },
            vec![Effect::Draw(Refresh::Quiet)],
        ),
        (Mode::Dashboard, Hit::Target(_) | Hit::Panel | Hit::Outside) => (state, vec![]),
        (Mode::Menu, Hit::Button | Hit::Outside | Hit::Target(Target::Close)) => (
            State {
                mode: Mode::Dashboard,
                ..state
            },
            vec![Effect::Draw(Refresh::Quiet)],
        ),
        (Mode::Menu, Hit::Panel) => (state, vec![]),
        (Mode::Menu, Hit::Target(Target::KindleUi)) => (state, vec![Effect::Exit]),
        (Mode::Menu, Hit::Target(Target::PortLeft)) => turn(state, Port::Left),
        (Mode::Menu, Hit::Target(Target::PortRight)) => turn(state, Port::Right),
        (Mode::Menu, Hit::Target(Target::Dimmer)) => light(state, current.stepped(false)),
        (Mode::Menu, Hit::Target(Target::Brighter)) => light(state, current.stepped(true)),
        (Mode::Menu, Hit::Target(Target::LightSwitch)) => {
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
mod tests {
    use super::*;
    use eink::canvas::Rect;

    fn centre(rect: Rect) -> Point {
        Point {
            x: rect.left + rect.width as i32 / 2,
            y: rect.top + rect.height as i32 / 2,
        }
    }

    fn on(target: Target) -> Event {
        let rect: Rect = menu::TARGETS
            .iter()
            .find(|(t, _): &&(Target, Rect)| *t == target)
            .unwrap()
            .1;
        Event::Tap(centre(rect))
    }

    fn button() -> Event {
        Event::Tap(centre(menu::BUTTON))
    }

    fn state(mode: Mode, level: u32, on: bool) -> State {
        State {
            mode,
            port: Port::Right,
            light: Light { level, on, max: 25 },
            feed: None,
        }
    }

    fn feed() -> Feed {
        Feed {
            record: Record::default(),
            now: DateTime::parse_from_rfc3339("2026-10-25T20:00:00-04:00").unwrap(),
        }
    }

    #[test]
    fn the_button_opens_the_menu() {
        assert_eq!(
            step(state(Mode::Dashboard, 12, true), button()),
            (
                state(Mode::Menu, 12, true),
                vec![Effect::Draw(Refresh::Quiet)]
            )
        );
    }

    #[test]
    fn on_the_dashboard_a_tap_anywhere_else_does_nothing() {
        let tapped: (State, Vec<Effect>) =
            step(state(Mode::Dashboard, 12, true), on(Target::PortLeft));
        assert_eq!(tapped, (state(Mode::Dashboard, 12, true), vec![]));
    }

    #[test]
    fn the_button_close_or_a_tap_outside_closes_the_menu() {
        let closers: [Event; 3] = [
            button(),
            on(Target::Close),
            Event::Tap(Point { x: 100, y: 400 }),
        ];
        let closed: Vec<(State, Vec<Effect>)> = closers
            .into_iter()
            .map(|e: Event| step(state(Mode::Menu, 12, true), e))
            .collect();
        let expected = || {
            (
                state(Mode::Dashboard, 12, true),
                vec![Effect::Draw(Refresh::Quiet)],
            )
        };
        assert_eq!(closed, vec![expected(), expected(), expected()]);
    }

    #[test]
    fn a_tap_between_menu_buttons_does_nothing() {
        let tapped: (State, Vec<Effect>) = step(
            state(Mode::Menu, 12, true),
            Event::Tap(Point { x: 794, y: 200 }),
        );
        assert_eq!(tapped, (state(Mode::Menu, 12, true), vec![]));
    }

    #[test]
    fn choosing_the_other_side_turns_the_page_with_a_flash_and_saves() {
        let (next, effects): (State, Vec<Effect>) =
            step(state(Mode::Menu, 12, true), on(Target::PortLeft));
        assert_eq!(
            (next.port, next.mode, effects),
            (
                Port::Left,
                Mode::Menu,
                vec![Effect::Save, Effect::Draw(Refresh::Flash)]
            )
        );
    }

    #[test]
    fn choosing_the_current_side_does_nothing() {
        assert_eq!(
            step(state(Mode::Menu, 12, true), on(Target::PortRight)),
            (state(Mode::Menu, 12, true), vec![])
        );
    }

    #[test]
    fn brighter_and_dimmer_step_the_light_and_set_it() {
        let brighter: (State, Vec<Effect>) =
            step(state(Mode::Menu, 12, true), on(Target::Brighter));
        let dimmer: (State, Vec<Effect>) = step(state(Mode::Menu, 12, true), on(Target::Dimmer));
        let effects = |level: u32| {
            vec![
                Effect::Light(level),
                Effect::Save,
                Effect::Draw(Refresh::Quiet),
            ]
        };
        assert_eq!(
            (brighter, dimmer),
            (
                (state(Mode::Menu, 15, true), effects(15)),
                (state(Mode::Menu, 9, true), effects(9))
            )
        );
    }

    #[test]
    fn the_light_stops_at_one_and_at_its_maximum() {
        let floor: (State, Vec<Effect>) = step(state(Mode::Menu, 2, true), on(Target::Dimmer));
        let ceiling: (State, Vec<Effect>) = step(state(Mode::Menu, 24, true), on(Target::Brighter));
        assert_eq!((floor.0.light.level, ceiling.0.light.level), (1, 25));
    }

    #[test]
    fn at_a_limit_a_further_step_does_nothing() {
        assert_eq!(
            step(state(Mode::Menu, 25, true), on(Target::Brighter)),
            (state(Mode::Menu, 25, true), vec![])
        );
    }

    #[test]
    fn switching_off_keeps_the_level_and_switching_on_restores_it() {
        let off: (State, Vec<Effect>) = step(state(Mode::Menu, 12, true), on(Target::LightSwitch));
        let back: (State, Vec<Effect>) =
            step(state(Mode::Menu, 12, false), on(Target::LightSwitch));
        let effects = |level: u32| {
            vec![
                Effect::Light(level),
                Effect::Save,
                Effect::Draw(Refresh::Quiet),
            ]
        };
        assert_eq!(
            (off, back),
            (
                (state(Mode::Menu, 12, false), effects(0)),
                (state(Mode::Menu, 12, true), effects(12))
            )
        );
    }

    #[test]
    fn a_step_while_off_switches_the_light_on() {
        assert_eq!(
            step(state(Mode::Menu, 12, false), on(Target::Brighter))
                .0
                .light,
            Light {
                level: 15,
                on: true,
                max: 25
            }
        );
    }

    #[test]
    fn a_push_stores_the_feed_and_redraws_whatever_is_showing() {
        let (next, effects): (State, Vec<Effect>) = step(
            state(Mode::Menu, 12, true),
            Event::Pushed(feed(), Refresh::Flash),
        );
        assert_eq!(
            (next.mode, next.feed, effects),
            (Mode::Menu, Some(feed()), vec![Effect::Draw(Refresh::Flash)])
        );
    }

    #[test]
    fn with_nothing_saved_the_light_starts_where_powerd_has_it() {
        let lit: Light = initial_light(Settings::default(), 24, 25);
        let dark: Light = initial_light(Settings::default(), 0, 25);
        assert_eq!(
            (lit, dark),
            (
                Light {
                    level: 24,
                    on: true,
                    max: 25
                },
                Light {
                    level: DEFAULT_LEVEL,
                    on: false,
                    max: 25
                }
            )
        );
    }

    #[test]
    fn saved_light_wins_over_powerd_and_is_clamped_to_its_range() {
        let saved = |level: u32, on: bool| Settings {
            port: None,
            level: Some(level),
            on: Some(on),
        };
        let lights: [Light; 3] = [
            initial_light(saved(9, false), 24, 25),
            initial_light(saved(40, true), 0, 25),
            initial_light(saved(0, true), 24, 25),
        ];
        let expected: [Light; 3] = [
            Light {
                level: 9,
                on: false,
                max: 25,
            },
            Light {
                level: 25,
                on: true,
                max: 25,
            },
            Light {
                level: 1,
                on: true,
                max: 25,
            },
        ];
        assert_eq!(lights, expected);
    }

    #[test]
    fn the_page_clock_runs_on_from_the_last_push() {
        let later: DateTime<FixedOffset> = feed().now_after(Duration::from_secs(90));
        assert_eq!(later.to_rfc3339(), "2026-10-25T20:01:30-04:00");
    }

    #[test]
    fn the_reply_names_the_side_and_the_light_actually_set() {
        let left: State = State {
            port: Port::Left,
            ..state(Mode::Dashboard, 12, false)
        };
        assert_eq!(
            (summary(&left), summary(&state(Mode::Dashboard, 12, true))),
            (
                String::from("drawn, USB left, light 0"),
                String::from("drawn, USB right, light 12")
            )
        );
    }

    #[test]
    fn exit_to_kindle_leaves_and_nothing_else() {
        assert_eq!(
            step(state(Mode::Menu, 12, true), on(Target::KindleUi)),
            (state(Mode::Menu, 12, true), vec![Effect::Exit])
        );
    }
}
