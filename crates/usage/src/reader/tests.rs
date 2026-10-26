use super::pages::THUMB_MIN;
use super::*;
use crate::screen::Faces;
use eink::text::Typeface;

fn faces() -> Faces {
    Faces {
        regular: Typeface::regular().unwrap(),
        bold: Typeface::bold().unwrap(),
    }
}

fn source(description: &str) -> Vec<Line> {
    lay(
        &faces(),
        900,
        &Source {
            place: "kindle \u{b7} kindle-bxx.3",
            title: "reader",
            meta: "P1 \u{b7} task",
            description,
        },
    )
}

fn body(description: &str) -> Vec<Line> {
    source(description).split_off(4)
}

fn text(line: &Line) -> String {
    line.words
        .iter()
        .enumerate()
        .map(|(i, w): (usize, &Word)| match (i, w.glued) {
            (0, _) | (_, true) => w.text.clone(),
            (_, false) => format!(" {}", w.text),
        })
        .collect()
}

fn line(kind: Kind, gap: i32) -> Line {
    Line {
        kind,
        indent: 0,
        marker: None,
        words: vec![],
        gap,
    }
}

#[test]
fn the_head_is_place_title_meta_and_a_rule() {
    let kinds: Vec<Kind> = source("").iter().map(|l: &Line| l.kind).collect();
    assert_eq!(kinds, [Kind::Meta, Kind::Title, Kind::Meta, Kind::Rule]);
}

#[test]
fn text_touching_inline_code_stays_one_word() {
    let lines: Vec<Line> = body("the `InkHatch`es and (`hatch`); `drawn`.");
    assert_eq!(text(&lines[0]), "the InkHatches and (hatch); drawn.");
}

#[test]
fn spaces_around_inline_code_are_kept() {
    let lines: Vec<Line> = body("a `b` c");
    assert_eq!(
        (
            text(&lines[0]),
            lines[0].words.iter().filter(|w: &&Word| w.glued).count()
        ),
        (String::from("a b c"), 0)
    );
}

#[test]
fn a_soft_break_is_a_space() {
    assert_eq!(text(&body("one\ntwo")[0]), "one two");
}

#[test]
fn strong_text_is_bold_and_the_rest_regular() {
    let weights: Vec<(String, Weight)> = body("plain **loud** plain")[0]
        .words
        .iter()
        .map(|w: &Word| (w.text.clone(), w.weight))
        .collect();
    assert_eq!(
        weights,
        [
            (String::from("plain"), Weight::Regular),
            (String::from("loud"), Weight::Bold),
            (String::from("plain"), Weight::Regular)
        ]
    );
}

#[test]
fn a_glued_word_is_never_split_across_lines() {
    let narrow: Vec<Line> = lay(
        &faces(),
        200,
        &Source {
            place: "",
            title: "",
            meta: "",
            description: "aaaa bbbb cccc `dddd`eeee",
        },
    );
    let last: &Line = narrow.last().unwrap();
    assert_eq!(text(last), "ddddeeee");
}

#[test]
fn lists_carry_bullets_numbers_from_their_start_and_task_boxes() {
    let markers: Vec<Option<String>> = body("- a\n- b\n\n3. c\n4. d\n\n- [ ] e\n- [x] f")
        .iter()
        .map(|l: &Line| l.marker.clone())
        .collect();
    assert_eq!(
        markers,
        [
            Some(String::from("\u{2022}")),
            Some(String::from("\u{2022}")),
            Some(String::from("3.")),
            Some(String::from("4.")),
            Some(String::from("\u{2610}")),
            Some(String::from("\u{2611}"))
        ]
    );
}

#[test]
fn a_heading_is_bold_and_stands_further_from_what_came_before() {
    let lines: Vec<Line> = body("first\n\n## Next\n\nsecond");
    assert_eq!(
        (
            lines[1].kind,
            lines[1].gap > lines[2].gap,
            lines[1].words[0].weight
        ),
        (Kind::Heading, true, Weight::Bold)
    );
}

