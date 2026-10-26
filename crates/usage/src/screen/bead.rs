use eink::canvas::{BLACK, Canvas, Rect};

use super::{Faces, MARGIN, SOFT};
use crate::menu;
use crate::view::BeadPage;

const TITLE_LEADING: i32 = 50;

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
        BeadPage::Active { place, title, meta } => {
            let width: u32 = canvas.geometry().width - 2 * MARGIN as u32;
            faces
                .regular
                .draw(canvas, place, 24.0, MARGIN, top + 40, SOFT);
            let mut y: i32 = top + 40;
            for line in faces.bold.wrap(title, 40.0, width) {
                y += TITLE_LEADING;
                faces.bold.draw(canvas, &line, 40.0, MARGIN, y, BLACK);
            }
            faces.regular.draw(canvas, meta, 26.0, MARGIN, y + 42, SOFT);
            canvas.fill_rect(
                Rect {
                    left: MARGIN,
                    top: y + 60,
                    width,
                    height: 2,
                },
                BLACK,
            );
        }
    }
}
