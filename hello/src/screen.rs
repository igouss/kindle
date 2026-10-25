use std::time::Duration;

use eink::canvas::{BLACK, Canvas, Geometry, Rect, WHITE};
use eink::text::Typeface;

pub const LEVELS: u32 = 16;
const MARGIN: i32 = 40;
const BANNER: u32 = 150;
const SWATCH_TOP: i32 = 560;
const SWATCH_HEIGHT: u32 = 160;

pub struct Facts {
    pub geometry: Geometry,
    pub uptime: Duration,
}

pub fn level_gray(level: u32) -> u8 {
    (level.min(LEVELS - 1) * 255 / (LEVELS - 1)) as u8
}

pub fn swatch(geometry: Geometry, level: u32) -> Rect {
    let width: u32 = geometry.width.saturating_sub(2 * MARGIN as u32) / LEVELS;
    let left: i32 = (geometry.width - width * LEVELS) as i32 / 2;
    Rect {
        left: left + (level * width) as i32,
        top: SWATCH_TOP,
        width,
        height: SWATCH_HEIGHT,
    }
}

pub fn format_uptime(uptime: Duration) -> String {
    let minutes: u64 = uptime.as_secs() / 60;
    let (days, hours, minutes): (u64, u64, u64) = (minutes / 1440, minutes / 60 % 24, minutes % 60);
    if days > 0 {
        format!("{days}d {hours:02}h {minutes:02}m")
    } else {
        format!("{hours}h {minutes:02}m")
    }
}

pub fn render(facts: &Facts, face: &Typeface) -> Canvas {
    let geometry: Geometry = facts.geometry;
    let mut canvas: Canvas = Canvas::new(geometry);
    canvas.fill_rect(
        Rect {
            left: 0,
            top: 0,
            width: geometry.width,
            height: BANNER,
        },
        BLACK,
    );
    face.draw(&mut canvas, "Rust on Kindle", 72.0, MARGIN, 100, WHITE);

    let lines: [String; 4] = [
        String::from("armv7-unknown-linux-musleabi, static"),
        format!(
            "framebuffer {}\u{d7}{}, stride {}",
            geometry.width, geometry.height, geometry.stride
        ),
        format!("uptime {}", format_uptime(facts.uptime)),
        format!("{} gray levels:", LEVELS),
    ];
    for (row, line) in lines.iter().enumerate() {
        face.draw(
            &mut canvas,
            line,
            36.0,
            MARGIN,
            250 + 70 * row as i32,
            BLACK,
        );
    }

    for level in 0..LEVELS {
        canvas.fill_rect(swatch(geometry, level), level_gray(level));
    }
    let frame: Rect = swatch(geometry, 0);
    let outline: Rect = Rect {
        left: frame.left - 2,
        top: frame.top - 2,
        width: frame.width * LEVELS + 4,
        height: 2,
    };
    canvas.fill_rect(outline, BLACK);
    canvas.fill_rect(
        Rect {
            top: frame.top + frame.height as i32,
            ..outline
        },
        BLACK,
    );

    let footer: &str = "drawn by /mnt/us/kindle-dev/hello";
    let left: i32 = (geometry.width as i32 - face.width(footer, 28.0) as i32) / 2;
    face.draw(
        &mut canvas,
        footer,
        28.0,
        left,
        geometry.height as i32 - MARGIN,
        BLACK,
    );
    canvas
}

#[cfg(test)]
mod tests {
    use super::*;

    const PW2: Geometry = Geometry {
        width: 758,
        height: 1024,
        stride: 768,
    };

    #[test]
    fn the_levels_run_from_black_to_white_in_even_steps() {
        let grays: Vec<u8> = (0..LEVELS).map(level_gray).collect();
        assert_eq!(
            grays,
            vec![
                0, 17, 34, 51, 68, 85, 102, 119, 136, 153, 170, 187, 204, 221, 238, 255
            ]
        );
    }

    #[test]
    fn a_level_past_the_last_is_white() {
        assert_eq!(level_gray(LEVELS), WHITE);
    }

    #[test]
    fn the_swatches_tile_the_row_centred_within_the_margins() {
        let first: Rect = swatch(PW2, 0);
        let last: Rect = swatch(PW2, LEVELS - 1);
        let right_gap: i32 = PW2.width as i32 - (last.left + last.width as i32);
        assert_eq!(
            (
                first.width,
                last.left - first.left,
                first.left - right_gap <= 1,
                first.left >= MARGIN
            ),
            (42, 630, true, true)
        );
    }

    #[test]
    fn each_swatch_is_painted_its_level() {
        let face: Typeface = Typeface::embedded().unwrap();
        let canvas: Canvas = render(
            &Facts {
                geometry: PW2,
                uptime: Duration::ZERO,
            },
            &face,
        );
        let centres: Vec<Option<u8>> = (0..LEVELS)
            .map(|level: u32| swatch(PW2, level))
            .map(|r: Rect| {
                canvas.pixel(
                    (r.left + r.width as i32 / 2) as u32,
                    (r.top + r.height as i32 / 2) as u32,
                )
            })
            .collect();
        assert_eq!(
            centres,
            (0..LEVELS)
                .map(|level: u32| Some(level_gray(level)))
                .collect::<Vec<Option<u8>>>()
        );
    }