#[test]
fn a_code_block_keeps_its_lines() {
    let kinds: Vec<(Kind, String)> = body("```\nlet a = 1;\nlet b = 2;\n```")
        .iter()
        .map(|l: &Line| (l.kind, text(l)))
        .collect();
    assert_eq!(
        kinds,
        [
            (Kind::Code, String::from("let a = 1;")),
            (Kind::Code, String::from("let b = 2;"))
        ]
    );
}

#[test]
fn nothing_paginates_to_one_empty_page() {
    assert_eq!(
        paginate(&[], 100),
        vec![Page {
            lines: 0..0,
            top: 0,
            height: 0
        }]
    );
}

#[test]
fn lines_that_fit_make_one_page() {
    let lines: Vec<Line> = vec![line(Kind::Body, 0), line(Kind::Body, 10)];
    assert_eq!(
        paginate(&lines, 80),
        vec![Page {
            lines: 0..2,
            top: 0,
            height: 80
        }]
    );
}

#[test]
fn many_lines_break_into_pages_and_the_gap_at_a_page_top_is_dropped() {
    let lines: Vec<Line> = vec![
        line(Kind::Body, 0),
        line(Kind::Body, 10),
        line(Kind::Body, 10),
    ];
    assert_eq!(
        paginate(&lines, 80),
        vec![
            Page {
                lines: 0..2,
                top: 0,
                height: 80
            },
            Page {
                lines: 2..3,
                top: 80,
                height: 35
            }
        ]
    );
}

#[test]
fn a_heading_never_ends_a_page() {
    let lines: Vec<Line> = vec![
        line(Kind::Body, 0),
        line(Kind::Heading, 0),
        line(Kind::Title, 0),
    ];
    let pages: Vec<Page> = paginate(&lines, 115);
    assert_eq!(
        pages
            .iter()
            .map(|p: &Page| p.lines.clone())
            .collect::<Vec<_>>(),
        [0..1, 1..3]
    );
}

#[test]
fn a_line_taller_than_the_page_gets_a_page_of_its_own() {
    let lines: Vec<Line> = vec![
        line(Kind::Body, 0),
        line(Kind::Title, 0),
        line(Kind::Body, 0),
    ];
    let pages: Vec<Page> = paginate(&lines, 40);
    assert_eq!(
        pages
            .iter()
            .map(|p: &Page| p.lines.clone())
            .collect::<Vec<_>>(),
        [0..1, 1..2, 2..3]
    );
}

fn shape(description: &str) -> Vec<(Kind, i32, i32, String)> {
    body(description)
        .iter()
        .map(|l: &Line| (l.kind, l.indent, l.gap, text(l)))
        .collect()
}

#[test]
fn the_first_block_sits_closer_to_the_head_than_later_blocks_to_each_other() {
    assert_eq!(
        shape("one\n\ntwo"),
        [
            (Kind::Body, 0, 8, String::from("one")),
            (Kind::Body, 0, 16, String::from("two"))
        ]
    );
}

#[test]
fn a_code_block_is_indented_and_only_its_first_line_has_a_gap() {
    assert_eq!(
        shape("```\na\nb\n```"),
        [
            (Kind::Code, INDENT / 2, 8, String::from("a")),
            (Kind::Code, INDENT / 2, 0, String::from("b"))
        ]
    );
}

#[test]
fn a_quote_is_indented_and_its_paragraphs_keep_a_small_gap() {
    assert_eq!(
        shape("> a\n>\n> b"),
        [
            (Kind::Body, INDENT, 8, String::from("a")),
            (Kind::Body, INDENT, 8, String::from("b"))
        ]
    );
}

#[test]
fn a_nested_list_indents_one_step_further() {
    let lines: Vec<Line> = body("1. a\n   - b\n2. c");
    let nested: Vec<(i32, Option<String>, i32)> = lines
        .iter()
        .map(|l: &Line| (l.indent, l.marker.clone(), l.gap))
        .collect();
    assert_eq!(
        nested,
        [
            (INDENT, Some(String::from("1.")), 8),
            (2 * INDENT, Some(String::from("\u{2022}")), 8),
            (INDENT, Some(String::from("2.")), 8)
        ]
    );
}

