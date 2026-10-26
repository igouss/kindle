use eink::canvas::{BLACK, Canvas, Geometry, Rect};
use eink::text::Typeface;

use super::{BANNER, Column, Faces, banner, columns};
use crate::view::{Dashboard, DayRow, Meter, ModelRow, format_duration, format_tokens};

pub(super) const TOP: i32 = BANNER as i32 + 10;
pub(super) const LIMIT_BLOCK: i32 = 110;
pub(super) const METER_HEIGHT: u32 = 20;
pub(super) const DAY_ROW: i32 = 54;
pub(super) const DAY_LABEL: i32 = 90;
pub(super) const DAY_TOKENS: i32 = 120;
pub(super) const MODEL_ROW: i32 = 44;
pub(super) const TRACK: u8 = 0xAA;
pub(super) const QUIET_DAY: u8 = 0x66;
pub(super) const MODEL_BAR: u8 = 0xCC;

pub(super) fn page(view: &Dashboard, faces: &Faces, geometry: Geometry) -> Canvas {
    let mut canvas: Canvas = Canvas::new(geometry);
    let (left, right): (Column, Column) = columns(geometry);
    banner(&mut canvas, view, faces);

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
