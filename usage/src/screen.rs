use eink::canvas::{BLACK, Canvas, Geometry, Rect, WHITE};
use eink::text::Typeface;

use crate::view::{Dashboard, DayRow, Meter, ModelRow, format_duration, format_tokens};

const MARGIN: i32 = 40;
const BANNER: u32 = 130;
const LIMIT_BLOCK: i32 = 100;
const METER_HEIGHT: u32 = 20;
const RULE: u32 = 2;
const DAY_ROW: i32 = 36;
const DAY_LABEL: i32 = 110;
const MODEL_ROW: i32 = 40;
const TRACK: u8 = 0xAA;
const MODEL_BAR: u8 = 0xCC;

pub struct Faces {
    pub regular: Typeface,
    pub bold: Typeface,
}

pub fn render(view: &Dashboard, faces: &Faces, geometry: Geometry) -> Canvas {
    let mut canvas: Canvas = Canvas::new(geometry);
    let right: i32 = geometry.width as i32 - MARGIN;
    banner(&mut canvas, view, faces, right);
    let mut y: i32 = BANNER as i32 + 10;
    for meter in &view.limits {
        limit(&mut canvas, meter, faces, y, right);
        y += LIMIT_BLOCK;
    }
    y += 45;
    faces
        .bold
        .draw(&mut canvas, "Tokens by day", 30.0, MARGIN, y, BLACK);
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
        MARGIN + DAY_LABEL,
        y + 28,
        BLACK,
    );
    y += 78;
    faces
        .bold
        .draw(&mut canvas, "Tokens by model", 30.0, MARGIN, y, BLACK);
    y += 15;
    let heaviest: u64 = view.models.first().map_or(0, |m: &ModelRow| m.tokens);
    for row in &view.models {
        model(&mut canvas, row, heaviest, faces, y, right);
        y += MODEL_ROW;
    }
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
    canvas.fill_rect(
        Rect {
            left: 0,
            top: 0,
            width: canvas.geometry().width,
            height: BANNER,
        },
        BLACK,
    );
    faces
        .bold
        .draw(canvas, &view.title, 56.0, MARGIN, 70, WHITE);
    faces
        .regular
        .draw(canvas, &view.plan, 30.0, MARGIN, 112, WHITE);
    let updated: i32 = right - faces.regular.width(&view.updated, 24.0) as i32;
    faces
        .regular
        .draw(canvas, &view.updated, 24.0, updated, 112, WHITE);
}

fn limit(canvas: &mut Canvas, meter: &Meter, faces: &Faces, top: i32, right: i32) {
    faces
        .bold
        .draw(canvas, &meter.title, 30.0, MARGIN, top + 36, BLACK);
    let percent: String = format!("{}%", (meter.fraction * 100.0).round() as i64);
    let left: i32 = right - faces.bold.width(&percent, 30.0) as i32;
    faces
        .bold
        .draw(canvas, &percent, 30.0, left, top + 36, BLACK);
    let track: Rect = Rect {
        left: MARGIN,
        top: top + 48,
        width: (right - MARGIN) as u32,
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
        faces.regular.draw(
            canvas,
            &format!("Resets in {}", format_duration(remaining)),
            24.0,
            MARGIN,
            top + 92,
            BLACK,
        );
    }
}

fn day(canvas: &mut Canvas, row: &DayRow, peak: u64, faces: &Faces, top: i32, right: i32) {
    let face: &Typeface = if row.today {
        &faces.bold
    } else {
        &faces.regular
    };
    let tokens: String = format_tokens(row.tokens);
    let tokens_left: i32 = right - face.width(&tokens, 26.0) as i32;
    face.draw(canvas, &row.label, 26.0, MARGIN, top + 26, BLACK);
    face.draw(canvas, &tokens, 26.0, tokens_left, top + 26, BLACK);
    let bar_left: i32 = MARGIN + DAY_LABEL;
    let room: u32 = (right - 120 - bar_left).max(0) as u32;
    let bar: Rect = Rect {
        left: bar_left,
        top: top + 8,
        width: share(room, row.tokens, peak),
        height: 20,
    };
    canvas.fill_rect(bar, if row.today { BLACK } else { 0x66 });
}

