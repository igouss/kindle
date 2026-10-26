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

fn state(menu: Menu, level: u32, on: bool) -> State {
    State {
        view: View::Usage,
        menu,
        port: Port::Right,
        light: Light { level, on, max: 25 },
        feed: None,
        work: Work::Unknown,
        epic: None,
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
        step(state(Menu::Closed, 12, true), button()),
        (
            state(Menu::Open, 12, true),
            vec![Effect::Draw(Refresh::Quiet)]
        )
    );
}

#[test]
fn on_the_dashboard_a_tap_anywhere_else_does_nothing() {
    let tapped: (State, Vec<Effect>) = step(state(Menu::Closed, 12, true), on(Target::PortLeft));
    assert_eq!(tapped, (state(Menu::Closed, 12, true), vec![]));
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
        .map(|e: Event| step(state(Menu::Open, 12, true), e))
        .collect();
    let expected = || {
        (
            state(Menu::Closed, 12, true),
            vec![Effect::Draw(Refresh::Quiet)],
        )
    };
    assert_eq!(closed, vec![expected(), expected(), expected()]);
}

#[test]
fn a_tap_between_menu_buttons_does_nothing() {
    let tapped: (State, Vec<Effect>) = step(
        state(Menu::Open, 12, true),
        Event::Tap(Point { x: 794, y: 200 }),
    );
    assert_eq!(tapped, (state(Menu::Open, 12, true), vec![]));
}

#[test]
fn choosing_the_other_side_turns_the_page_with_a_flash_and_saves() {
    let (next, effects): (State, Vec<Effect>) =
        step(state(Menu::Open, 12, true), on(Target::PortLeft));
    assert_eq!(
        (next.port, next.menu, effects),
        (
            Port::Left,
            Menu::Open,
            vec![Effect::Save, Effect::Draw(Refresh::Flash)]
        )
    );
}

#[test]
fn choosing_the_current_side_does_nothing() {
    assert_eq!(
        step(state(Menu::Open, 12, true), on(Target::PortRight)),
        (state(Menu::Open, 12, true), vec![])
    );
}

#[test]
fn brighter_and_dimmer_step_the_light_and_set_it() {
    let brighter: (State, Vec<Effect>) = step(state(Menu::Open, 12, true), on(Target::Brighter));
    let dimmer: (State, Vec<Effect>) = step(state(Menu::Open, 12, true), on(Target::Dimmer));
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
            (state(Menu::Open, 15, true), effects(15)),
            (state(Menu::Open, 9, true), effects(9))
        )
    );
}

#[test]
fn the_light_stops_at_one_and_at_its_maximum() {
    let floor: (State, Vec<Effect>) = step(state(Menu::Open, 2, true), on(Target::Dimmer));
    let ceiling: (State, Vec<Effect>) = step(state(Menu::Open, 24, true), on(Target::Brighter));
    assert_eq!((floor.0.light.level, ceiling.0.light.level), (1, 25));
}

#[test]
fn at_a_limit_a_further_step_does_nothing() {
    assert_eq!(
        step(state(Menu::Open, 25, true), on(Target::Brighter)),
        (state(Menu::Open, 25, true), vec![])
    );
}

#[test]
fn switching_off_keeps_the_level_and_switching_on_restores_it() {
    let off: (State, Vec<Effect>) = step(state(Menu::Open, 12, true), on(Target::LightSwitch));
    let back: (State, Vec<Effect>) = step(state(Menu::Open, 12, false), on(Target::LightSwitch));
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
            (state(Menu::Open, 12, false), effects(0)),
            (state(Menu::Open, 12, true), effects(12))
        )
    );
}

#[test]
fn a_step_while_off_switches_the_light_on() {
    assert_eq!(
        step(state(Menu::Open, 12, false), on(Target::Brighter))
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
        state(Menu::Open, 12, true),
        Event::Pushed(feed(), Refresh::Flash),
    );
    assert_eq!(
        (next.menu, next.feed, effects),
        (Menu::Open, Some(feed()), vec![Effect::Draw(Refresh::Flash)])
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
        ..state(Menu::Closed, 12, false)
    };
    assert_eq!(
        (summary(&left), summary(&state(Menu::Closed, 12, true))),
        (
            String::from("drawn, USB left, light 0"),
            String::from("drawn, USB right, light 12")
        )
    );
}

