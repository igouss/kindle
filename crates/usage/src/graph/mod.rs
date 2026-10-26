use eink::canvas::Canvas;
use fm_render_canvas::{CanvasRenderConfig, CanvasRenderResult};

use crate::bead::{Epic, Member, Status};
use crate::screen::Faces;

mod paint;

use paint::{Label, Painter};

const LINE_CHARS: usize = 14;
const TITLE_LINES: usize = 2;
const SHADES: &str = "  classDef closed fill:#cccccc,stroke:#444444,color:#000000\n  classDef active fill:#000000,stroke:#000000,color:#ffffff\n  classDef ready fill:#ffffff,stroke:#000000,stroke-width:4px,color:#000000\n  classDef waiting fill:#ffffff,stroke:#555555,color:#333333\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shade {
    Closed,
    Active,
    Ready,
    Waiting,
}

pub fn shade(epic: &Epic, member: &Member) -> Shade {
    let open_blocker: bool = epic.blocks.iter().any(|[from, to]: &[String; 2]| {
        to == &member.id
            && epic
                .beads
                .iter()
                .any(|m: &Member| &m.id == from && m.status != Status::Closed)
    });
    match member.status {
        Status::Closed => Shade::Closed,
        Status::InProgress => Shade::Active,
        Status::Open if !open_blocker => Shade::Ready,
        Status::Open | Status::Blocked | Status::Deferred | Status::Other => Shade::Waiting,
    }
}

pub fn short_id(id: &str) -> &str {
    id.rsplit_once('-')
        .map_or(id, |(_, tail): (&str, &str)| tail)
}

pub fn short_title(title: &str) -> Vec<String> {
    let clean: String = title.replace(['"', '[', ']', '<', '>'], "");
    let mut lines: Vec<String> = vec![String::new()];
    for word in clean.split_whitespace() {
        let line: &mut String = lines.last_mut().unwrap_or_else(|| unreachable!());
        let longer: usize =
            line.chars().count() + word.chars().count() + usize::from(!line.is_empty());
        if longer <= LINE_CHARS || line.is_empty() {
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(&word.chars().take(LINE_CHARS).collect::<String>());
            continue;
        }
        if lines.len() == TITLE_LINES {
            if let Some(last) = lines.last_mut() {
                last.push('\u{2026}');
            }
            return lines;
        }
        lines.push(word.chars().take(LINE_CHARS).collect());
    }
    lines
}

pub fn source(epic: &Epic) -> String {
    let mut text: String = String::from("flowchart LR\n");
    for (i, member) in epic.beads.iter().enumerate() {
        text.push_str(&format!(
            "  n{i}[\"{}<br/>{}\"]\n",
            short_id(&member.id),
            short_title(&member.title).join("<br/>")
        ));
    }
    for [from, to] in &epic.blocks {
        let index = |id: &String| epic.beads.iter().position(|m: &Member| &m.id == id);
        if let (Some(a), Some(b)) = (index(from), index(to)) {
            text.push_str(&format!("  n{a} --> n{b}\n"));
        }
    }
    text.push_str(SHADES);
    for (i, member) in epic.beads.iter().enumerate() {
        let class: &str = match shade(epic, member) {
            Shade::Closed => "closed",
            Shade::Active => "active",
            Shade::Ready => "ready",
            Shade::Waiting => "waiting",
        };
        text.push_str(&format!("  class n{i} {class}\n"));
    }
    text
}

fn config() -> CanvasRenderConfig {
    CanvasRenderConfig {
        font_size: 18.0,
        padding: 16.0,
        node_stroke_width: 2.0,
        edge_stroke: String::from("#000000"),
        edge_stroke_width: 2.0,
        ..CanvasRenderConfig::default()
    }
}

pub fn draw(canvas: &mut Canvas, faces: &Faces, epic: &Epic, area: eink::canvas::Rect) -> usize {
    let parsed: fm_parser::ParseResult = fm_parser::parse(&source(epic));
    let Some(mut painter) = Painter::new(faces, area.width, area.height) else {
        return 0;
    };
    let result: CanvasRenderResult =
        fm_render_canvas::render_to_canvas(&parsed.ir, &mut painter, &config());
    let labels: Vec<Label> = painter.into_canvas(canvas, area.left, area.top);
    for label in labels {
        let face: &eink::text::Typeface = if label.bold {
            &faces.bold
        } else {
            &faces.regular
        };
        face.draw(
            canvas,
            &label.text,
            label.size,
            area.left + label.left,
            area.top + label.baseline,
            label.ink,
        );
    }
    result.nodes_drawn
}

#[cfg(test)]
mod tests;
