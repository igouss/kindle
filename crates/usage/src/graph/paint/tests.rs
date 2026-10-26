use super::*;
use eink::canvas::{Geometry, WHITE};
use eink::text::Typeface;

const AREA: Geometry = Geometry {
    width: 100,
    height: 100,
    stride: 100,
};

pub(super) fn faces() -> Faces {
    Faces {
        regular: Typeface::regular().unwrap(),
        bold: Typeface::bold().unwrap(),
    }
}

pub(super) fn shown(painter: Painter<'_>) -> (Canvas, Vec<Label>) {
    let mut canvas: Canvas = Canvas::new(AREA);
    let labels: Vec<Label> = painter.into_canvas(&mut canvas, 0, 0);
    (canvas, labels)
}

pub(super) fn at(canvas: &Canvas, x: u32, y: u32) -> u8 {
    canvas.pixel(x, y).unwrap()
}

pub(super) fn dark(canvas: &Canvas) -> usize {
    canvas.pixels().iter().filter(|&&p: &&u8| p < 128).count()
}

#[test]
fn css_colours_become_luminance_and_unknown_ones_black() {
    assert_eq!(
        (
            gray("#ffffff"),
            gray("#000"),
            gray("rgb(255,0,0)"),
            gray("rgba(0,0,255,0.5)"),
            gray("teal")
        ),
        ((255, 1.0), (0, 1.0), (76, 1.0), (29, 0.5), (0, 1.0))
    );
}

#[test]
fn a_css_font_gives_its_size_and_whether_it_is_bold() {
    assert_eq!(
        (
            font("bold 14px sans-serif"),
            font("600 20px Inter"),
            font("500 12px Inter"),
            font("serif")
        ),
        ((14.0, true), (20.0, true), (12.0, false), (16.0, false))
    );
}

#[test]
fn a_new_page_is_white() {
    let faces: Faces = faces();
    let painter: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    assert_eq!(dark(&shown(painter).0), 0);
}

#[test]
fn fill_rect_paints_exactly_its_area_in_the_fill_gray() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.set_fill_style("#808080");
    p.fill_rect(10.0, 20.0, 30.0, 40.0);
    let (canvas, _): (Canvas, Vec<Label>) = shown(p);
    assert_eq!(
        (
            at(&canvas, 10, 20),
            at(&canvas, 39, 59),
            at(&canvas, 9, 20),
            at(&canvas, 40, 59),
            at(&canvas, 10, 60)
        ),
        (128, 128, WHITE, WHITE, WHITE)
    );
}

#[test]
fn a_stroke_is_as_wide_as_the_line_width() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.set_stroke_style("#000000");
    p.set_line_width(4.0);
    p.begin_path();
    p.move_to(10.0, 50.0);
    p.line_to(90.0, 50.0);
    p.stroke();
    let (canvas, _): (Canvas, Vec<Label>) = shown(p);
    let column: Vec<bool> = (44..56).map(|y: u32| at(&canvas, 50, y) < 128).collect();
    assert_eq!(column.iter().filter(|&&d: &&bool| d).count(), 4);
}

#[test]
fn a_square_cap_reaches_past_the_end_and_a_butt_cap_does_not() {
    let faces: Faces = faces();
    let line = |cap: LineCap| -> Canvas {
        let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
        p.set_line_width(10.0);
        p.set_line_cap(cap);
        p.begin_path();
        p.move_to(20.0, 50.0);
        p.line_to(80.0, 50.0);
        p.stroke();
        shown(p).0
    };
    let (butt, square, round): (Canvas, Canvas, Canvas) = (
        line(LineCap::Butt),
        line(LineCap::Square),
        line(LineCap::Round),
    );
    assert_eq!(
        (
            at(&butt, 83, 50) < 128,
            at(&square, 83, 54) < 128,
            at(&round, 83, 50) < 128,
            at(&round, 84, 54) < 128
        ),
        (false, true, true, false)
    );
}

#[test]
fn a_miter_join_has_a_point_and_bevel_and_round_joins_do_not() {
    let faces: Faces = faces();
    let elbow = |join: LineJoin| -> Canvas {
        let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
        p.set_line_width(16.0);
        p.set_line_join(join);
        p.begin_path();
        p.move_to(20.0, 70.0);
        p.line_to(70.0, 70.0);
        p.line_to(70.0, 20.0);
        p.stroke();
        shown(p).0
    };
    assert_eq!(
        (
            at(&elbow(LineJoin::Miter), 77, 77) < 128,
            at(&elbow(LineJoin::Bevel), 77, 77) < 128,
            at(&elbow(LineJoin::Round), 77, 77) < 128
        ),
        (true, false, false)
    );
}