    #[test]
    fn the_banner_is_black_where_no_title_falls() {
        let face: Typeface = Typeface::embedded().unwrap();
        let canvas: Canvas = render(
            &Facts {
                geometry: PW2,
                uptime: Duration::ZERO,
            },
            &face,
        );
        assert_eq!(
            (
                canvas.pixel(5, 5),
                canvas.pixel(757, 149),
                canvas.pixel(5, 150)
            ),
            (Some(BLACK), Some(BLACK), Some(WHITE))
        );
    }

    fn rendered() -> Canvas {
        let face: Typeface = Typeface::embedded().unwrap();
        render(
            &Facts {
                geometry: PW2,
                uptime: Duration::ZERO,
            },
            &face,
        )
    }

    fn inked_in(canvas: &Canvas, rows: std::ops::Range<u32>) -> Vec<(u32, u32)> {
        rows.flat_map(|y: u32| (0..PW2.width).map(move |x: u32| (x, y)))
            .filter(|&(x, y): &(u32, u32)| canvas.pixel(x, y) != Some(WHITE))
            .collect()
    }

    #[test]
    fn each_text_line_sits_on_its_own_baseline() {
        let canvas: Canvas = rendered();
        let baselines: [u32; 4] = [250, 320, 390, 460];
        let ink: Vec<(u32, u32)> = inked_in(&canvas, BANNER..SWATCH_TOP as u32 - 2);
        let on_a_line: bool = ink
            .iter()
            .all(|&(_, y): &(u32, u32)| baselines.iter().any(|&b: &u32| y + 36 >= b && y < b + 12));
        let every_line: Vec<bool> = baselines
            .iter()
            .map(|&b: &u32| ink.iter().any(|&(_, y): &(u32, u32)| y + 36 >= b && y < b))
            .collect();
        assert_eq!((on_a_line, every_line), (true, vec![true; 4]));
    }

    #[test]
    fn the_swatch_row_is_ruled_above_and_below_and_nowhere_else() {
        let canvas: Canvas = rendered();
        let frame: Rect = swatch(PW2, 0);
        let (left, right): (u32, u32) = (
            (frame.left - 2) as u32,
            (frame.left - 3) as u32 + frame.width * LEVELS + 4,
        );
        let (top, bottom): (u32, u32) = (frame.top as u32 - 2, frame.top as u32 + frame.height);
        let rules: Vec<Option<u8>> = vec![
            canvas.pixel(left, top),
            canvas.pixel(right, top + 1),
            canvas.pixel(left, bottom),
            canvas.pixel(right, bottom + 1),
            canvas.pixel(left - 1, top),
            canvas.pixel(right + 1, top),
            canvas.pixel(left, top - 1),
            canvas.pixel(left, bottom + 2),
        ];
        let expected: Vec<Option<u8>> = [BLACK, BLACK, BLACK, BLACK, WHITE, WHITE, WHITE, WHITE]
            .map(Some)
            .to_vec();
        assert_eq!(rules, expected);
    }

    #[test]
    fn the_footer_is_centred_above_the_bottom_margin() {
        let canvas: Canvas = rendered();
        let ink: Vec<(u32, u32)> =
            inked_in(&canvas, SWATCH_TOP as u32 + SWATCH_HEIGHT + 2..PW2.height);
        let columns: Vec<u32> = ink.iter().map(|&(x, _): &(u32, u32)| x).collect();
        let rows: Vec<u32> = ink.iter().map(|&(_, y): &(u32, u32)| y).collect();
        let (first, last): (u32, u32) = (
            *columns.iter().min().unwrap(),
            *columns.iter().max().unwrap(),
        );
        let (top, bottom): (u32, u32) = (*rows.iter().min().unwrap(), *rows.iter().max().unwrap());
        let baseline: u32 = PW2.height - MARGIN as u32;
        assert_eq!(
            (
                first.abs_diff(PW2.width - 1 - last) <= 6,
                top + 28 >= baseline,
                bottom < baseline + 8,
                bottom + 8 >= baseline
            ),
            (true, true, true, true)
        );
    }

    #[test]
    fn under_a_minute_of_uptime_reads_as_zero() {
        assert_eq!(format_uptime(Duration::from_secs(59)), "0h 00m");
    }

    #[test]
    fn one_hour_one_minute_of_uptime() {
        assert_eq!(format_uptime(Duration::from_secs(3660)), "1h 01m");
    }

    #[test]
    fn days_of_uptime_carry_hours_and_minutes() {
        assert_eq!(
            format_uptime(Duration::from_secs(2 * 86400 + 3 * 3600 + 4 * 60 + 5)),
            "2d 03h 04m"
        );
    }
}
