use super::*;
use eink::canvas::{Geometry, Rect};
use eink::text::Typeface;

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

fn member(id: &str, status: Status) -> Member {
    Member {
        id: id.to_owned(),
        title: format!("title {id}"),
        status,
    }
}

fn epic(beads: Vec<Member>, blocks: &[(&str, &str)]) -> Epic {
    Epic {
        id: String::from("k-e"),
        title: String::from("epic"),
        beads,
        blocks: blocks
            .iter()
            .map(|(a, b): &(&str, &str)| [a.to_string(), b.to_string()])
            .collect(),
    }
}

fn g3o() -> Epic {
    serde_json::from_str(include_str!("../../golden/g3o-epic.json")).unwrap()
}

#[test]
fn closed_is_closed_and_in_progress_is_active() {
    let e: Epic = epic(
        vec![
            member("k-e.1", Status::Closed),
            member("k-e.2", Status::InProgress),
        ],
        &[],
    );
    assert_eq!(
        (shade(&e, &e.beads[0]), shade(&e, &e.beads[1])),
        (Shade::Closed, Shade::Active)
    );
}

#[test]
fn an_open_bead_is_ready_when_every_blocker_is_closed_and_waits_otherwise() {
    let e: Epic = epic(
        vec![
            member("k-e.1", Status::Closed),
            member("k-e.2", Status::Open),
            member("k-e.3", Status::Open),
            member("k-e.4", Status::Open),
        ],
        &[("k-e.1", "k-e.2"), ("k-e.2", "k-e.3")],
    );
    assert_eq!(
        e.beads
            .iter()
            .map(|m: &Member| shade(&e, m))
            .collect::<Vec<Shade>>(),
        [Shade::Closed, Shade::Ready, Shade::Waiting, Shade::Ready]
    );
}

#[test]
fn blocked_deferred_and_unknown_beads_wait() {
    let e: Epic = epic(
        vec![
            member("a", Status::Blocked),
            member("b", Status::Deferred),
            member("c", Status::Other),
        ],
        &[],
    );
    assert!(
        e.beads
            .iter()
            .all(|m: &Member| shade(&e, m) == Shade::Waiting)
    );
}

#[test]
fn the_short_id_is_what_follows_the_last_dash() {
    assert_eq!(
        (short_id("omarchy-expose-g3o.13"), short_id("plain")),
        ("g3o.13", "plain")
    );
}

#[test]
fn a_title_is_cut_to_two_short_lines_with_an_ellipsis() {
    assert_eq!(
        (
            short_title("graph: the Graph view draws the epic"),
            short_title("tiny"),
            short_title(""),
            short_title("Supercalifragilisticexpialidocious word")
        ),
        (
            vec![
                String::from("graph: the"),
                String::from("Graph view\u{2026}")
            ],
            vec![String::from("tiny")],
            vec![String::new()],
            vec![String::from("Supercalifragi"), String::from("word")]
        )
    );
}

#[test]
fn quotes_and_brackets_never_reach_the_diagram_source() {
    assert_eq!(
        short_title(r#"say "hi" [x] <y>"#),
        vec![String::from("say hi x y")]
    );
}

#[test]
fn the_source_has_a_node_per_bead_an_edge_per_block_and_a_class_each() {
    let e: Epic = epic(
        vec![
            member("k-e.1", Status::Closed),
            member("k-e.2", Status::Open),
        ],
        &[("k-e.1", "k-e.2"), ("k-e.1", "elsewhere")],
    );
    let text: String = source(&e);
    let lines: Vec<&str> = text
        .lines()
        .filter(|l: &&str| !l.contains("classDef"))
        .collect();
    assert_eq!(
        lines,
        [
            "flowchart LR",
            "  n0[\"e.1<br/>title k-e.1\"]",
            "  n1[\"e.2<br/>title k-e.2\"]",
            "  n0 --> n1",
            "  class n0 closed",
            "  class n1 ready"
        ]
    );
}

#[test]
fn every_bead_of_a_real_epic_is_drawn() {
    let mut canvas: Canvas = Canvas::new(PAGE);
    let drawn: usize = draw(
        &mut canvas,
        &faces(),
        &g3o(),
        Rect {
            left: 0,
            top: 118,
            width: 1024,
            height: 640,
        },
    );
    assert_eq!(drawn, 13);
}