fn at(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
}

fn bead(id: &str) -> Bead {
    Bead {
        project: String::from("kindle"),
        id: id.to_owned(),
        title: format!("title of {id}"),
        priority: 1,
        issue_type: String::from("task"),
        assignee: None,
        started: at("2026-10-26T09:00:00-04:00"),
        description: String::new(),
    }
}

const PAGES: usize = 5;

fn sighting(bead: Option<Bead>) -> Event {
    Event::BeadPushed(
        BeadFeed {
            epic: None,
            bead,
            now: at("2026-10-26T09:30:00-04:00"),
        },
        PAGES,
    )
}

fn active(id: &str, page: usize) -> Work {
    reading(id, page, PAGES, 0)
}

fn reading(id: &str, page: usize, pages: usize, quiet: u32) -> Work {
    Work::Active {
        bead: bead(id),
        now: at("2026-10-26T09:30:00-04:00"),
        page,
        pages,
        quiet,
    }
}

fn on_bead(work: Work) -> State {
    State {
        view: View::Bead,
        work,
        ..state(Menu::Closed, 12, true)
    }
}

fn tap_at(x: i32) -> Event {
    Event::Tap(Point { x, y: 400 })
}

const BACK: i32 = 10;
const FORWARD: i32 = 900;

fn on_tab(view: View) -> Event {
    let rect: Rect = menu::TABS
        .iter()
        .find(|(v, _): &&(View, Rect)| *v == view)
        .unwrap()
        .1;
    Event::Tap(centre(rect))
}

#[test]
fn a_first_bead_opens_the_bead_tab_at_its_first_page_and_closes_the_menu() {
    let (next, effects): (State, Vec<Effect>) =
        step(state(Menu::Open, 12, true), sighting(Some(bead("a"))));
    assert_eq!(
        (next.view, next.menu, next.work, effects),
        (
            View::Bead,
            Menu::Closed,
            active("a", 0),
            vec![Effect::Draw(Refresh::Flash)]
        )
    );
}

#[test]
fn the_same_bead_again_keeps_its_page_and_redraws_quietly_only_where_it_shows() {
    let reading: State = State {
        view: View::Bead,
        work: active("a", 3),
        ..state(Menu::Closed, 12, true)
    };
    let elsewhere: State = State {
        view: View::Usage,
        work: active("a", 3),
        ..state(Menu::Closed, 12, true)
    };
    let (seen, seen_effects): (State, Vec<Effect>) = step(reading, sighting(Some(bead("a"))));
    let (unseen, unseen_effects): (State, Vec<Effect>) = step(elsewhere, sighting(Some(bead("a"))));
    assert_eq!(
        (
            seen.work,
            seen_effects,
            unseen.view,
            unseen.work,
            unseen_effects
        ),
        (
            active("a", 3),
            vec![Effect::Draw(Refresh::Quiet)],
            View::Usage,
            active("a", 3),
            vec![]
        )
    );
}

#[test]
fn the_same_bead_again_replaces_what_was_known_of_it() {
    let reading: State = State {
        view: View::Bead,
        work: active("a", 3),
        ..state(Menu::Closed, 12, true)
    };
    let edited: Bead = Bead {
        title: String::from("a new title"),
        ..bead("a")
    };
    let later: Event = Event::BeadPushed(
        BeadFeed {
            epic: None,
            bead: Some(edited.clone()),
            now: at("2026-10-26T10:00:00-04:00"),
        },
        PAGES,
    );
    assert_eq!(
        step(reading, later).0.work,
        Work::Active {
            bead: edited,
            now: at("2026-10-26T10:00:00-04:00"),
            page: 3,
            pages: PAGES,
            quiet: 0
        }
    );
}

#[test]
fn a_shorter_description_keeps_the_reader_on_its_last_page() {
    let shorter: Event = Event::BeadPushed(
        BeadFeed {
            epic: None,
            bead: Some(bead("a")),
            now: at("2026-10-26T09:30:00-04:00"),
        },
        2,
    );
    assert_eq!(
        step(on_bead(reading("a", 4, PAGES, 0)), shorter).0.work,
        reading("a", 1, 2, 0)
    );
}

