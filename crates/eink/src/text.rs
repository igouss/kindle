use fontdue::{Font, FontSettings, Metrics};

use crate::canvas::Canvas;

const REGULAR: &[u8] = include_bytes!("../assets/LiberationSans-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../assets/LiberationSans-Bold.ttf");

pub struct Typeface {
    font: Font,
}

impl Typeface {
    pub fn regular() -> Result<Typeface, &'static str> {
        Typeface::parse(REGULAR)
    }

    pub fn bold() -> Result<Typeface, &'static str> {
        Typeface::parse(BOLD)
    }

    fn parse(face: &[u8]) -> Result<Typeface, &'static str> {
        Font::from_bytes(face, FontSettings::default()).map(|font: Font| Typeface { font })
    }

    pub fn width(&self, text: &str, size: f32) -> u32 {
        text.chars()
            .map(|c: char| self.font.metrics(c, size).advance_width)
            .sum::<f32>()
            .ceil() as u32
    }

    pub fn wrap(&self, text: &str, size: f32, width: u32) -> Vec<String> {
        let mut lines: Vec<String> = vec![];
        let mut line: String = String::new();
        for word in text.split_whitespace() {
            let joined: String = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if self.width(&joined, size) <= width {
                line = joined;
                continue;
            }
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            for c in word.chars() {
                let longer: String = format!("{line}{c}");
                if self.width(&longer, size) > width && !line.is_empty() {
                    lines.push(std::mem::replace(&mut line, c.to_string()));
                } else {
                    line = longer;
                }
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
        lines
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
    fn wrapping_nothing_gives_no_lines() {
        let face: Typeface = Typeface::regular().unwrap();
        assert_eq!(face.wrap("  ", 24.0, 300), Vec::<String>::new());
    }

    #[test]
    fn text_that_fits_stays_on_one_line_with_single_spaces() {
        let face: Typeface = Typeface::regular().unwrap();
        assert_eq!(
            face.wrap("one  two\nthree", 24.0, 300),
            vec!["one two three"]
        );
    }

    #[test]
    fn many_words_break_at_spaces_and_every_line_fits() {
        let face: Typeface = Typeface::regular().unwrap();
        let text: &str = "the quick brown fox jumps over the lazy dog again and again";
        let lines: Vec<String> = face.wrap(text, 24.0, 160);
        assert!(
            lines.len() > 2
                && lines.iter().all(|l: &String| face.width(l, 24.0) <= 160)
                && lines.join(" ") == text
        );
    }

    #[test]
    fn a_line_exactly_as_wide_as_the_space_fits() {
        let face: Typeface = Typeface::regular().unwrap();
        let width: u32 = face.width("one two", 24.0);
        assert_eq!(
            (
                face.wrap("one two", 24.0, width),
                face.wrap("one two", 24.0, width - 1)
            ),
            (
                vec![String::from("one two")],
                vec![String::from("one"), String::from("two")]
            )
        );
    }

    #[test]
    fn letters_that_exactly_fill_the_line_stay_on_it() {
        let face: Typeface = Typeface::regular().unwrap();
        let width: u32 = face.width("abcd", 24.0);
        assert_eq!(face.wrap("abcdefgh", 24.0, width)[0], "abcd");
    }

    #[test]
    fn a_word_wider_than_the_line_is_broken_between_letters() {
        let face: Typeface = Typeface::regular().unwrap();
        let word: &str = "omarchy-expose-g3o.13-omarchy-expose-g3o.13";
        let lines: Vec<String> = face.wrap(&format!("id {word}"), 24.0, 200);
        assert!(
            lines.len() > 2
                && lines[0] == "id"
                && lines.iter().all(|l: &String| face.width(l, 24.0) <= 200)
                && lines[1..].concat() == word
        );
    }

    #[test]
    fn empty_text_has_no_width_and_inks_nothing() {
        let face: Typeface = Typeface::regular().unwrap();
        let mut canvas: Canvas = Canvas::new(AREA);
        face.draw(&mut canvas, "", 32.0, 10, 40, BLACK);
        assert_eq!((face.width("", 32.0), inked(&canvas)), (0, vec![]));
    }

    #[test]
    fn a_glyph_inks_only_above_the_baseline_and_within_its_advance() {
        let face: Typeface = Typeface::regular().unwrap();
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
        let face: Typeface = Typeface::regular().unwrap();
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
        let face: Typeface = Typeface::regular().unwrap();
        let mut canvas: Canvas = Canvas::new(AREA);
        face.draw(&mut canvas, "p", 32.0, 10, 40, BLACK);
        assert!(inked(&canvas).iter().any(|&(_, y): &(u32, u32)| y > 44));
    }

    #[test]
    fn a_glyph_reaches_full_ink_in_its_stem() {
        let face: Typeface = Typeface::regular().unwrap();
        let mut canvas: Canvas = Canvas::new(AREA);
        face.draw(&mut canvas, "H", 32.0, 10, 40, BLACK);
        assert!(
            inked(&canvas)
                .iter()
                .any(|&(x, y): &(u32, u32)| canvas.pixel(x, y) == Some(BLACK))
        );
    }

    #[test]
    fn bold_strokes_are_heavier_than_regular() {
        let (regular, bold): (Typeface, Typeface) =
            (Typeface::regular().unwrap(), Typeface::bold().unwrap());
        let mut thin: Canvas = Canvas::new(AREA);
        let mut heavy: Canvas = Canvas::new(AREA);
        regular.draw(&mut thin, "l", 32.0, 10, 40, BLACK);
        bold.draw(&mut heavy, "l", 32.0, 10, 40, BLACK);
        assert!(inked(&heavy).len() > inked(&thin).len());
    }

    #[test]
    fn the_second_glyph_starts_one_advance_after_the_first() {
        let face: Typeface = Typeface::regular().unwrap();
        let (mut one, mut two): (Canvas, Canvas) = (Canvas::new(AREA), Canvas::new(AREA));
        face.draw(&mut one, "I", 32.0, 10, 40, BLACK);
        face.draw(&mut two, "II", 32.0, 10, 40, BLACK);
        let right = |canvas: &Canvas| -> u32 {
            inked(canvas)
                .iter()
                .map(|&(x, _): &(u32, u32)| x)
                .max()
                .unwrap()
        };
        assert!(right(&two).abs_diff(right(&one) + face.width("I", 32.0)) <= 1);
    }

    #[test]
    fn many_glyphs_advance_left_to_right() {
        let face: Typeface = Typeface::regular().unwrap();
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