fn model(canvas: &mut Canvas, row: &ModelRow, heaviest: u64, faces: &Faces, top: i32, right: i32) {
    let room: u32 = (right - MARGIN) as u32;
    canvas.fill_rect(
        Rect {
            left: MARGIN,
            top,
            width: share(room, row.tokens, heaviest),
            height: (MODEL_ROW - 6) as u32,
        },
        MODEL_BAR,
    );
    canvas.fill_rect(
        Rect {
            left: MARGIN,
            top: top + MODEL_ROW - 6,
            width: room,
            height: RULE / 2,
        },
        TRACK,
    );
    let tokens: String = format_tokens(row.tokens);
    faces
        .regular
        .draw(canvas, &row.name, 26.0, MARGIN + 10, top + 27, BLACK);
    faces.regular.draw(
        canvas,
        &tokens,
        26.0,
        right - 10 - faces.regular.width(&tokens, 26.0) as i32,
        top + 27,
        BLACK,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const PW2: Geometry = Geometry {
        width: 758,
        height: 1024,
        stride: 768,
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

    fn row_inked(canvas: &Canvas, y: u32) -> bool {
        (0..PW2.width).any(|x: u32| canvas.pixel(x, y) != Some(WHITE))
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
        let actual: Vec<u8> = pgm(&render(&full(), &faces(), PW2));
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
                filled(678, 0.0),
                filled(678, 0.33),
                filled(678, 1.7),
                filled(678, -0.2)
            ),
            (0, 224, 678, 0)
        );
    }

    #[test]
    fn a_full_dashboard_fits_the_panel_with_its_bottom_margin_blank() {
        let canvas: Canvas = render(&full(), &faces(), PW2);
        let last: u32 = (0..PW2.height)
            .rev()
            .find(|&y: &u32| row_inked(&canvas, y))
            .unwrap();
        assert!(
            (990..PW2.height - 10).contains(&last),
            "last inked row {last}"
        );
    }

    #[test]
    fn the_banner_is_black_where_no_text_falls() {
        let canvas: Canvas = render(&full(), &faces(), PW2);
        assert_eq!(
            (
                canvas.pixel(5, 5),
                canvas.pixel(757, 129),
                canvas.pixel(5, 130)
            ),
            (Some(BLACK), Some(BLACK), Some(WHITE))
        );
    }

    #[test]
    fn a_half_used_meter_is_black_to_its_middle_and_track_gray_after() {
        let canvas: Canvas = render(&full(), &faces(), PW2);
        let y: u32 = BANNER + 10 + 48 + METER_HEIGHT / 2;
        let probes: [Option<u8>; 4] = [
            canvas.pixel(40, y),
            canvas.pixel(40 + 338, y),
            canvas.pixel(40 + 340, y),
            canvas.pixel(717, y),
        ];
        assert_eq!(probes, [Some(BLACK), Some(BLACK), Some(TRACK), Some(TRACK)]);
    }

    #[test]
    fn todays_bar_is_black_and_the_busiest_day_fills_the_room() {
        let canvas: Canvas = render(&full(), &faces(), PW2);
        let top: u32 = BANNER + 10 + 3 * LIMIT_BLOCK as u32 + 45 + 15 + 6 * DAY_ROW as u32 + 18;
        let bar_end: u32 =
            (MARGIN + DAY_LABEL) as u32 + (718 - 120 - (MARGIN + DAY_LABEL)) as u32 - 1;
        assert_eq!(
            (canvas.pixel(bar_end, top), canvas.pixel(bar_end + 1, top)),
            (Some(BLACK), Some(WHITE))
        );
    }

    #[test]
    fn a_quiet_day_draws_no_bar() {
        let canvas: Canvas = render(&full(), &faces(), PW2);
        let top: u32 = BANNER + 10 + 3 * LIMIT_BLOCK as u32 + 45 + 15 + 18;
        assert_eq!(
            canvas.pixel((MARGIN + DAY_LABEL) as u32 + 5, top),
            Some(WHITE)
        );
    }

    #[test]
    fn the_heaviest_model_bar_spans_the_row_and_lighter_ones_scale_down() {
        let canvas: Canvas = render(&full(), &faces(), PW2);
        let first: u32 =
            BANNER + 10 + 3 * LIMIT_BLOCK as u32 + 45 + 15 + 7 * DAY_ROW as u32 + 78 + 15 + 2;
        let fourth: u32 = first + 3 * MODEL_ROW as u32;
        let probes: [Option<u8>; 3] = [
            canvas.pixel(716, first),
            canvas.pixel(40 + 168, fourth),
            canvas.pixel(40 + 171, fourth),
        ];
        assert_eq!(probes, [Some(MODEL_BAR), Some(MODEL_BAR), Some(WHITE)]);
    }

    #[test]
    fn a_meter_without_a_reset_prints_no_countdown() {
        let mut view: Dashboard = full();
        view.limits[0].resets_in = None;
        let canvas: Canvas = render(&view, &faces(), PW2);
        let rows: std::ops::Range<u32> = BANNER + 10 + 70..BANNER + 10 + LIMIT_BLOCK as u32;
        assert!(!rows.clone().any(|y: u32| row_inked(&canvas, y)));
        assert!(
            rows.map(|y: u32| y + LIMIT_BLOCK as u32)
                .any(|y: u32| row_inked(&canvas, y))
        );
    }
}