#[test]
fn a_bead_with_no_pages_counts_as_one() {
    let empty: Event = Event::BeadPushed(
        BeadFeed {
            epic: None,
            bead: Some(bead("a")),
            now: at("2026-10-26T09:30:00-04:00"),
        },
        0,
    );
    let (opened, _): (State, Vec<Effect>) = step(state(Menu::Closed, 12, true), empty);
    let (turned, effects): (State, Vec<Effect>) = step(opened, tap_at(FORWARD));
    assert_eq!((turned.work, effects), (reading("a", 0, 1, 0), vec![]));
}

#[test]
fn forward_and_back_turn_one_page_quietly() {
    let (forward, forward_effects): (State, Vec<Effect>) =
        step(on_bead(reading("a", 2, PAGES, 0)), tap_at(FORWARD));
    let (back, back_effects): (State, Vec<Effect>) =
        step(on_bead(reading("a", 2, PAGES, 0)), tap_at(BACK));
    assert_eq!(
        (forward.work, forward_effects, back.work, back_effects),
        (
            reading("a", 3, PAGES, 1),
            vec![Effect::Draw(Refresh::Quiet)],
            reading("a", 1, PAGES, 1),
            vec![Effect::Draw(Refresh::Quiet)]
        )
    );
}

#[test]
fn the_first_and_last_pages_stop_the_turn_and_count_nothing() {
    let first: (State, Vec<Effect>) = step(on_bead(reading("a", 0, PAGES, 2)), tap_at(BACK));
    let last: (State, Vec<Effect>) =
        step(on_bead(reading("a", PAGES - 1, PAGES, 2)), tap_at(FORWARD));
    assert_eq!(
        (first, last),
        (
            (on_bead(reading("a", 0, PAGES, 2)), vec![]),
            (on_bead(reading("a", PAGES - 1, PAGES, 2)), vec![])
        )
    );
}

#[test]
fn every_sixth_turn_flashes_and_starts_counting_again() {
    let (fifth, fifth_effects): (State, Vec<Effect>) = step(
        on_bead(reading("a", 1, PAGES, FLASH_EVERY - 2)),
        tap_at(FORWARD),
    );
    let (sixth, sixth_effects): (State, Vec<Effect>) = step(fifth, tap_at(FORWARD));
    assert_eq!(
        (fifth_effects, sixth.work, sixth_effects),
        (
            vec![Effect::Draw(Refresh::Quiet)],
            reading("a", 3, PAGES, 0),
            vec![Effect::Draw(Refresh::Flash)]
        )
    );
}

#[test]
fn page_taps_do_nothing_on_the_usage_tab_or_without_a_bead() {
    let usage: State = State {
        view: View::Usage,
        work: active("a", 2),
        ..state(Menu::Closed, 12, true)
    };
    let idle: State = on_bead(Work::Idle {
        now: at("2026-10-26T09:30:00-04:00"),
    });
    let expected_usage: State = State {
        view: View::Usage,
        work: active("a", 2),
        ..state(Menu::Closed, 12, true)
    };
    assert_eq!(
        (step(usage, tap_at(FORWARD)), step(idle, tap_at(BACK))),
        (
            (expected_usage, vec![]),
            (
                on_bead(Work::Idle {
                    now: at("2026-10-26T09:30:00-04:00")
                }),
                vec![]
            )
        )
    );
}

#[test]
fn with_the_menu_open_a_page_tap_only_closes_the_menu() {
    let open: State = State {
        menu: Menu::Open,
        ..on_bead(active("a", 2))
    };
    assert_eq!(
        step(open, tap_at(BACK)),
        (on_bead(active("a", 2)), vec![Effect::Draw(Refresh::Quiet)])
    );
}

#[test]
fn a_different_bead_starts_again_at_its_first_page_on_the_bead_tab() {
    let reading: State = State {
        view: View::Usage,
        work: active("a", 3),
        ..state(Menu::Closed, 12, true)
    };
    let (next, effects): (State, Vec<Effect>) = step(reading, sighting(Some(bead("b"))));
    assert_eq!(
        (next.view, next.work, effects),
        (
            View::Bead,
            active("b", 0),
            vec![Effect::Draw(Refresh::Flash)]
        )
    );
}

