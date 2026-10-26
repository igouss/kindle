use super::usage::*;
use super::*;
use crate::view::{DayRow, Meter, ModelRow};
use std::time::Duration;

const PAGE: Geometry = Geometry {
    width: 1024,
    height: 758,
    stride: 1024,
};

fn faces() -> Faces {
    Faces {
        regular: Typeface::regular().unwrap(),
        bold: Typeface::bold().unwrap(),
    }
}

fn full() -> Dashboard {
    Dashboard {
        title: String::from("Claude Code"),
        plan: String::from("Max 5x"),
        limits: (0..3)
            .map(|i: u32| Meter {
                title: format!("L{i}"),
                fraction: 0.5,
                resets_in: Some(Duration::from_secs(3600)),
            })
            .collect(),
        days: (0..7)
            .map(|i: u64| DayRow {
                label: format!("D{i}"),
                tokens: i * 100,
                today: i == 6,
            })
            .collect(),
        today: String::from("1 prompts \u{b7} 1 sessions"),
        models: (0..4)
            .map(|i: u64| ModelRow {
                name: format!("M{i}"),
                tokens: 400 - i * 100,
            })
            .collect(),
        updated: String::from("updated 19:39"),
    }
}

fn inked(canvas: &Canvas, xs: std::ops::Range<u32>, y: u32) -> bool {
    xs.into_iter()
        .any(|x: u32| canvas.pixel(x, y) != Some(WHITE))
}

fn pgm(canvas: &Canvas) -> Vec<u8> {
    let geometry: Geometry = canvas.geometry();
    let header: String = format!("P5\n{} {}\n255\n", geometry.stride, geometry.height);
    [header.as_bytes(), canvas.pixels()].concat()
}

fn golden(name: &str, canvas: &Canvas) {
    let path: std::path::PathBuf = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("golden")
        .join(name);
    let actual: Vec<u8> = pgm(canvas);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &actual).unwrap();
    }
    let expected: Vec<u8> = std::fs::read(&path).unwrap();
    let differing: usize = actual
        .iter()
        .zip(&expected)
        .filter(|(a, e): &(&u8, &u8)| a != e)
        .count();
    assert_eq!(
        (differing, actual.len()),
        (0, expected.len()),
        "UPDATE_GOLDEN=1 cargo test rewrites {}",
        path.display()
    );
}

fn menu_view(port: Port, on: bool) -> MenuView {
    MenuView {
        port,
        light: Light {
            level: 12,
            on,
            max: 25,
        },
    }
}

#[test]
fn a_full_dashboard_matches_the_golden_page() {
    golden(
        "full.pgm",
        &render(&Page::Usage(&full()), &faces(), PAGE, None),
    );
}

fn g3o(page: usize, description: &str) -> BeadPage {
    BeadPage::Active {
        place: String::from("omarchy-panorama \u{b7} omarchy-expose-g3o.13"),
        title: String::from(
            "Cutaway Street: hatching and street life are computed once, after the open motion, and ink in with the drawing",
        ),
        meta: String::from("P2 \u{b7} task \u{b7} elendal"),
        description: description.to_owned(),
        elapsed: String::from("42m"),
        page,
    }
}

const G3O: &str = include_str!("../../golden/g3o.md");

#[test]
fn the_first_page_of_a_long_bead_matches_the_golden_page() {
    golden(
        "bead.pgm",
        &render(&Page::Bead(&g3o(0, G3O)), &faces(), PAGE, None),
    );
}

#[test]
fn a_middle_page_of_a_long_bead_matches_the_golden_page() {
    golden(
        "bead-3.pgm",
        &render(&Page::Bead(&g3o(2, G3O)), &faces(), PAGE, None),
    );
}

#[test]
fn the_last_page_of_a_long_bead_matches_the_golden_page() {
    golden(
        "bead-6.pgm",
        &render(&Page::Bead(&g3o(5, G3O)), &faces(), PAGE, None),
    );
}

#[test]
fn code_quotes_lists_tables_rules_and_long_words_match_the_golden_page() {
    golden(
        "mixed.pgm",
        &render(
            &Page::Bead(&g3o(0, include_str!("../../golden/mixed.md"))),
            &faces(),
            PAGE,
            None,
        ),
    );
}

#[test]
fn a_long_description_runs_to_many_pages_and_a_short_one_to_one() {
    assert_eq!(
        (
            pages(&faces(), &g3o(0, G3O)),
            pages(&faces(), &g3o(0, "One line.")),
            pages(&faces(), &g3o(0, "")),
            pages(&faces(), &BeadPage::Idle)
        ),
        (6, 1, 1, 1)
    );
}

#[test]
fn a_page_past_the_end_shows_the_last_page() {
    let past: Canvas = render(&Page::Bead(&g3o(99, G3O)), &faces(), PAGE, None);
    let last: Canvas = render(&Page::Bead(&g3o(6, G3O)), &faces(), PAGE, None);
    assert!(past.pixels() == last.pixels());
}

