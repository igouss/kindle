use eink::canvas::{BLACK, Canvas, Rect, WHITE};
use eink::text::Typeface;

use super::{Faces, MARGIN, SOFT};
use crate::menu;
use crate::reader::{self, Kind, Line, Page, Source, Weight, Word};
use crate::view::BeadPage;

const PAD: i32 = 14;
const BAR: i32 = 28;
const TRACK: u8 = 0xAA;
pub const WIDTH: u32 = 1024 - 2 * MARGIN as u32 - BAR as u32;
pub const HEIGHT: i32 = 758 - menu::HEADER as i32 - 2 * PAD;

pub struct Laid {
    pub lines: Vec<Line>,
    pub pages: Vec<Page>,
}

pub fn lay(faces: &Faces, source: &Source<'_>) -> Laid {
    let lines: Vec<Line> = reader::lay(faces, WIDTH, source);
    let pages: Vec<Page> = reader::paginate(&lines, HEIGHT);
    Laid { lines, pages }
}

pub(super) fn page(canvas: &mut Canvas, page: &BeadPage, faces: &Faces) {
    let top: i32 = menu::HEADER as i32;
    match page {
        BeadPage::Waiting => {
            faces
                .bold
                .draw(canvas, "Waiting for devbox", 40.0, MARGIN, top + 90, BLACK)
        }
        BeadPage::Idle => {
            faces
                .bold
                .draw(canvas, "Nothing in progress", 40.0, MARGIN, top + 90, BLACK)
        }
        BeadPage::Active {
            place,
            title,
            meta,
            description,
            elapsed,
            page,
        } => {
            let laid: Laid = lay(
                faces,
                &Source {
                    place,
                    title,
                    meta,
                    description,
                },
            );
            let shown: usize = (*page).min(laid.pages.len() - 1);
            let current: &Page = &laid.pages[shown];
            text(canvas, faces, &laid.lines[current.lines.clone()]);
            let status: String = if laid.pages.len() > 1 {
                scrollbar(canvas, &laid.pages, current);
                format!("{elapsed} \u{b7} {} of {}", shown + 1, laid.pages.len())
            } else {
                elapsed.clone()
            };
            let right: i32 = menu::BUTTON.left - 12 - faces.regular.width(&status, 24.0) as i32;
            faces.regular.draw(canvas, &status, 24.0, right, 44, WHITE);
        }
    }
}

fn text(canvas: &mut Canvas, faces: &Faces, lines: &[Line]) {
    let mut y: i32 = menu::HEADER as i32 + PAD;
    for (i, line) in lines.iter().enumerate() {
        let leading: i32 = line.height(i == 0);
        let left: i32 = MARGIN + line.indent;
        match line.kind {
            Kind::Rule => canvas.fill_rect(
                Rect {
                    left,
                    top: y + leading - 10,
                    width: WIDTH - line.indent as u32,
                    height: 2,
                },
                BLACK,
            ),
            Kind::Code => canvas.fill_rect(
                Rect {
                    left: left - 10,
                    top: y + leading - Kind::Code.metrics().leading,
                    width: 3,
                    height: Kind::Code.metrics().leading as u32,
                },
                TRACK,
            ),
            Kind::Title | Kind::Meta | Kind::Heading | Kind::Body => {}
        }
        let baseline: i32 = y + leading - leading_slack(line.kind);
        let size: f32 = line.kind.metrics().size;
        let ink: u8 = match line.kind {
            Kind::Meta => SOFT,
            Kind::Title | Kind::Heading | Kind::Body | Kind::Code | Kind::Rule => BLACK,
        };
        if let Some(marker) = &line.marker {
            faces.regular.draw(
                canvas,
                marker,
                size,
                left - reader::INDENT + 4,
                baseline,
                BLACK,
            );
        }
        words(canvas, faces, &line.words, size, left, baseline, ink);
        y += leading;
    }
}

fn leading_slack(kind: Kind) -> i32 {
    kind.metrics().leading / 4
}

fn words(
    canvas: &mut Canvas,
    faces: &Faces,
    words: &[Word],
    size: f32,
    left: i32,
    baseline: i32,
    ink: u8,
) {
    let space: i32 = faces.regular.width(" ", size) as i32;
    let mut x: i32 = left;
    for (i, word) in words.iter().enumerate() {
        let face: &Typeface = match word.weight {
            Weight::Regular => &faces.regular,
            Weight::Bold => &faces.bold,
        };
        if i > 0 && !word.glued {
            x += space;
        }
        face.draw(canvas, &word.text, size, x, baseline, ink);
        x += face.width(&word.text, size) as i32;
    }
}

fn scrollbar(canvas: &mut Canvas, pages: &[Page], current: &Page) {
    let top: i32 = menu::HEADER as i32 + PAD;
    let x: i32 = 1024 - MARGIN + 6;
    canvas.fill_rect(
        Rect {
            left: x + 2,
            top,
            width: 2,
            height: HEIGHT as u32,
        },
        TRACK,
    );
    let (start, length): (u32, u32) = reader::thumb(pages, current, HEIGHT as u32);
    canvas.fill_rect(
        Rect {
            left: x - 2,
            top: top + start as i32,
            width: 10,
            height: length,
        },
        BLACK,
    );
}