#[test]
fn dashes_scale_with_the_transform() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.scale(3.0, 3.0);
    p.set_line_width(1.0);
    p.set_line_dash(&[5.0, 5.0]);
    p.begin_path();
    p.move_to(0.0, 10.0);
    p.line_to(33.0, 10.0);
    p.stroke();
    let canvas: Canvas = shown(p).0;
    assert_eq!(
        (at(&canvas, 12, 30) < 128, at(&canvas, 22, 30) < 128),
        (true, false)
    );
}

#[test]
fn text_size_follows_the_square_root_of_the_transform_determinant() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.set_font("10px sans-serif");
    p.scale(3.0, 3.0);
    p.fill_text("a", 0.0, 0.0);
    p.set_transform(2.0, 1.0, 1.0, 2.0, 0.0, 0.0);
    p.fill_text("a", 0.0, 0.0);
    let (_, labels): (Canvas, Vec<Label>) = shown(p);
    assert_eq!(
        (labels[0].size, (labels[1].size * 100.0).round()),
        (30.0, (10.0 * 3.0_f32.sqrt() * 100.0).round())
    );
}

#[test]
fn translate_and_scale_move_and_grow_what_is_drawn() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.translate(20.0, 30.0);
    p.scale(2.0, 2.0);
    p.fill_rect(0.0, 0.0, 10.0, 10.0);
    let (canvas, _): (Canvas, Vec<Label>) = shown(p);
    assert_eq!(
        (
            at(&canvas, 20, 30) < 128,
            at(&canvas, 39, 49) < 128,
            at(&canvas, 40, 50) < 128,
            at(&canvas, 19, 30) < 128
        ),
        (true, true, false, false)
    );
}

#[test]
fn set_transform_replaces_and_reset_transform_clears() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.translate(50.0, 50.0);
    p.set_transform(1.0, 0.0, 0.0, 1.0, 10.0, 0.0);
    p.fill_rect(0.0, 0.0, 5.0, 5.0);
    p.reset_transform();
    p.fill_rect(0.0, 90.0, 5.0, 5.0);
    let (canvas, _): (Canvas, Vec<Label>) = shown(p);
    assert_eq!(
        (
            at(&canvas, 11, 1) < 128,
            at(&canvas, 61, 51) < 128,
            at(&canvas, 1, 91) < 128
        ),
        (true, false, true)
    );
}

#[test]
fn a_quarter_turn_puts_the_x_axis_downwards() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.translate(50.0, 10.0);
    p.rotate(std::f64::consts::FRAC_PI_2);
    p.fill_rect(0.0, -2.0, 40.0, 4.0);
    let (canvas, _): (Canvas, Vec<Label>) = shown(p);
    assert_eq!(
        (at(&canvas, 50, 40) < 128, at(&canvas, 80, 12) < 128),
        (true, false)
    );
}

#[test]
fn a_restore_with_nothing_saved_changes_nothing() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.set_fill_style("#000000");
    p.restore();
    p.fill_rect(0.0, 0.0, 10.0, 10.0);
    assert_eq!(at(&shown(p).0, 5, 5), 0);
}

#[test]
fn a_clip_keeps_drawing_inside_its_path_until_restored() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.save();
    p.begin_path();
    p.rect(0.0, 0.0, 50.0, 100.0);
    p.clip();
    p.fill_rect(0.0, 0.0, 100.0, 50.0);
    p.restore();
    p.fill_rect(0.0, 60.0, 100.0, 10.0);
    let (canvas, _): (Canvas, Vec<Label>) = shown(p);
    assert_eq!(
        (
            at(&canvas, 25, 25) < 128,
            at(&canvas, 75, 25) < 128,
            at(&canvas, 75, 65) < 128
        ),
        (true, false, true)
    );
}

#[test]
fn two_clips_intersect() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.begin_path();
    p.rect(0.0, 0.0, 60.0, 100.0);
    p.clip();
    p.begin_path();
    p.rect(40.0, 0.0, 60.0, 100.0);
    p.clip();
    p.fill_rect(0.0, 0.0, 100.0, 100.0);
    let (canvas, _): (Canvas, Vec<Label>) = shown(p);
    assert_eq!(
        (
            at(&canvas, 20, 50) < 128,
            at(&canvas, 50, 50) < 128,
            at(&canvas, 80, 50) < 128
        ),
        (false, true, false)
    );
}

#[test]
fn global_alpha_and_colour_alpha_lighten_the_fill() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.set_global_alpha(0.5);
    p.fill_rect(0.0, 0.0, 10.0, 10.0);
    p.set_global_alpha(1.0);
    p.set_fill_style("rgba(0,0,0,0.5)");
    p.fill_rect(20.0, 0.0, 10.0, 10.0);
    let (canvas, _): (Canvas, Vec<Label>) = shown(p);
    assert_eq!((at(&canvas, 5, 5), at(&canvas, 25, 5)), (127, 127));
}