#[test]
fn one_page_has_no_scroll_bar_and_many_pages_do() {
    let column = |canvas: &Canvas| -> bool {
        (menu::HEADER + 14..PAGE.height - 14).any(|y: u32| inked(canvas, 990..1000, y))
    };
    let one: Canvas = render(&Page::Bead(&g3o(0, "One line.")), &faces(), PAGE, None);
    let many: Canvas = render(&Page::Bead(&g3o(0, G3O)), &faces(), PAGE, None);
    assert_eq!((column(&one), column(&many)), (false, true));
}

#[test]
fn before_any_bead_push_the_bead_tab_waits_for_devbox() {
    golden(
        "waiting.pgm",
        &render(&Page::Bead(&BeadPage::Waiting), &faces(), PAGE, None),
    );
}

#[test]
fn with_nothing_in_progress_the_bead_tab_says_so() {
    golden(
        "idle.pgm",
        &render(&Page::Bead(&BeadPage::Idle), &faces(), PAGE, None),
    );
}

#[test]
fn the_open_menu_matches_the_golden_page() {
    golden(
        "menu.pgm",
        &render(
            &Page::Usage(&full()),
            &faces(),
            PAGE,
            Some(menu_view(Port::Right, true)),
        ),
    );
}

#[test]
fn the_chosen_side_is_filled_black_and_the_other_left_white() {
    let probe = |port: Port| -> [Option<u8>; 2] {
        let canvas: Canvas = render(
            &Page::Usage(&full()),
            &faces(),
            PAGE,
            Some(menu_view(port, true)),
        );
        let at = |target: Target| -> Option<u8> {
            let r: Rect = menu::TARGETS
                .iter()
                .find(|(t, _): &&(Target, Rect)| *t == target)
                .unwrap()
                .1;
            canvas.pixel((r.left + 6) as u32, (r.top + 6) as u32)
        };
        [at(Target::PortLeft), at(Target::PortRight)]
    };
    assert_eq!(
        (probe(Port::Left), probe(Port::Right)),
        ([Some(BLACK), Some(WHITE)], [Some(WHITE), Some(BLACK)])
    );
}

#[test]
fn the_light_switch_offers_the_opposite_state() {
    let switch: Rect = menu::TARGETS[3].1;
    let inked = |on: bool| -> usize {
        let canvas: Canvas = render(
            &Page::Usage(&full()),
            &faces(),
            PAGE,
            Some(menu_view(Port::Right, on)),
        );
        (switch.top as u32..switch.top as u32 + switch.height)
            .flat_map(|y: u32| {
                (switch.left as u32 + 4..switch.left as u32 + switch.width - 4)
                    .map(move |x: u32| (x, y))
            })
            .filter(|&(x, y): &(u32, u32)| canvas.pixel(x, y) != Some(WHITE))
            .count()
    };
    assert_ne!(inked(true), inked(false));
}

#[test]
fn the_menu_button_shows_three_white_bars_on_the_header() {
    let canvas: Canvas = render(&Page::Usage(&full()), &faces(), PAGE, None);
    let x: u32 = (menu::BUTTON.left + 45) as u32;
    let column: Vec<Option<u8>> = [21, 22, 27, 28, 35, 40, 48, 53, 54]
        .map(|y: u32| canvas.pixel(x, y))
        .to_vec();
    let expected: Vec<Option<u8>> = [
        BLACK, WHITE, WHITE, BLACK, WHITE, WHITE, WHITE, WHITE, BLACK,
    ]
    .map(Some)
    .to_vec();
    assert_eq!(column, expected);
}

#[test]
fn the_columns_split_the_page_around_a_gutter_inside_the_margins() {
    assert_eq!(
        columns(PAGE),
        (
            Column {
                left: 40,
                right: 492
            },
            Column {
                left: 532,
                right: 984
            }
        )
    );
}

#[test]
fn a_share_of_nothing_is_nothing() {
    assert_eq!((share(100, 5, 0), share(100, 0, 9)), (0, 0));
}

#[test]
fn the_peak_fills_the_room_and_half_fills_half() {
    assert_eq!(
        (share(100, 9, 9), share(100, 50, 100), share(100, 12, 9)),
        (100, 50, 100)
    );
}

#[test]
fn a_meter_fills_its_fraction_clamped_to_the_track() {
    assert_eq!(
        (
            filled(452, 0.0),
            filled(452, 0.33),
            filled(452, 1.7),
            filled(452, -0.2)
        ),
        (0, 149, 452, 0)
    );
}

#[test]
fn both_columns_end_above_the_bottom_margin() {
    let canvas: Canvas = render(&Page::Usage(&full()), &faces(), PAGE, None);
    let last = |xs: std::ops::Range<u32>| -> u32 {
        (0..PAGE.height)
            .rev()
            .find(|&y: &u32| inked(&canvas, xs.clone(), y))
            .unwrap()
    };
    let (left, right): (u32, u32) = (last(0..512), last(512..PAGE.width));
    assert!(
        left < PAGE.height - 40 && right < PAGE.height - 40,
        "last inked rows {left}, {right}"
    );
}

