use std::time::Duration;

use crate::canvas::Point;

pub const EVENT_SIZE: usize = 16;
const EV_SYN: u16 = 0;
const EV_KEY: u16 = 1;
const EV_ABS: u16 = 3;
const SYN_REPORT: u16 = 0;
const BTN_TOUCH: u16 = 330;
const ABS_MT_POSITION_X: u16 = 53;
const ABS_MT_POSITION_Y: u16 = 54;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputEvent {
    pub time: Duration,
    pub kind: u16,
    pub code: u16,
    pub value: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TouchDown {
    pub at: Point,
    pub time: Duration,
}

pub fn parse(bytes: &[u8; EVENT_SIZE]) -> InputEvent {
    let word = |i: usize| -> u32 {
        u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]])
    };
    InputEvent {
        time: Duration::from_secs(u64::from(word(0))) + Duration::from_micros(u64::from(word(4))),
        kind: u16::from_le_bytes([bytes[8], bytes[9]]),
        code: u16::from_le_bytes([bytes[10], bytes[11]]),
        value: word(12) as i32,
    }
}

#[derive(Default)]
pub struct Frames {
    x: Option<i32>,
    y: Option<i32>,
    pressed: bool,
}

impl Frames {
    pub fn feed(&mut self, event: InputEvent) -> Option<TouchDown> {
        match (event.kind, event.code) {
            (EV_ABS, ABS_MT_POSITION_X) => self.x = Some(event.value),
            (EV_ABS, ABS_MT_POSITION_Y) => self.y = Some(event.value),
            (EV_KEY, BTN_TOUCH) => self.pressed = event.value == 1,
            (EV_SYN, SYN_REPORT) if self.pressed => {
                if let (Some(x), Some(y)) = (self.x, self.y) {
                    self.pressed = false;
                    return Some(TouchDown {
                        at: Point { x, y },
                        time: event.time,
                    });
                }
            }
            _ => {}
        }
        None
    }
}

pub struct Debounce {
    quiet: Duration,
    until: Duration,
}

impl Debounce {
    pub fn new(quiet: Duration) -> Debounce {
        Debounce {
            quiet,
            until: Duration::ZERO,
        }
    }

    pub fn accept(&mut self, touch: TouchDown) -> bool {
        let fresh: bool = touch.time >= self.until;
        self.until = self.until.max(touch.time + self.quiet);
        fresh
    }

    pub fn hold(&mut self, until: Duration) {
        self.until = self.until.max(until);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn events(fixture: &[u8]) -> Vec<InputEvent> {
        fixture
            .as_chunks::<EVENT_SIZE>()
            .0
            .iter()
            .map(parse)
            .collect()
    }

    fn downs(fixture: &[u8]) -> Vec<TouchDown> {
        let mut frames: Frames = Frames::default();
        events(fixture)
            .into_iter()
            .filter_map(|e: InputEvent| frames.feed(e))
            .collect()
    }

    fn down(x: i32, y: i32, millis: u64) -> TouchDown {
        TouchDown {
            at: Point { x, y },
            time: Duration::from_millis(millis),
        }
    }

    #[test]
    fn an_event_reads_the_kindles_32_bit_little_endian_layout() {
        let bytes: [u8; 16] = [
            0x10, 0, 0, 0, 0x20, 0xA1, 0x07, 0, 3, 0, 53, 0, 0xFF, 0xFF, 0xFF, 0xFF,
        ];
        let expected: InputEvent = InputEvent {
            time: Duration::from_micros(16_500_000),
            kind: 3,
            code: 53,
            value: -1,
        };
        assert_eq!(parse(&bytes), expected);
    }

    #[test]
    fn the_tap_on_updated_is_one_touch_down_where_it_landed() {
        let found: Vec<Point> = downs(include_bytes!("../fixtures/tap-updated.raw"))
            .iter()
            .map(|d: &TouchDown| d.at)
            .collect();
        assert_eq!(found, vec![Point { x: 669, y: 907 }]);
    }

    #[test]
    fn the_swipes_are_twenty_touch_downs_each_at_its_first_position() {
        let found: Vec<TouchDown> = downs(include_bytes!("../fixtures/swipes.raw"));
        assert_eq!(
            (found.len(), found[0].at, found[19].at),
            (20, Point { x: 568, y: 418 }, Point { x: 368, y: 628 })
        );
    }

    #[test]
    fn a_frame_without_a_position_waits_for_one() {
        let mut frames: Frames = Frames::default();
        let at = |kind: u16, code: u16, value: i32| InputEvent {
            time: Duration::from_millis(5),
            kind,
            code,
            value,
        };
        let first: Option<TouchDown> = [at(1, 330, 1), at(0, 0, 0)]
            .into_iter()
            .find_map(|e: InputEvent| frames.feed(e));
        let second: Option<TouchDown> = [at(3, 53, 7), at(3, 54, 9), at(0, 0, 0)]
            .into_iter()
            .find_map(|e: InputEvent| frames.feed(e));
        assert_eq!((first, second), (None, Some(down(7, 9, 5))));
    }

    #[test]
    fn a_release_reports_nothing() {
        let mut frames: Frames = Frames::default();
        let at = |kind: u16, code: u16, value: i32| InputEvent {
            time: Duration::ZERO,
            kind,
            code,
            value,
        };
        let found: Vec<TouchDown> = [at(3, 53, 1), at(3, 54, 2), at(1, 330, 0), at(0, 0, 0)]
            .into_iter()
            .filter_map(|e: InputEvent| frames.feed(e))
            .collect();
        assert_eq!(found, vec![]);
    }

    #[test]
    fn the_first_touch_is_accepted() {
        assert!(Debounce::new(Duration::from_millis(600)).accept(down(1, 1, 1000)));
    }

    #[test]
    fn a_burst_of_touches_counts_once_however_long_it_lasts() {
        let mut debounce: Debounce = Debounce::new(Duration::from_millis(600));
        let accepted: Vec<bool> = [0, 400, 800, 1200, 1900]
            .map(|t: u64| debounce.accept(down(1, 1, 1000 + t)))
            .to_vec();
        assert_eq!(accepted, vec![true, false, false, false, true]);
    }

    #[test]
    fn a_hold_ignores_touches_made_before_it_ends() {
        let mut debounce: Debounce = Debounce::new(Duration::from_millis(600));
        debounce.hold(Duration::from_millis(5000));
        assert_eq!(
            (
                debounce.accept(down(1, 1, 4999)),
                debounce.accept(down(1, 1, 5600))
            ),
            (false, true)
        );
    }

    #[test]
    fn the_swipes_debounce_to_the_two_strokes() {
        let mut debounce: Debounce = Debounce::new(Duration::from_millis(300));
        let strokes: usize = downs(include_bytes!("../fixtures/swipes.raw"))
            .into_iter()
            .filter(|d: &TouchDown| debounce.accept(*d))
            .count();
        assert_eq!(strokes, 2);
    }
}