#[test]
fn a_list_item_that_opens_with_a_code_block_keeps_its_marker_on_a_line_of_its_own() {
    let lines: Vec<Line> = body("- ```\n  code\n  ```");
    let kinds: Vec<(Kind, i32, Option<String>)> = lines
        .iter()
        .map(|l: &Line| (l.kind, l.indent, l.marker.clone()))
        .collect();
    assert_eq!(
        kinds,
        [
            (Kind::Body, INDENT, Some(String::from("\u{2022}"))),
            (Kind::Code, INDENT + INDENT / 2, None)
        ]
    );
}

#[test]
fn a_table_reads_row_by_row_with_a_bold_head() {
    let lines: Vec<Line> = body("| a | b |\n| --- | --- |\n| 1 | 2 |\n| 3 | 4 |");
    let rows: Vec<(String, Weight, i32)> = lines
        .iter()
        .map(|l: &Line| (text(l), l.words[0].weight, l.gap))
        .collect();
    assert_eq!(
        rows,
        [
            (String::from("a \u{b7} b"), Weight::Bold, 8),
            (String::from("1 \u{b7} 2"), Weight::Regular, 4),
            (String::from("3 \u{b7} 4"), Weight::Regular, 4)
        ]
    );
}

#[test]
fn a_one_column_table_has_no_separators() {
    let lines: Vec<Line> = body("| a |\n| --- |\n| 1 |");
    assert_eq!(
        lines.iter().map(text).collect::<Vec<String>>(),
        [String::from("a"), String::from("1")]
    );
}

#[test]
fn a_thematic_break_is_a_rule() {
    assert_eq!(
        shape("a\n\n---\n\nb"),
        [
            (Kind::Body, 0, 8, String::from("a")),
            (Kind::Rule, 0, 16, String::new()),
            (Kind::Body, 0, 16, String::from("b"))
        ]
    );
}

#[test]
fn a_word_wider_than_the_line_is_cut_into_pieces_that_each_fit() {
    let faces: Faces = faces();
    let url: &str = "https://github.com/igouss/franken_markdown/blob/4b705d147c10cebb52866f37dbdb1492ce8505ea/src/ast.rs";
    let lines: Vec<Line> = lay(
        &faces,
        300,
        &Source {
            place: "",
            title: "",
            meta: "",
            description: &format!("see {url}"),
        },
    );
    let pieces: Vec<String> = lines[4..].iter().map(text).collect();
    assert!(
        pieces[0] == "see"
            && pieces[1..].concat() == url
            && lines[4..]
                .iter()
                .all(|l: &Line| faces.regular.width(&text(l), 26.0) <= 300)
    );
}

#[test]
fn words_that_exactly_fill_the_line_share_it() {
    let faces: Faces = faces();
    let width: u32 = faces.regular.width("aaa", 26.0)
        + faces.regular.width(" ", 26.0)
        + faces.regular.width("bbb", 26.0);
    let lines: Vec<Line> = lay(
        &faces,
        width,
        &Source {
            place: "",
            title: "",
            meta: "",
            description: "aaa bbb ccc",
        },
    );
    assert_eq!(
        lines[4..].iter().map(text).collect::<Vec<String>>(),
        [String::from("aaa bbb"), String::from("ccc")]
    );
}

#[test]
fn code_keeps_its_indentation_and_inner_spaces() {
    assert_eq!(
        body("```\nfn a() {\n    let  b = 1;\n}\n```")
            .iter()
            .map(text)
            .collect::<Vec<String>>(),
        [
            String::from("fn a() {"),
            String::from("    let  b = 1;"),
            String::from("}")
        ]
    );
}

#[test]
fn an_empty_code_line_keeps_its_place() {
    assert_eq!(body("```\na\n\nb\n```").len(), 3);
}

#[test]
fn a_code_line_wider_than_the_page_breaks_between_characters() {
    let faces: Faces = faces();
    let long: String = "x = 1; ".repeat(40);
    let lines: Vec<Line> = lay(
        &faces,
        300,
        &Source {
            place: "",
            title: "",
            meta: "",
            description: &format!("```\n{long}\n```"),
        },
    );
    let code: Vec<&Line> = lines[4..].iter().collect();
    assert!(
        code.len() > 3
            && code.iter().map(|l: &&Line| text(l)).collect::<String>() == long.trim_end()
            && code.iter().all(|l: &&Line| {
                faces.regular.width(&text(l), 22.0) <= 300 - (INDENT / 2) as u32
            })
    );
}

