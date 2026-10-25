pub const WHITE: u8 = 0xFF;
pub const BLACK: u8 = 0x00;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geometry {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
}

pub struct Canvas {
    geometry: Geometry,
    pixels: Vec<u8>,
}

impl Canvas {
    pub fn new(geometry: Geometry) -> Canvas {
        let size: usize = geometry.stride as usize * geometry.height as usize;
        Canvas {
            geometry,
            pixels: vec![WHITE; size],
        }
    }

    pub fn geometry(&self) -> Geometry {
        self.geometry
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<u8> {
        self.index(x, y).map(|i: usize| self.pixels[i])
    }

    pub fn fill_rect(&mut self, rect: Rect, gray: u8) {
        for y in self.span(rect.top, rect.height, self.geometry.height) {
            for x in self.span(rect.left, rect.width, self.geometry.width) {
                let i: usize = self.offset(x, y);
                self.pixels[i] = gray;
            }
        }
    }

    pub fn blend(&mut self, x: i32, y: i32, ink: u8, coverage: u8) {
        let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) else {
            return;
        };
        if let Some(i) = self.index(x, y) {
            let paper: u32 = u32::from(self.pixels[i]);
            let alpha: u32 = u32::from(coverage);
            let mixed: u32 = (paper * (255 - alpha) + u32::from(ink) * alpha + 127) / 255;
            self.pixels[i] = mixed as u8;
        }
    }

    fn index(&self, x: u32, y: u32) -> Option<usize> {
        (x < self.geometry.width && y < self.geometry.height).then(|| self.offset(x, y))
    }

    fn offset(&self, x: u32, y: u32) -> usize {
        y as usize * self.geometry.stride as usize + x as usize
    }

    fn span(&self, start: i32, length: u32, limit: u32) -> std::ops::Range<u32> {
        let first: i64 = i64::from(start).clamp(0, i64::from(limit));
        let end: i64 = (i64::from(start) + i64::from(length)).clamp(0, i64::from(limit));
        first as u32..end.max(first) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMALL: Geometry = Geometry {
        width: 4,
        height: 3,
        stride: 6,
    };

    fn dark(canvas: &Canvas) -> Vec<(u32, u32)> {
        let geometry: Geometry = canvas.geometry();
        (0..geometry.height)
            .flat_map(|y: u32| (0..geometry.stride).map(move |x: u32| (x, y)))
            .filter(|&(x, y): &(u32, u32)| {
                canvas.pixels()[(y * geometry.stride + x) as usize] != WHITE
            })
            .collect()
    }

    #[test]
    fn a_new_canvas_is_white_including_the_stride_padding() {
        let canvas: Canvas = Canvas::new(SMALL);
        assert_eq!(canvas.pixels(), &[WHITE; 18]);
    }

    #[test]
    fn an_empty_rect_paints_nothing() {
        let mut canvas: Canvas = Canvas::new(SMALL);
        canvas.fill_rect(
            Rect {
                left: 1,
                top: 1,
                width: 0,
                height: 2,
            },
            BLACK,
        );
        assert_eq!(dark(&canvas), vec![]);
    }

    #[test]
    fn a_one_pixel_rect_paints_that_pixel() {
        let mut canvas: Canvas = Canvas::new(SMALL);
        canvas.fill_rect(
            Rect {
                left: 2,
                top: 1,
                width: 1,
                height: 1,
            },
            0x40,
        );
        assert_eq!(
            (dark(&canvas), canvas.pixel(2, 1)),
            (vec![(2, 1)], Some(0x40))
        );
    }

    #[test]
    fn a_rect_past_the_edges_is_clipped_to_the_visible_area() {
        let mut canvas: Canvas = Canvas::new(SMALL);
        canvas.fill_rect(
            Rect {
                left: -1,
                top: 2,
                width: 9,
                height: 5,
            },
            BLACK,
        );
        assert_eq!(dark(&canvas), vec![(0, 2), (1, 2), (2, 2), (3, 2)]);
    }

    #[test]
    fn a_rect_wholly_outside_paints_nothing() {
        let mut canvas: Canvas = Canvas::new(SMALL);
        canvas.fill_rect(
            Rect {
                left: -5,
                top: -5,
                width: 3,
                height: 3,
            },
            BLACK,
        );
        assert_eq!(dark(&canvas), vec![]);
    }

    #[test]
    fn a_pixel_outside_the_visible_area_reads_as_none() {
        let canvas: Canvas = Canvas::new(SMALL);
        assert_eq!((canvas.pixel(4, 0), canvas.pixel(0, 3)), (None, None));
    }

    #[test]
    fn full_coverage_lays_down_the_ink() {
        let mut canvas: Canvas = Canvas::new(SMALL);
        canvas.blend(1, 1, BLACK, 255);
        assert_eq!(canvas.pixel(1, 1), Some(BLACK));
    }

    #[test]
    fn zero_coverage_keeps_the_paper() {
        let mut canvas: Canvas = Canvas::new(SMALL);
        canvas.blend(1, 1, BLACK, 0);
        assert_eq!(canvas.pixel(1, 1), Some(WHITE));
    }

    #[test]
    fn half_coverage_mixes_ink_and_paper() {
        let mut canvas: Canvas = Canvas::new(SMALL);
        canvas.blend(1, 1, BLACK, 128);
        assert_eq!(canvas.pixel(1, 1), Some(127));
    }

    #[test]
    fn half_coverage_of_gray_ink_mixes_both() {
        let mut canvas: Canvas = Canvas::new(SMALL);
        canvas.blend(1, 1, 0x40, 128);
        assert_eq!(canvas.pixel(1, 1), Some(159));
    }

    #[test]
    fn blending_outside_the_visible_area_touches_nothing() {
        let mut canvas: Canvas = Canvas::new(SMALL);
        canvas.blend(-1, 0, BLACK, 255);
        canvas.blend(4, 0, BLACK, 255);
        canvas.blend(0, 3, BLACK, 255);
        assert_eq!(dark(&canvas), vec![]);
    }
}
