use eink::canvas::{Point, Rect};

pub const BUTTON: Rect = Rect {
    left: 934,
    top: 0,
    width: 90,
    height: 110,
};
pub const PANEL: Rect = Rect {
    left: 594,
    top: 120,
    width: 400,
    height: 450,
};
pub const ORIENTATION_BASELINE: i32 = 160;
pub const LIGHT_BASELINE: i32 = 295;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    PortLeft,
    PortRight,
    Dimmer,
    LightSwitch,
    Brighter,
    Close,
    KindleUi,
}

pub const TARGETS: [(Target, Rect); 7] = [
    (
        Target::PortLeft,
        Rect {
            left: 614,
            top: 175,
            width: 175,
            height: 70,
        },
    ),
    (
        Target::PortRight,
        Rect {
            left: 799,
            top: 175,
            width: 175,
            height: 70,
        },
    ),
    (
        Target::Dimmer,
        Rect {
            left: 614,
            top: 310,
            width: 100,
            height: 70,
        },
    ),
    (
        Target::LightSwitch,
        Rect {
            left: 724,
            top: 310,
            width: 140,
            height: 70,
        },
    ),
    (
        Target::Brighter,
        Rect {
            left: 874,
            top: 310,
            width: 100,
            height: 70,
        },
    ),
    (
        Target::Close,
        Rect {
            left: 614,
            top: 400,
            width: 360,
            height: 70,
        },
    ),
    (
        Target::KindleUi,
        Rect {
            left: 614,
            top: 480,
            width: 360,
            height: 70,
        },
    ),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Button,
    Target(Target),
    Panel,
    Outside,
}

pub fn contains(rect: Rect, point: Point) -> bool {
    (rect.left..rect.left + rect.width as i32).contains(&point.x)
        && (rect.top..rect.top + rect.height as i32).contains(&point.y)
}

pub fn hit(point: Point) -> Hit {
    if contains(BUTTON, point) {
        return Hit::Button;
    }
    match TARGETS
        .iter()
        .find(|(_, rect): &&(Target, Rect)| contains(*rect, point))
    {
        Some(&(target, _)) => Hit::Target(target),
        None if contains(PANEL, point) => Hit::Panel,
        None => Hit::Outside,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn centre(rect: Rect) -> Point {
        Point {
            x: rect.left + rect.width as i32 / 2,
            y: rect.top + rect.height as i32 / 2,
        }
    }

    #[test]
    fn each_target_is_hit_at_its_centre() {
        let hits: Vec<Hit> = TARGETS
            .iter()
            .map(|&(_, rect): &(Target, Rect)| hit(centre(rect)))
            .collect();
        let expected: Vec<Hit> = TARGETS
            .iter()
            .map(|&(target, _): &(Target, Rect)| Hit::Target(target))
            .collect();
        assert_eq!(hits, expected);
    }

    #[test]
    fn a_rect_includes_its_top_left_corner_and_excludes_its_far_edges() {
        let rect: Rect = Rect {
            left: 10,
            top: 20,
            width: 5,
            height: 4,
        };
        let probes: [bool; 4] = [
            contains(rect, Point { x: 10, y: 20 }),
            contains(rect, Point { x: 14, y: 23 }),
            contains(rect, Point { x: 15, y: 23 }),
            contains(rect, Point { x: 14, y: 24 }),
        ];
        assert_eq!(probes, [true, true, false, false]);
    }

    #[test]
    fn the_top_right_corner_is_the_menu_button() {
        assert_eq!(
            (hit(Point { x: 1023, y: 0 }), hit(Point { x: 934, y: 109 })),
            (Hit::Button, Hit::Button)
        );
    }

    #[test]
    fn the_gap_between_buttons_is_the_panel_and_beyond_it_is_outside() {
        let probes: [Hit; 3] = [
            hit(Point { x: 794, y: 200 }),
            hit(Point { x: 600, y: 560 }),
            hit(Point { x: 300, y: 400 }),
        ];
        assert_eq!(probes, [Hit::Panel, Hit::Panel, Hit::Outside]);
    }

    #[test]
    fn every_target_lies_inside_the_panel_and_none_overlap() {
        let inside: bool = TARGETS.iter().all(|&(_, r): &(Target, Rect)| {
            contains(
                PANEL,
                Point {
                    x: r.left,
                    y: r.top,
                },
            ) && contains(
                PANEL,
                Point {
                    x: r.left + r.width as i32 - 1,
                    y: r.top + r.height as i32 - 1,
                },
            )
        });
        let overlapping: usize = TARGETS
            .iter()
            .flat_map(|a: &(Target, Rect)| TARGETS.iter().map(move |b: &(Target, Rect)| (a, b)))
            .filter(|(a, b): &(&(Target, Rect), &(Target, Rect))| {
                a.0 != b.0 && contains(a.1, centre(b.1))
            })
            .count();
        assert_eq!((inside, overlapping), (true, 0));
    }
}
