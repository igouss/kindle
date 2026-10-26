use eink::canvas::{BLACK, Canvas, Geometry, Port, Rect, WHITE};
use eink::text::Typeface;

use crate::app::{Light, View};
use crate::bead::Epic;
use crate::menu::{self, Target};
use crate::view::{BeadPage, Dashboard};

mod bead;
mod usage;

const MARGIN: i32 = 40;
const GUTTER: i32 = 40;
const SOFT: u8 = 0x55;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuView {
    pub port: Port,
    pub light: Light,
}

pub enum Page<'a> {
    Bead(&'a BeadPage),
    Graph(Option<&'a Epic>),
    Usage(&'a Dashboard),
}

pub fn render(
    page: &Page<'_>,
    faces: &Faces,
    geometry: Geometry,
    menu: Option<MenuView>,
) -> Canvas {
    let mut canvas: Canvas = Canvas::new(geometry);
    match page {
        Page::Bead(bead) => {
            header(&mut canvas, faces, View::Bead);
            bead::page(&mut canvas, bead, faces);
        }
        Page::Graph(epic) => {
            header(&mut canvas, faces, View::Graph);
            graph_page(&mut canvas, *epic, faces);
        }
        Page::Usage(view) => {
            header(&mut canvas, faces, View::Usage);
            usage::page(&mut canvas, view, faces);
        }
    }
    if let Some(menu) = menu {
        panel(&mut canvas, menu, faces);
    }
    canvas
}

fn graph_page(canvas: &mut Canvas, epic: Option<&Epic>, faces: &Faces) {
    let top: i32 = menu::HEADER as i32;
    match epic {
        Some(epic) => {
            faces
                .regular
                .draw(canvas, &epic.title, 24.0, MARGIN, top + 36, SOFT);
            let area: Rect = Rect {
                left: 0,
                top: top + 48,
                width: canvas.geometry().width,
                height: canvas.geometry().height - (top + 48) as u32,
            };
            crate::graph::draw(canvas, faces, epic, area);
        }
        None => faces
            .bold
            .draw(canvas, "No epic yet", 40.0, MARGIN, top + 90, BLACK),
    }
}

fn header(canvas: &mut Canvas, faces: &Faces, shown: View) {
    let width: u32 = canvas.geometry().width;
    canvas.fill_rect(
        Rect {
            left: 0,
            top: 0,
            width,
            height: menu::HEADER,
        },
        BLACK,
    );
    for (view, rect) in menu::TABS {
        let label: &str = match view {
            View::Bead => "Bead",
            View::Graph => "Graph",
            View::Usage => "Usage",
        };
        let ink: u8 = if view == shown {
            canvas.fill_rect(
                Rect {
                    top: rect.top + 10,
                    height: rect.height - 20,
                    ..rect
                },
                WHITE,
            );
            BLACK
        } else {
            WHITE
        };
        let left: i32 = rect.left + (rect.width as i32 - faces.bold.width(label, 30.0) as i32) / 2;
        faces.bold.draw(canvas, label, 30.0, left, 46, ink);
    }
    let bar_left: i32 = menu::BUTTON.left + 25;
    for row in 0..3 {
        let bar: Rect = Rect {
            left: bar_left,
            top: 22 + 13 * row,
            width: 40,
            height: 6,
        };
        canvas.fill_rect(bar, WHITE);
    }
}

fn panel(canvas: &mut Canvas, view: MenuView, faces: &Faces) {
    let frame: Rect = menu::PANEL;
    canvas.fill_rect(frame, BLACK);
    let inner: Rect = Rect {
        left: frame.left + 3,
        top: frame.top + 3,
        width: frame.width - 6,
        height: frame.height - 6,
    };
    canvas.fill_rect(inner, WHITE);
    let left: i32 = menu::TARGETS[0].1.left;
    faces.bold.draw(
        canvas,
        "Orientation",
        28.0,
        left,
        menu::ORIENTATION_BASELINE,
        BLACK,
    );
    faces
        .bold
        .draw(canvas, "Light", 28.0, left, menu::LIGHT_BASELINE, BLACK);
    let level: String = if view.light.on {
        format!("{} of {}", view.light.level, view.light.max)
    } else {
        String::from("off")
    };
    let right: i32 = frame.left + frame.width as i32 - 20;
    let level_left: i32 = right - faces.regular.width(&level, 26.0) as i32;
    faces.regular.draw(
        canvas,
        &level,
        26.0,
        level_left,
        menu::LIGHT_BASELINE,
        BLACK,
    );
    for (target, rect) in menu::TARGETS {
        let (label, chosen): (&str, bool) = match target {
            Target::PortLeft => ("USB left", view.port == Port::Left),
            Target::PortRight => ("USB right", view.port == Port::Right),
            Target::Dimmer => ("\u{2212}", false),
            Target::LightSwitch => (if view.light.on { "Off" } else { "On" }, false),
            Target::Brighter => ("+", false),
            Target::Close => ("Close", false),
            Target::KindleUi => ("Exit to Kindle", false),
        };
        button(canvas, faces, rect, label, chosen);
    }
}

fn button(canvas: &mut Canvas, faces: &Faces, rect: Rect, label: &str, chosen: bool) {
    let (paper, ink): (u8, u8) = if chosen {
        (BLACK, WHITE)
    } else {
        (WHITE, BLACK)
    };
    canvas.fill_rect(rect, BLACK);
    let inner: Rect = Rect {
        left: rect.left + 2,
        top: rect.top + 2,
        width: rect.width - 4,
        height: rect.height - 4,
    };
    canvas.fill_rect(inner, paper);
    let left: i32 = rect.left + (rect.width as i32 - faces.bold.width(label, 30.0) as i32) / 2;
    faces.bold.draw(
        canvas,
        label,
        30.0,
        left,
        rect.top + rect.height as i32 / 2 + 11,
        ink,
    );
}

pub fn pages(faces: &Faces, page: &BeadPage) -> usize {
    match page {
        BeadPage::Active {
            place,
            title,
            meta,
            description,
            ..
        } => bead::lay(
            faces,
            &crate::reader::Source {
                place,
                title,
                meta,
                description,
            },
        )
        .pages
        .len(),
        BeadPage::Waiting | BeadPage::Idle => 1,
    }
}

#[cfg(test)]
mod tests;