fn page(top: i32, height: i32) -> Page {
    Page {
        lines: 0..0,
        top,
        height,
    }
}

#[test]
fn one_page_fills_the_whole_track() {
    let pages: Vec<Page> = vec![page(0, 500)];
    assert_eq!(thumb(&pages, &pages[0], 600), (0, 600));
}

#[test]
fn the_thumb_is_the_share_of_the_document_on_screen() {
    let pages: Vec<Page> = vec![page(0, 300), page(300, 300), page(600, 300)];
    assert_eq!(
        (
            thumb(&pages, &pages[0], 600),
            thumb(&pages, &pages[1], 600),
            thumb(&pages, &pages[2], 600)
        ),
        ((0, 200), (200, 200), (400, 200))
    );
}

#[test]
fn a_sliver_of_a_last_page_still_shows_a_full_size_thumb_at_the_bottom() {
    let pages: Vec<Page> = vec![page(0, 990), page(990, 10)];
    assert_eq!(thumb(&pages, &pages[1], 600), (600 - THUMB_MIN, THUMB_MIN));
}

#[test]
fn a_track_shorter_than_the_minimum_thumb_is_filled() {
    let pages: Vec<Page> = vec![page(0, 50), page(50, 50)];
    assert_eq!(thumb(&pages, &pages[1], 10), (0, 10));
}

#[test]
fn a_glued_run_exactly_as_wide_as_the_line_stays_on_it() {
    let faces: Faces = faces();
    let width: u32 = faces.regular.width("aaa", 26.0) + faces.regular.width("bbb", 26.0);
    let lines: Vec<Line> = lay(
        &faces,
        width,
        &Source {
            place: "",
            title: "",
            meta: "",
            description: "`aaa`bbb",
        },
    );
    assert_eq!(text(&lines[4]), "aaabbb");
}

#[test]
fn a_code_line_exactly_as_wide_as_the_line_stays_on_it() {
    let faces: Faces = faces();
    let width: u32 = faces.regular.width("abc def", 22.0) + (INDENT / 2) as u32;
    let lines: Vec<Line> = lay(
        &faces,
        width,
        &Source {
            place: "",
            title: "",
            meta: "",
            description: "```\nabc def\n```",
        },
    );
    assert_eq!(
        lines[4..].iter().map(text).collect::<Vec<String>>(),
        [String::from("abc def")]
    );
}

#[test]
fn an_empty_list_item_still_shows_its_marker() {
    assert_eq!(
        body("-\n- a")
            .iter()
            .map(|l: &Line| (l.indent, l.marker.clone(), text(l)))
            .collect::<Vec<_>>(),
        [
            (INDENT, Some(String::from("\u{2022}")), String::new()),
            (INDENT, Some(String::from("\u{2022}")), String::from("a"))
        ]
    );
}

#[test]
fn a_definition_list_shows_bold_terms_over_indented_definitions() {
    let lines: Vec<Line> = body("Lead\n\nTerm\n: Meaning\n\nOther\n: Sense");
    assert_eq!(
        lines[1..]
            .iter()
            .map(|l: &Line| (l.indent, l.gap, l.words[0].weight, text(l)))
            .collect::<Vec<_>>(),
        [
            (0, 16, Weight::Bold, String::from("Term")),
            (INDENT, 0, Weight::Regular, String::from("Meaning")),
            (0, 8, Weight::Bold, String::from("Other")),
            (INDENT, 0, Weight::Regular, String::from("Sense"))
        ]
    );
}

#[test]
fn a_term_with_two_names_puts_the_gap_only_above_the_first() {
    let lines: Vec<Line> = body("One\nTwo\n: Meaning");
    assert_eq!(
        lines
            .iter()
            .map(|l: &Line| (l.gap, text(l)))
            .collect::<Vec<_>>(),
        [
            (8, String::from("One")),
            (0, String::from("Two")),
            (0, String::from("Meaning"))
        ]
    );
}
