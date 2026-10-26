use super::tests::{at, dark, faces, shown};
use super::*;
use eink::canvas::WHITE;

#[test]
fn a_full_arc_fills_a_disc_of_its_radius() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.begin_path();
    p.arc(50.0, 50.0, 30.0, 0.0, TAU);
    p.fill();
    let area: f64 = dark(&shown(p).0) as f64;
    let disc: f64 = std::f64::consts::PI * 900.0;
    assert!((area - disc).abs() < disc * 0.02, "{area} vs {disc}");
}

#[test]
fn a_half_arc_fills_half_a_disc_below_the_centre() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.begin_path();
    p.arc(50.0, 50.0, 30.0, 0.0, std::f64::consts::PI);
    p.fill();
    let (canvas, _): (Canvas, Vec<Label>) = shown(p);
    assert_eq!(
        (at(&canvas, 50, 70) < 128, at(&canvas, 50, 30) < 128),
        (true, false)
    );
}

fn corner(faces: &Faces, shift: f64, rounded: bool) -> Canvas {
    let mut p: Painter<'_> = Painter::new(faces, 100, 100).unwrap();
    p.translate(shift, shift);
    p.begin_path();
    p.move_to(10.0, 10.0);
    p.line_to(60.0, 10.0);
    if rounded {
        p.arc_to(90.0, 10.0, 90.0, 40.0, 30.0);
    } else {
        p.arc(60.0, 40.0, 30.0, -std::f64::consts::FRAC_PI_2, 0.0);
    }
    p.line_to(90.0, 90.0);
    p.line_to(10.0, 90.0);
    p.close_path();
    p.fill();
    shown(p).0
}

#[test]
fn arc_to_draws_the_same_corner_as_the_matching_arc() {
    let faces: Faces = faces();
    assert!(corner(&faces, 0.0, true).pixels() == corner(&faces, 0.0, false).pixels());
}

#[test]
fn arc_to_draws_the_same_corner_under_a_transform() {
    let faces: Faces = faces();
    assert!(corner(&faces, 5.0, true).pixels() == corner(&faces, 5.0, false).pixels());
}

#[test]
fn the_rounded_corner_cuts_off_the_square_corner() {
    let faces: Faces = faces();
    let canvas: Canvas = corner(&faces, 0.0, true);
    assert_eq!(
        (at(&canvas, 80, 20) < 128, at(&canvas, 84, 16) < 128),
        (true, false)
    );
}

fn degenerate(faces: &Faces, to: (f64, f64), radius: f64, pen: bool) -> Canvas {
    let mut p: Painter<'_> = Painter::new(faces, 100, 100).unwrap();
    p.set_line_width(2.0);
    p.begin_path();
    if pen {
        p.move_to(10.0, 10.0);
    }
    p.arc_to(90.0, 10.0, to.0, to.1, radius);
    p.line_to(90.0, 90.0);
    p.stroke();
    shown(p).0
}

#[test]
fn a_zero_radius_arc_to_is_a_sharp_corner() {
    let canvas: Canvas = degenerate(&faces(), (90.0, 40.0), 0.0, true);
    assert_eq!(
        (at(&canvas, 89, 10) < 128, at(&canvas, 50, 10) < 128),
        (true, true)
    );
}

#[test]
fn a_straight_arc_to_runs_to_its_corner_point() {
    let canvas: Canvas = degenerate(&faces(), (95.0, 10.0), 10.0, true);
    assert_eq!(
        (at(&canvas, 50, 10) < 128, at(&canvas, 90, 50) < 128),
        (true, true)
    );
}

#[test]
fn an_arc_to_with_no_pen_only_moves_to_its_corner_point() {
    let canvas: Canvas = degenerate(&faces(), (90.0, 40.0), 10.0, false);
    assert_eq!(
        (at(&canvas, 50, 10) < 128, at(&canvas, 90, 50) < 128),
        (false, true)
    );
}

fn ring(canvas: &Canvas, angle: f64, radius: f64) -> bool {
    let x: f64 = 50.0 + radius * angle.cos();
    let y: f64 = 50.0 + radius * angle.sin();
    at(canvas, x.round() as u32, y.round() as u32) < 128
}

#[test]
fn a_quarter_arc_from_any_start_fills_exactly_its_quarter() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    let half_pi: f64 = std::f64::consts::FRAC_PI_2;
    p.begin_path();
    p.move_to(50.0, 50.0);
    p.arc(50.0, 50.0, 40.0, half_pi, 2.0 * half_pi);
    p.close_path();
    p.fill();
    let canvas: Canvas = shown(p).0;
    let middle: f64 = 1.5 * half_pi;
    assert_eq!(
        (
            ring(&canvas, middle, 38.0),
            ring(&canvas, middle, 42.0),
            ring(&canvas, half_pi + 0.15, 30.0),
            ring(&canvas, 2.0 * half_pi - 0.15, 30.0),
            ring(&canvas, half_pi - 0.15, 30.0),
            ring(&canvas, 2.0 * half_pi + 0.15, 30.0)
        ),
        (true, false, true, true, false, false)
    );
}

#[test]
fn a_cubic_curve_bends_toward_its_control_points() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.set_line_width(3.0);
    p.begin_path();
    p.move_to(10.0, 90.0);
    p.bezier_curve_to(10.0, 10.0, 90.0, 10.0, 90.0, 90.0);
    p.stroke();
    let canvas: Canvas = shown(p).0;
    assert_eq!(
        (at(&canvas, 50, 30) < 128, at(&canvas, 50, 89) < 128),
        (true, false)
    );
}