#[test]
fn the_gutter_stays_blank_below_the_header() {
    let canvas: Canvas = render(&Page::Usage(&full()), &faces(), PAGE, None);
    assert!(!(menu::HEADER..PAGE.height).any(|y: u32| inked(&canvas, 493..532, y)));
}

#[test]
fn the_header_is_black_where_no_text_falls() {
    let canvas: Canvas = render(&Page::Usage(&full()), &faces(), PAGE, None);
    let probes: [Option<u8>; 3] = [
        canvas.pixel(5, 5),
        canvas.pixel(1023, 69),
        canvas.pixel(5, 70),
    ];
    assert_eq!(probes, [Some(BLACK), Some(BLACK), Some(WHITE)]);
}

#[test]
fn a_half_used_meter_is_black_to_its_middle_and_track_gray_after() {
    let canvas: Canvas = render(&Page::Usage(&full()), &faces(), PAGE, None);
    let y: u32 = TOP as u32 + 48 + METER_HEIGHT / 2;
    let probes: [Option<u8>; 5] = [
        canvas.pixel(40, y),
        canvas.pixel(40 + 225, y),
        canvas.pixel(40 + 226, y),
        canvas.pixel(491, y),
        canvas.pixel(492, y),
    ];
    assert_eq!(
        probes,
        [
            Some(BLACK),
            Some(BLACK),
            Some(TRACK),
            Some(TRACK),
            Some(WHITE)
        ]
    );
}

#[test]
fn todays_bar_is_black_and_the_busiest_day_fills_the_room() {
    let canvas: Canvas = render(&Page::Usage(&full()), &faces(), PAGE, None);
    let y: u32 = (TOP + 36 + 15 + 6 * DAY_ROW + 20) as u32;
    let end: u32 = (984 - DAY_TOKENS) as u32;
    assert_eq!(
        (canvas.pixel(end - 1, y), canvas.pixel(end, y)),
        (Some(BLACK), Some(WHITE))
    );
}

#[test]
fn a_quiet_day_draws_no_bar_and_a_busier_one_a_gray_bar() {
    let canvas: Canvas = render(&Page::Usage(&full()), &faces(), PAGE, None);
    let y: u32 = (TOP + 36 + 15 + 20) as u32;
    let x: u32 = (532 + DAY_LABEL) as u32 + 5;
    assert_eq!(
        (canvas.pixel(x, y), canvas.pixel(x, y + DAY_ROW as u32)),
        (Some(WHITE), Some(QUIET_DAY))
    );
}

#[test]
fn the_heaviest_model_bar_spans_the_column_and_lighter_ones_scale_down() {
    let canvas: Canvas = render(&Page::Usage(&full()), &faces(), PAGE, None);
    let first: u32 = (TOP + 3 * LIMIT_BLOCK + 50 + 15 + 2) as u32;
    let fourth: u32 = first + 3 * MODEL_ROW as u32;
    let probes: [Option<u8>; 4] = [
        canvas.pixel(491, first),
        canvas.pixel(492, first),
        canvas.pixel(40 + 112, fourth),
        canvas.pixel(40 + 114, fourth),
    ];
    assert_eq!(
        probes,
        [Some(MODEL_BAR), Some(WHITE), Some(MODEL_BAR), Some(WHITE)]
    );
}

#[test]
fn a_meter_without_a_reset_prints_no_countdown() {
    let mut view: Dashboard = full();
    view.limits[0].resets_in = None;
    let canvas: Canvas = render(&Page::Usage(&view), &faces(), PAGE, None);
    let rows: std::ops::Range<u32> = TOP as u32 + 70..(TOP + LIMIT_BLOCK) as u32;
    let first: bool = rows.clone().any(|y: u32| inked(&canvas, 0..512, y));
    let second: bool = rows
        .map(|y: u32| y + LIMIT_BLOCK as u32)
        .any(|y: u32| inked(&canvas, 0..512, y));
    assert_eq!((first, second), (false, true));
}

#[test]
fn a_rule_inside_a_quote_ends_at_the_text_edge() {
    let canvas: Canvas = render(&Page::Bead(&g3o(0, "> a\n>\n> ---")), &faces(), PAGE, None);
    let edge: u32 = MARGIN as u32 + bead::WIDTH;
    assert!(!(menu::HEADER..PAGE.height).any(|y: u32| inked(&canvas, edge + 2..edge + 30, y)));
}

#[test]
fn a_real_epic_on_the_graph_tab_matches_the_golden_page() {
    let epic: crate::bead::Epic =
        serde_json::from_str(include_str!("../../golden/g3o-epic.json")).unwrap();
    golden(
        "graph.pgm",
        &render(&Page::Graph(Some(&epic)), &faces(), PAGE, None),
    );
}

#[test]
fn before_any_epic_the_graph_tab_says_so() {
    golden(
        "no-epic.pgm",
        &render(&Page::Graph(None), &faces(), PAGE, None),
    );
}
