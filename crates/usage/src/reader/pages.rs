use std::ops::Range;

use super::{Kind, Line};

pub const THUMB_MIN: u32 = 24;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub lines: Range<usize>,
    pub top: i32,
    pub height: i32,
}

pub fn paginate(lines: &[Line], height: i32) -> Vec<Page> {
    let mut pages: Vec<Page> = vec![];
    let mut start: usize = 0;
    let mut used: i32 = 0;
    let mut offset: i32 = 0;
    let mut i: usize = 0;
    while i < lines.len() {
        let need: i32 = lines[i].height(i == start);
        let held: i32 = match lines[i].kind {
            Kind::Heading => lines.get(i + 1).map_or(0, |next: &Line| next.height(false)),
            Kind::Title | Kind::Meta | Kind::Body | Kind::Code | Kind::Rule => 0,
        };
        if used + need + held > height && i > start {
            pages.push(Page {
                lines: start..i,
                top: offset,
                height: used,
            });
            offset += used;
            start = i;
            used = 0;
            continue;
        }
        used += need;
        i += 1;
    }
    pages.push(Page {
        lines: start..lines.len(),
        top: offset,
        height: used,
    });
    pages
}

pub fn thumb(pages: &[Page], current: &Page, track: u32) -> (u32, u32) {
    let total: i64 = pages
        .last()
        .map_or(1, |p: &Page| i64::from(p.top + p.height))
        .max(1);
    let floor: u32 = THUMB_MIN.min(track);
    let start: u32 =
        ((i64::from(track) * i64::from(current.top) / total) as u32).min(track - floor);
    let length: u32 = ((i64::from(track) * i64::from(current.height) / total) as u32).max(floor);
    (start, length)
}