#[test]
fn restore_brings_back_the_saved_fill_and_transform() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.set_fill_style("#000000");
    p.save();
    p.set_fill_style("#ffffff");
    p.translate(50.0, 0.0);
    p.restore();
    p.fill_rect(0.0, 0.0, 10.0, 10.0);
    let (canvas, _): (Canvas, Vec<Label>) = shown(p);
    assert_eq!((at(&canvas, 5, 5), at(&canvas, 55, 5)), (0, WHITE));
}

#[test]
fn a_quadratic_curve_bends_toward_its_control_point() {
    let faces: Faces = faces();
    let mut p: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    p.set_line_width(3.0);
    p.begin_path();
    p.move_to(10.0, 90.0);
    p.quadratic_curve_to(50.0, 10.0, 90.0, 90.0);
    p.stroke();
    let (canvas, _): (Canvas, Vec<Label>) = shown(p);
    assert_eq!(
        (at(&canvas, 50, 50) < 128, at(&canvas, 50, 89) < 128),
        (true, false)
    );
}

fn differing(a: &Canvas, b: &Canvas) -> usize {
    a.pixels()
        .iter()
        .zip(b.pixels())
        .filter(|(x, y): &(&u8, &u8)| x.abs_diff(**y) > 32)
        .count()
}

#[test]
fn a_diagonal_arc_to_matches_the_arc_worked_out_by_hand() {
    let faces: Faces = faces();
    let root: f64 = std::f64::consts::SQRT_2;
    let quarter: f64 = std::f64::consts::FRAC_PI_4;
    let mut rounded: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    rounded.set_line_width(3.0);
    rounded.begin_path();
    rounded.move_to(20.0, 50.0);
    rounded.arc_to(50.0, 20.0, 80.0, 50.0, 15.0);
    rounded.line_to(80.0, 50.0);
    rounded.stroke();
    let mut by_hand: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    by_hand.set_line_width(3.0);
    by_hand.begin_path();
    by_hand.move_to(20.0, 50.0);
    by_hand.arc(50.0, 20.0 + 15.0 * root, 15.0, -3.0 * quarter, -quarter);
    by_hand.line_to(80.0, 50.0);
    by_hand.stroke();
    assert_eq!(differing(&shown(rounded).0, &shown(by_hand).0), 0);
}

fn after_arc_to(
    faces: &Faces,
    from: (f64, f64),
    corner: (f64, f64),
    to: (f64, f64),
    radius: f64,
) -> Canvas {
    let mut p: Painter<'_> = Painter::new(faces, 100, 100).unwrap();
    p.set_line_width(2.0);
    p.begin_path();
    p.move_to(from.0, from.1);
    p.arc_to(corner.0, corner.1, to.0, to.1, radius);
    p.line_to(10.0, 90.0);
    p.stroke();
    shown(p).0
}

#[test]
fn an_arc_to_from_its_own_corner_keeps_the_rest_of_the_path() {
    let canvas: Canvas = after_arc_to(&faces(), (90.0, 10.0), (90.0, 10.0), (90.0, 40.0), 10.0);
    assert!(at(&canvas, 50, 50) < 128);
}

#[test]
fn an_arc_to_whose_corner_is_its_end_keeps_the_rest_of_the_path() {
    let canvas: Canvas = after_arc_to(&faces(), (10.0, 10.0), (90.0, 10.0), (90.0, 10.0), 10.0);
    assert_eq!(
        (at(&canvas, 50, 10) < 128, at(&canvas, 50, 50) < 128),
        (true, true)
    );
}

#[test]
fn an_arc_to_folded_back_on_itself_keeps_the_rest_of_the_path() {
    let canvas: Canvas = after_arc_to(&faces(), (10.0, 10.0), (90.0, 10.0), (50.0, 10.0), 10.0);
    assert_eq!(
        (at(&canvas, 50, 10) < 128, at(&canvas, 50, 50) < 128),
        (true, true)
    );
}

#[test]
fn a_negative_radius_gives_a_sharp_corner() {
    let faces: Faces = faces();
    let sharp: Canvas = after_arc_to(&faces, (10.0, 10.0), (90.0, 10.0), (90.0, 40.0), 0.0);
    let negative: Canvas = after_arc_to(&faces, (10.0, 10.0), (90.0, 10.0), (90.0, 40.0), -10.0);
    assert_eq!(differing(&sharp, &negative), 0);
}

#[test]
fn an_acute_arc_to_matches_the_arc_worked_out_by_hand() {
    let faces: Faces = faces();
    let (sin60, cos60): (f64, f64) = (3.0_f64.sqrt() / 2.0, 0.5);
    let reach: f64 = 10.0 / (std::f64::consts::PI / 6.0).tan();
    let mut rounded: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    rounded.set_line_width(3.0);
    rounded.begin_path();
    rounded.move_to(20.0, 20.0);
    rounded.arc_to(70.0, 20.0, 70.0 - 40.0 * cos60, 20.0 + 40.0 * sin60, 10.0);
    rounded.stroke();
    let mut by_hand: Painter<'_> = Painter::new(&faces, 100, 100).unwrap();
    by_hand.set_line_width(3.0);
    by_hand.begin_path();
    by_hand.move_to(20.0, 20.0);
    by_hand.arc(
        70.0 - reach,
        30.0,
        10.0,
        -std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI / 6.0,
    );
    by_hand.stroke();
    assert_eq!(differing(&shown(rounded).0, &shown(by_hand).0), 0);
}

#[test]
fn a_diagonal_arc_to_folded_back_on_itself_keeps_the_rest_of_the_path() {
    let canvas: Canvas = after_arc_to(&faces(), (10.0, 10.0), (50.0, 50.0), (30.0, 30.0), 10.0);
    assert_eq!(
        (at(&canvas, 30, 30) < 128, at(&canvas, 30, 70) < 128),
        (true, true)
    );
}