#[test]
fn a_dashed_line_has_gaps() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.set_line_width(2.0);
    p.set_line_dash(&[10.0, 10.0]);
    p.begin_path();
    p.move_to(0.0, 50.0);
    p.line_to(100.0, 50.0);
    p.stroke();
    let (canvas, _): (Canvas, Vec<Label>) = shown(p);
    assert_eq!(
        (at(&canvas, 5, 50) < 128, at(&canvas, 15, 50) < 128),
        (true, false)
    );
}

#[test]
fn clear_rect_whitens_whatever_the_fill_and_keeps_the_fill() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.set_fill_style("#000000");
    p.fill_rect(0.0, 0.0, 100.0, 100.0);
    p.clear_rect(10.0, 10.0, 10.0, 10.0);
    p.fill_rect(30.0, 30.0, 5.0, 5.0);
    let (canvas, _): (Canvas, Vec<Label>) = shown(p);
    assert_eq!((at(&canvas, 15, 15), at(&canvas, 32, 32)), (WHITE, 0));
}

#[test]
fn stroke_rect_draws_the_outline_and_keeps_the_open_path() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.set_line_width(2.0);
    p.begin_path();
    p.move_to(0.0, 90.0);
    p.line_to(100.0, 90.0);
    p.stroke_rect(20.0, 20.0, 40.0, 40.0);
    p.stroke();
    let (canvas, _): (Canvas, Vec<Label>) = shown(p);
    assert_eq!(
        (
            at(&canvas, 20, 40) < 128,
            at(&canvas, 40, 40) < 128,
            at(&canvas, 50, 90) < 128
        ),
        (true, false, true)
    );
}

#[test]
fn a_line_without_a_start_begins_there() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.set_line_width(2.0);
    p.begin_path();
    p.line_to(10.0, 50.0);
    p.line_to(90.0, 50.0);
    p.stroke();
    let (canvas, _): (Canvas, Vec<Label>) = shown(p);
    assert_eq!(
        (at(&canvas, 5, 50) < 128, at(&canvas, 50, 50) < 128),
        (false, true)
    );
}

#[test]
fn text_is_set_after_the_shapes_where_alignment_and_baseline_put_it() {
    let faces: Faces = faces();
    let width: f64 = f64::from(faces.regular.width("Hi", 20.0));
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.set_font("20px sans-serif");
    p.set_fill_style("#000000");
    p.fill_text("Hi", 10.0, 50.0);
    p.set_text_align(TextAlign::Center);
    p.set_text_baseline(TextBaseline::Top);
    p.fill_text("Hi", 50.0, 10.0);
    p.set_text_align(TextAlign::Right);
    p.set_text_baseline(TextBaseline::Middle);
    p.fill_text("Hi", 90.0, 30.0);
    p.set_text_baseline(TextBaseline::Bottom);
    p.set_stroke_style("#808080");
    p.stroke_text("Hi", 90.0, 80.0);
    let (_, labels): (Canvas, Vec<Label>) = shown(p);
    let placed: Vec<(i32, i32, u8)> = labels
        .iter()
        .map(|l: &Label| (l.left, l.baseline, l.ink))
        .collect();
    assert_eq!(
        placed,
        [
            (10, 50, 0),
            ((50.0 - width / 2.0).round() as i32, 26, 0),
            ((90.0 - width).round() as i32, 37, 0),
            ((90.0 - width).round() as i32, 76, 128)
        ]
    );
}

#[test]
fn text_grows_with_the_transform_and_measures_with_the_dashboard_faces() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.set_font("bold 10px sans-serif");
    let measured: TextMetrics = p.measure_text("Hi");
    p.scale(2.0, 2.0);
    p.fill_text("Hi", 5.0, 20.0);
    let (_, labels): (Canvas, Vec<Label>) = shown(p);
    assert_eq!(
        (
            measured.width,
            measured.height,
            labels[0].size,
            labels[0].bold,
            (labels[0].left, labels[0].baseline)
        ),
        (
            f64::from(faces.bold.width("Hi", 10.0)),
            10.0,
            20.0,
            true,
            (10, 40)
        )
    );
}

#[test]
fn shadows_are_ignored_and_the_size_is_the_page() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 80, 60).unwrap();
    p.set_shadow_blur(5.0);
    p.set_shadow_color("#000000");
    p.set_shadow_offset(3.0, 3.0);
    assert_eq!((p.width(), p.height(), dark(&shown(p).0)), (80.0, 60.0, 0));
}