#[test]
fn nothing_in_progress_makes_the_work_idle_and_keeps_the_tab() {
    let reading: State = State {
        view: View::Bead,
        work: active("a", 3),
        ..state(Menu::Closed, 12, true)
    };
    let (closed, effects): (State, Vec<Effect>) = step(reading, sighting(None));
    let (never, never_effects): (State, Vec<Effect>) =
        step(state(Menu::Closed, 12, true), sighting(None));
    assert_eq!(
        (
            closed.view,
            closed.work,
            effects,
            never.view,
            never.work,
            never_effects
        ),
        (
            View::Bead,
            Work::Idle {
                now: at("2026-10-26T09:30:00-04:00")
            },
            vec![Effect::Draw(Refresh::Quiet)],
            View::Usage,
            Work::Idle {
                now: at("2026-10-26T09:30:00-04:00")
            },
            vec![]
        )
    );
}

#[test]
fn a_usage_push_redraws_only_the_usage_tab() {
    let reading: State = State {
        view: View::Bead,
        ..state(Menu::Closed, 12, true)
    };
    let (next, effects): (State, Vec<Effect>) =
        step(reading, Event::Pushed(feed(), Refresh::Flash));
    assert_eq!((next.feed, effects), (Some(feed()), vec![]));
}

#[test]
fn a_tab_shows_its_view_with_a_flash_and_the_shown_tab_does_nothing() {
    let (bead_tab, effects): (State, Vec<Effect>) =
        step(state(Menu::Closed, 12, true), on_tab(View::Bead));
    let same: (State, Vec<Effect>) = step(state(Menu::Closed, 12, true), on_tab(View::Usage));
    assert_eq!(
        (bead_tab.view, effects, same),
        (
            View::Bead,
            vec![Effect::Draw(Refresh::Flash)],
            (state(Menu::Closed, 12, true), vec![])
        )
    );
}

#[test]
fn leaving_the_bead_tab_and_coming_back_keeps_the_page() {
    let reading: State = State {
        view: View::Bead,
        work: active("a", 3),
        ..state(Menu::Closed, 12, true)
    };
    let (away, _): (State, Vec<Effect>) = step(reading, on_tab(View::Usage));
    let (back, _): (State, Vec<Effect>) = step(away, on_tab(View::Bead));
    assert_eq!((back.view, back.work), (View::Bead, active("a", 3)));
}

#[test]
fn with_the_menu_open_a_tab_only_closes_the_menu() {
    assert_eq!(
        step(state(Menu::Open, 12, true), on_tab(View::Bead)),
        (
            state(Menu::Closed, 12, true),
            vec![Effect::Draw(Refresh::Quiet)]
        )
    );
}

#[test]
fn exit_to_kindle_leaves_and_nothing_else() {
    assert_eq!(
        step(state(Menu::Open, 12, true), on(Target::KindleUi)),
        (state(Menu::Open, 12, true), vec![Effect::Exit])
    );
}

#[test]
fn every_bead_push_replaces_the_known_epic_even_when_nothing_is_in_progress() {
    let epic = |status: crate::bead::Status| crate::bead::Epic {
        id: String::from("e"),
        title: String::from("epic"),
        beads: vec![crate::bead::Member {
            id: String::from("a"),
            title: String::from("a"),
            status,
        }],
        blocks: vec![],
    };
    let push = |bead: Option<Bead>, status: crate::bead::Status| {
        Event::BeadPushed(
            BeadFeed {
                bead,
                epic: Some(epic(status)),
                now: at("2026-10-26T09:30:00-04:00"),
            },
            PAGES,
        )
    };
    let (working, _): (State, Vec<Effect>) = step(
        state(Menu::Closed, 12, true),
        push(Some(bead("a")), crate::bead::Status::InProgress),
    );
    let (closed, _): (State, Vec<Effect>) = step(working, push(None, crate::bead::Status::Closed));
    assert_eq!(closed.epic, Some(epic(crate::bead::Status::Closed)));
}
