use eink::canvas::{BLACK, Canvas, Geometry, Port, Rect, WHITE};
use eink::text::Typeface;

use crate::app::Light;
use crate::menu::{self, Target};
use crate::view::Dashboard;

mod usage;

const MARGIN: i32 = 40;
const GUTTER: i32 = 40;
pub(super) const BANNER: u32 = 110;

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

pub fn render(
    view: &Dashboard,
    faces: &Faces,
    geometry: Geometry,
    menu: Option<MenuView>,
) -> Canvas {
    let mut canvas: Canvas = usage::page(view, faces, geometry);
    if let Some(menu) = menu {
        panel(&mut canvas, menu, faces);
    }
    canvas
}

pub(super) fn banner(canvas: &mut Canvas, view: &Dashboard, faces: &Faces) {
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
    let updated: i32 = menu::BUTTON.left - 16 - faces.regular.width(&view.updated, 24.0) as i32;
    faces
        .regular
        .draw(canvas, &view.updated, 24.0, updated, 98, WHITE);
    let bar_left: i32 = menu::BUTTON.left + 25;
    for row in 0..3 {
        let bar: Rect = Rect {
            left: bar_left,
            top: 37 + 15 * row,
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

#[cfg(test)]
mod tests;
