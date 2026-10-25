use fontdue::{Font, FontSettings, Metrics};

use crate::canvas::Canvas;

const FACE: &[u8] = include_bytes!("../assets/LiberationSans-Regular.ttf");

pub struct Typeface {
    font: Font,
}

impl Typeface {
    pub fn embedded() -> Result<Typeface, &'static str> {
        Font::from_bytes(FACE, FontSettings::default()).map(|font: Font| Typeface { font })
    }

    pub fn width(&self, text: &str, size: f32) -> u32 {
        text.chars()
            .map(|c: char| self.font.metrics(c, size).advance_width)
            .sum::<f32>()
            .ceil() as u32
    }

    pub fn draw(
        &self,
        canvas: &mut Canvas,
        text: &str,
        size: f32,
        left: i32,
        baseline: i32,
        ink: u8,
    ) {
        let mut pen: f32 = left as f32;
        for c in text.chars() {
            let (metrics, coverage): (Metrics, Vec<u8>) = self.font.rasterize(c, size);
            let x0: i32 = pen.round() as i32 + metrics.xmin;
            let y0: i32 = baseline - metrics.height as i32 - metrics.ymin;
            for (i, &alpha) in coverage.iter().enumerate() {
                let (col, row): (usize, usize) = (i % metrics.width, i / metrics.width);
                canvas.blend(x0 + col as i32, y0 + row as i32, ink, alpha);
            }
            pen += metrics.advance_width;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::{BLACK, Geometry, WHITE};

    const AREA: Geometry = Geometry {
        width: 200,
        height: 60,
        stride: 200,
    };

    fn inked(canvas: &Canvas) -> Vec<(u32, u32)> {
        (0..AREA.height)
            .flat_map(|y: u32| (0..AREA.width).map(move |x: u32| (x, y)))
            .filter(|&(x, y): &(u32, u32)| canvas.pixel(x, y) != Some(WHITE))
            .collect()
    }

    #[test]
    fn empty_text_has_no_width_and_inks_nothing() {
        let face: Typeface = Typeface::embedded().unwrap();
        let mut canvas: Canvas = Canvas::new(AREA);
        face.draw(&mut canvas, "", 32.0, 10, 40, BLACK);
        assert_eq!((face.width("", 32.0), inked(&canvas)), (0, vec![]));
    }

    #[test]
    fn a_glyph_inks_only_above_the_baseline_and_within_its_advance() {
        let face: Typeface = Typeface::embedded().unwrap();
        let mut canvas: Canvas = Canvas::new(AREA);
        face.draw(&mut canvas, "H", 32.0, 10, 40, BLACK);
        let ink: Vec<(u32, u32)> = inked(&canvas);
        let right: u32 = 10 + face.width("H", 32.0);
        assert!(
            !ink.is_empty()
                && ink
                    .iter()
                    .all(|&(x, y): &(u32, u32)| (10..right).contains(&x) && y < 40)
        );
    }

    #[test]
    fn a_capital_stands_on_the_baseline_at_its_full_height() {
        let face: Typeface = Typeface::embedded().unwrap();
        let mut canvas: Canvas = Canvas::new(AREA);
        face.draw(&mut canvas, "H", 32.0, 10, 40, BLACK);
        let rows: Vec<u32> = inked(&canvas)
            .iter()
            .map(|&(_, y): &(u32, u32)| y)
            .collect();
        let (top, bottom): (u32, u32) = (*rows.iter().min().unwrap(), *rows.iter().max().unwrap());
        assert_eq!((bottom, top <= 40 - 20), (39, true));
    }

    #[test]
    fn a_descender_hangs_below_the_baseline() {
        let face: Typeface = Typeface::embedded().unwrap();
        let mut canvas: Canvas = Canvas::new(AREA);
        face.draw(&mut canvas, "p", 32.0, 10, 40, BLACK);
        assert!(inked(&canvas).iter().any(|&(_, y): &(u32, u32)| y > 44));
    }

    #[test]
    fn a_glyph_reaches_full_ink_in_its_stem() {
        let face: Typeface = Typeface::embedded().unwrap();
        let mut canvas: Canvas = Canvas::new(AREA);
        face.draw(&mut canvas, "H", 32.0, 10, 40, BLACK);
        assert!(
            inked(&canvas)
                .iter()
                .any(|&(x, y): &(u32, u32)| canvas.pixel(x, y) == Some(BLACK))
        );
    }

    #[test]
    fn many_glyphs_advance_left_to_right() {
        let face: Typeface = Typeface::embedded().unwrap();
        let mut canvas: Canvas = Canvas::new(AREA);
        face.draw(&mut canvas, "II", 32.0, 10, 40, BLACK);
        let one: u32 = face.width("I", 32.0);
        let columns: Vec<u32> = inked(&canvas)
            .iter()
            .map(|&(x, _): &(u32, u32)| x)
            .collect();
        let first: bool = columns.iter().any(|&x: &u32| x < 10 + one);
        let second: bool = columns.iter().any(|&x: &u32| x >= 10 + one);
        assert_eq!((first, second), (true, true));
    }
}
