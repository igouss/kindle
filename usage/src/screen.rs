use eink::canvas::{BLACK, Canvas, Geometry, Rect, WHITE};
use eink::text::Typeface;

use crate::view::{Dashboard, DayRow, Meter, ModelRow, format_duration, format_tokens};

const MARGIN: i32 = 40;
const GUTTER: i32 = 40;
const BANNER: u32 = 110;
const TOP: i32 = BANNER as i32 + 10;
const LIMIT_BLOCK: i32 = 110;
const METER_HEIGHT: u32 = 20;
const DAY_ROW: i32 = 54;
const DAY_LABEL: i32 = 90;
const DAY_TOKENS: i32 = 120;
const MODEL_ROW: i32 = 44;
const TRACK: u8 = 0xAA;
const QUIET_DAY: u8 = 0x66;
const MODEL_BAR: u8 = 0xCC;

pub struct Faces {
    pub regular: Typeface,
    pub bold: Typeface,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Column {
    pub left: i32,
    pub right: i32,
}

impl Column {
    fn width(self) -> u32 {
        (self.right - self.left).max(0) as u32
    }
}

pub fn columns(geometry: Geometry) -> (Column, Column) {
    let middle: i32 = geometry.width as i32 / 2;
    (
        Column {
            left: MARGIN,
            right: middle - GUTTER / 2,
        },
        Column {
            left: middle + GUTTER / 2,
            right: geometry.width as i32 - MARGIN,
        },
    )
}

pub fn render(view: &Dashboard, faces: &Faces, geometry: Geometry) -> Canvas {
    let mut canvas: Canvas = Canvas::new(geometry);
    let (left, right): (Column, Column) = columns(geometry);
    banner(&mut canvas, view, faces, right.right);

    let mut y: i32 = TOP;
    for meter in &view.limits {
        limit(&mut canvas, meter, faces, y, left);
        y += LIMIT_BLOCK;
    }
    y += 50;
    faces
        .bold
        .draw(&mut canvas, "Tokens by model", 30.0, left.left, y, BLACK);
    y += 15;
    let heaviest: u64 = view.models.first().map_or(0, |m: &ModelRow| m.tokens);
    for row in &view.models {
        model(&mut canvas, row, heaviest, faces, y, left);
        y += MODEL_ROW;
    }

    let mut y: i32 = TOP + 36;
    faces
        .bold
        .draw(&mut canvas, "Tokens by day", 30.0, right.left, y, BLACK);
    y += 15;
    let peak: u64 = view
        .days
        .iter()
        .map(|d: &DayRow| d.tokens)
        .max()
        .unwrap_or(0);
    for row in &view.days {
        day(&mut canvas, row, peak, faces, y, right);
        y += DAY_ROW;
    }
    faces.regular.draw(
        &mut canvas,
        &view.today,
        24.0,
        right.left + DAY_LABEL,
        y + 20,
        BLACK,
    );
    canvas
}

pub fn share(width: u32, part: u64, whole: u64) -> u32 {
    if whole == 0 {
        0
    } else {
        (u128::from(width) * u128::from(part.min(whole)) / u128::from(whole)) as u32
    }
}

pub fn filled(width: u32, fraction: f64) -> u32 {
    (f64::from(width) * fraction.clamp(0.0, 1.0)).round() as u32
}

fn banner(canvas: &mut Canvas, view: &Dashboard, faces: &Faces, right: i32) {
    let width: u32 = canvas.geometry().width;
    canvas.fill_rect(
        Rect {
            left: 0,
            top: 0,
            width,
            height: BANNER,
        },
        BLACK,
    );
    faces
        .bold
        .draw(canvas, &view.title, 52.0, MARGIN, 64, WHITE);
    faces
        .regular
        .draw(canvas, &view.plan, 28.0, MARGIN, 98, WHITE);
    let updated: i32 = right - faces.regular.width(&view.updated, 24.0) as i32;
    faces
        .regular
        .draw(canvas, &view.updated, 24.0, updated, 98, WHITE);
}

fn limit(canvas: &mut Canvas, meter: &Meter, faces: &Faces, top: i32, column: Column) {
    faces
        .bold
        .draw(canvas, &meter.title, 30.0, column.left, top + 36, BLACK);
    let percent: String = format!("{}%", (meter.fraction * 100.0).round() as i64);
    let left: i32 = column.right - faces.bold.width(&percent, 30.0) as i32;
    faces
        .bold
        .draw(canvas, &percent, 30.0, left, top + 36, BLACK);
    let track: Rect = Rect {
        left: column.left,
        top: top + 48,
        width: column.width(),
        height: METER_HEIGHT,
    };
    canvas.fill_rect(track, TRACK);
    canvas.fill_rect(
        Rect {
            width: filled(track.width, meter.fraction),
            ..track
        },
        BLACK,
    );
    if let Some(remaining) = meter.resets_in {
        let text: String = format!("Resets in {}", format_duration(remaining));
        faces
            .regular
            .draw(canvas, &text, 24.0, column.left, top + 92, BLACK);
    }
}

fn day(canvas: &mut Canvas, row: &DayRow, peak: u64, faces: &Faces, top: i32, column: Column) {
    let face: &Typeface = if row.today {
        &faces.bold
    } else {
        &faces.regular
    };
    let tokens: String = format_tokens(row.tokens);
    let tokens_left: i32 = column.right - face.width(&tokens, 26.0) as i32;
    face.draw(canvas, &row.label, 26.0, column.left, top + 30, BLACK);
    face.draw(canvas, &tokens, 26.0, tokens_left, top + 30, BLACK);
    let bar_left: i32 = column.left + DAY_LABEL;
    let room: u32 = (column.right - DAY_TOKENS - bar_left).max(0) as u32;
    let bar: Rect = Rect {
        left: bar_left,
        top: top + 12,
        width: share(room, row.tokens, peak),
        height: 22,
    };
    canvas.fill_rect(bar, if row.today { BLACK } else { QUIET_DAY });
}

fn model(
    canvas: &mut Canvas,
    row: &ModelRow,
    heaviest: u64,
    faces: &Faces,
    top: i32,
    column: Column,
) {
    let bar: Rect = Rect {
        left: column.left,
        top,
        width: share(column.width(), row.tokens, heaviest),
        height: (MODEL_ROW - 6) as u32,
    };
    canvas.fill_rect(bar, MODEL_BAR);
    canvas.fill_rect(
        Rect {
            left: column.left,
            top: top + MODEL_ROW - 6,
            width: column.width(),
            height: 1,
        },
        TRACK,
    );
    let tokens: String = format_tokens(row.tokens);
    faces
        .regular
        .draw(canvas, &row.name, 26.0, column.left + 10, top + 27, BLACK);
    let tokens_left: i32 = column.right - 10 - faces.regular.width(&tokens, 26.0) as i32;
    faces
        .regular
        .draw(canvas, &tokens, 26.0, tokens_left, top + 27, BLACK);
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn a_full_dashboard_matches_the_golden_page() {
        let golden: std::path::PathBuf =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("golden/full.pgm");
        let actual: Vec<u8> = pgm(&render(&full(), &faces(), PAGE));
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            std::fs::write(&golden, &actual).unwrap();
        }
        let expected: Vec<u8> = std::fs::read(&golden).unwrap();
        let differing: usize = actual
            .iter()
            .zip(&expected)
            .filter(|(a, e): &(&u8, &u8)| a != e)
            .count();
        assert_eq!(
            (differing, actual.len()),
            (0, expected.len()),
            "UPDATE_GOLDEN=1 cargo test rewrites {}",
            golden.display()
        );
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
        let canvas: Canvas = render(&full(), &faces(), PAGE);
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
    fn the_gutter_stays_blank_below_the_banner() {
        let canvas: Canvas = render(&full(), &faces(), PAGE);
        assert!(!(BANNER..PAGE.height).any(|y: u32| inked(&canvas, 493..532, y)));
    }

    #[test]
    fn the_banner_is_black_where_no_text_falls() {
        let canvas: Canvas = render(&full(), &faces(), PAGE);
        let probes: [Option<u8>; 3] = [
            canvas.pixel(5, 5),
            canvas.pixel(1023, 109),
            canvas.pixel(5, 110),
        ];
        assert_eq!(probes, [Some(BLACK), Some(BLACK), Some(WHITE)]);
    }

    #[test]
    fn a_half_used_meter_is_black_to_its_middle_and_track_gray_after() {
        let canvas: Canvas = render(&full(), &faces(), PAGE);
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
        let canvas: Canvas = render(&full(), &faces(), PAGE);
        let y: u32 = (TOP + 36 + 15 + 6 * DAY_ROW + 20) as u32;
        let end: u32 = (984 - DAY_TOKENS) as u32;
        assert_eq!(
            (canvas.pixel(end - 1, y), canvas.pixel(end, y)),
            (Some(BLACK), Some(WHITE))
        );
    }

    #[test]
    fn a_quiet_day_draws_no_bar_and_a_busier_one_a_gray_bar() {
        let canvas: Canvas = render(&full(), &faces(), PAGE);
        let y: u32 = (TOP + 36 + 15 + 20) as u32;
        let x: u32 = (532 + DAY_LABEL) as u32 + 5;
        assert_eq!(
            (canvas.pixel(x, y), canvas.pixel(x, y + DAY_ROW as u32)),
            (Some(WHITE), Some(QUIET_DAY))
        );
    }

    #[test]
    fn the_heaviest_model_bar_spans_the_column_and_lighter_ones_scale_down() {
        let canvas: Canvas = render(&full(), &faces(), PAGE);
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
        let canvas: Canvas = render(&view, &faces(), PAGE);
        let rows: std::ops::Range<u32> = TOP as u32 + 70..(TOP + LIMIT_BLOCK) as u32;
        let first: bool = rows.clone().any(|y: u32| inked(&canvas, 0..512, y));
        let second: bool = rows
            .map(|y: u32| y + LIMIT_BLOCK as u32)
            .any(|y: u32| inked(&canvas, 0..512, y));
        assert_eq!((first, second), (false, true));
    }
}
