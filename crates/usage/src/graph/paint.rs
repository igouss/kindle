use std::f64::consts::{FRAC_PI_2, TAU};

use eink::canvas::Canvas;
use fm_render_canvas::{
    Canvas2dContext, Color, LineCap, LineJoin, TextAlign, TextBaseline, TextMetrics,
};
use tiny_skia::{FillRule, Mask, Paint, PathBuilder, Pixmap, Stroke, StrokeDash, Transform};

use crate::screen::Faces;

const ASCENT: f64 = 0.78;
const MIDDLE: f64 = 0.36;
const DESCENT: f64 = 0.22;

#[derive(Clone)]
struct Style {
    fill: u8,
    fill_alpha: f64,
    stroke: u8,
    stroke_alpha: f64,
    width: f64,
    cap: LineCap,
    join: LineJoin,
    dash: Vec<f64>,
    alpha: f64,
    size: f64,
    bold: bool,
    align: TextAlign,
    baseline: TextBaseline,
    transform: Transform,
    clip: Option<Mask>,
}

pub struct Label {
    pub text: String,
    pub left: i32,
    pub baseline: i32,
    pub size: f32,
    pub bold: bool,
    pub ink: u8,
}

pub struct Painter<'a> {
    faces: &'a Faces,
    pixmap: Pixmap,
    style: Style,
    saved: Vec<Style>,
    path: PathBuilder,
    pen: Option<(f32, f32)>,
    pub labels: Vec<Label>,
}

pub fn gray(css: &str) -> (u8, f64) {
    match Color::from_css(css) {
        Some(c) => {
            let luma: f64 =
                0.299 * f64::from(c.r) + 0.587 * f64::from(c.g) + 0.114 * f64::from(c.b);
            (luma.round() as u8, c.a)
        }
        None => (0, 1.0),
    }
}

pub fn font(css: &str) -> (f64, bool) {
    let size: f64 = css
        .split_whitespace()
        .find_map(|token: &str| token.strip_suffix("px")?.parse::<f64>().ok())
        .unwrap_or(16.0);
    let bold: bool = css.split_whitespace().any(|token: &str| {
        token == "bold" || token.parse::<u32>().is_ok_and(|weight: u32| weight >= 600)
    });
    (size, bold)
}

impl<'a> Painter<'a> {
    pub fn new(faces: &'a Faces, width: u32, height: u32) -> Option<Painter<'a>> {
        let mut pixmap: Pixmap = Pixmap::new(width.max(1), height.max(1))?;
        pixmap.fill(tiny_skia::Color::WHITE);
        Some(Painter {
            faces,
            pixmap,
            style: Style {
                fill: 0,
                fill_alpha: 1.0,
                stroke: 0,
                stroke_alpha: 1.0,
                width: 1.0,
                cap: LineCap::Butt,
                join: LineJoin::Miter,
                dash: vec![],
                alpha: 1.0,
                size: 16.0,
                bold: false,
                align: TextAlign::Left,
                baseline: TextBaseline::Alphabetic,
                transform: Transform::identity(),
                clip: None,
            },
            saved: vec![],
            path: PathBuilder::new(),
            pen: None,
            labels: vec![],
        })
    }

    pub fn into_canvas(self, canvas: &mut Canvas, left: i32, top: i32) -> Vec<Label> {
        let width: u32 = self.pixmap.width();
        for (i, pixel) in self.pixmap.pixels().iter().enumerate() {
            let c: tiny_skia::ColorU8 = pixel.demultiply();
            let luma: f64 = 0.299 * f64::from(c.red())
                + 0.587 * f64::from(c.green())
                + 0.114 * f64::from(c.blue());
            let x: i32 = left + (i as u32 % width) as i32;
            let y: i32 = top + (i as u32 / width) as i32;
            canvas.blend(x, y, 0, 255 - luma.round() as u8);
        }
        self.labels
    }

    fn magnification(&self) -> f64 {
        let t: Transform = self.style.transform;
        f64::from((t.sx * t.sy - t.kx * t.ky).abs().sqrt())
    }

    fn point(&self, x: f64, y: f64) -> (f32, f32) {
        let mut p: tiny_skia::Point = tiny_skia::Point::from_xy(x as f32, y as f32);
        self.style.transform.map_point(&mut p);
        (p.x, p.y)
    }

    fn paint(&self, value: u8, alpha: f64) -> Paint<'static> {
        let mut paint: Paint<'static> = Paint::default();
        let a: u8 = (alpha * self.style.alpha * 255.0).round().clamp(0.0, 255.0) as u8;
        paint.set_color_rgba8(value, value, value, a);
        paint.anti_alias = true;
        paint
    }

    fn peek_path(&self) -> Option<tiny_skia::Path> {
        self.path.clone().finish()
    }

    fn cubic(&mut self, c1: (f64, f64), c2: (f64, f64), to: (f64, f64)) {
        let (a, b): (f32, f32) = self.point(c1.0, c1.1);
        let (c, d): (f32, f32) = self.point(c2.0, c2.1);
        let (e, f): (f32, f32) = self.point(to.0, to.1);
        self.path.cubic_to(a, b, c, d, e, f);
        self.pen = Some((e, f));
    }

    fn line_or_move(&mut self, x: f64, y: f64) {
        match self.pen {
            Some(_) => self.line_to(x, y),
            None => self.move_to(x, y),
        }
    }

    fn arc_segments(&mut self, cx: f64, cy: f64, r: f64, start: f64, sweep: f64) {
        let pieces: usize = (sweep / FRAC_PI_2).ceil().max(1.0) as usize;
        let step: f64 = sweep / pieces as f64;
        let k: f64 = 4.0 / 3.0 * (step / 4.0).tan() * r;
        self.line_or_move(cx + r * start.cos(), cy + r * start.sin());
        for i in 0..pieces {
            let a0: f64 = start + step * i as f64;
            let a1: f64 = a0 + step;
            let (s0, c0, s1, c1): (f64, f64, f64, f64) = (a0.sin(), a0.cos(), a1.sin(), a1.cos());
            self.cubic(
                (cx + r * c0 - k * s0, cy + r * s0 + k * c0),
                (cx + r * c1 + k * s1, cy + r * s1 - k * c1),
                (cx + r * c1, cy + r * s1),
            );
        }
    }

    fn stroke_settings(&self) -> Stroke {
        let scale: f64 = self.magnification();
        Stroke {
            width: (self.style.width * scale).max(0.5) as f32,
            line_cap: match self.style.cap {
                LineCap::Butt => tiny_skia::LineCap::Butt,
                LineCap::Round => tiny_skia::LineCap::Round,
                LineCap::Square => tiny_skia::LineCap::Square,
            },
            line_join: match self.style.join {
                LineJoin::Miter => tiny_skia::LineJoin::Miter,
                LineJoin::Round => tiny_skia::LineJoin::Round,
                LineJoin::Bevel => tiny_skia::LineJoin::Bevel,
            },
            dash: StrokeDash::new(
                self.style
                    .dash
                    .iter()
                    .map(|d: &f64| (d * scale) as f32)
                    .collect(),
                0.0,
            ),
            ..Stroke::default()
        }
    }

    fn label(&mut self, text: &str, x: f64, y: f64, ink: u8) {
        let face: &eink::text::Typeface = if self.style.bold {
            &self.faces.bold
        } else {
            &self.faces.regular
        };
        let size: f64 = self.style.size;
        let width: f64 = f64::from(face.width(text, size as f32));
        let left: f64 = match self.style.align {
            TextAlign::Left => x,
            TextAlign::Center => x - width / 2.0,
            TextAlign::Right => x - width,
        };
        let baseline: f64 = match self.style.baseline {
            TextBaseline::Top => y + size * ASCENT,
            TextBaseline::Middle => y + size * MIDDLE,
            TextBaseline::Alphabetic => y,
            TextBaseline::Bottom => y - size * DESCENT,
        };
        let (dx, dy): (f32, f32) = self.point(left, baseline);
        self.labels.push(Label {
            text: text.to_owned(),
            left: dx.round() as i32,
            baseline: dy.round() as i32,
            size: (size * self.magnification()) as f32,
            bold: self.style.bold,
            ink,
        });
    }
}

impl Canvas2dContext for Painter<'_> {
    fn width(&self) -> f64 {
        f64::from(self.pixmap.width())
    }

    fn height(&self) -> f64 {
        f64::from(self.pixmap.height())
    }

    fn save(&mut self) {
        self.saved.push(self.style.clone());
    }

    fn restore(&mut self) {
        if let Some(style) = self.saved.pop() {
            self.style = style;
        }
    }

    fn set_fill_style(&mut self, color: &str) {
        (self.style.fill, self.style.fill_alpha) = gray(color);
    }

    fn set_stroke_style(&mut self, color: &str) {
        (self.style.stroke, self.style.stroke_alpha) = gray(color);
    }

    fn set_line_width(&mut self, width: f64) {
        self.style.width = width;
    }

    fn set_line_cap(&mut self, cap: LineCap) {
        self.style.cap = cap;
    }

    fn set_line_join(&mut self, join: LineJoin) {
        self.style.join = join;
    }

    fn set_line_dash(&mut self, pattern: &[f64]) {
        self.style.dash = pattern.to_vec();
    }

    fn set_global_alpha(&mut self, alpha: f64) {
        self.style.alpha = alpha.clamp(0.0, 1.0);
    }

    fn set_font(&mut self, font: &str) {
        (self.style.size, self.style.bold) = self::font(font);
    }

    fn set_text_align(&mut self, align: TextAlign) {
        self.style.align = align;
    }

    fn set_text_baseline(&mut self, baseline: TextBaseline) {
        self.style.baseline = baseline;
    }

    fn begin_path(&mut self) {
        self.path = PathBuilder::new();
        self.pen = None;
    }

    fn close_path(&mut self) {
        self.path.close();
    }

    fn move_to(&mut self, x: f64, y: f64) {
        let (px, py): (f32, f32) = self.point(x, y);
        self.path.move_to(px, py);
        self.pen = Some((px, py));
    }

    fn line_to(&mut self, x: f64, y: f64) {
        let (px, py): (f32, f32) = self.point(x, y);
        match self.pen {
            Some(_) => self.path.line_to(px, py),
            None => self.path.move_to(px, py),
        }
        self.pen = Some((px, py));
    }

    fn quadratic_curve_to(&mut self, cpx: f64, cpy: f64, x: f64, y: f64) {
        let (a, b): (f32, f32) = self.point(cpx, cpy);
        let (c, d): (f32, f32) = self.point(x, y);
        self.path.quad_to(a, b, c, d);
        self.pen = Some((c, d));
    }

    fn bezier_curve_to(&mut self, cp1x: f64, cp1y: f64, cp2x: f64, cp2y: f64, x: f64, y: f64) {
        self.cubic((cp1x, cp1y), (cp2x, cp2y), (x, y));
    }

    fn arc(&mut self, x: f64, y: f64, radius: f64, start_angle: f64, end_angle: f64) {
        let span: f64 = end_angle - start_angle;
        let sweep: f64 = if span >= TAU {
            TAU
        } else {
            span.rem_euclid(TAU)
        };
        self.arc_segments(x, y, radius.abs(), start_angle, sweep);
    }

    fn arc_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, radius: f64) {
        let Some((px, py)) = self.pen else {
            return self.move_to(x1, y1);
        };
        let inverse: Option<Transform> = self.style.transform.invert();
        let mut p0: tiny_skia::Point = tiny_skia::Point::from_xy(px, py);
        if let Some(inv) = inverse {
            inv.map_point(&mut p0);
        }
        let (x0, y0): (f64, f64) = (f64::from(p0.x), f64::from(p0.y));
        let (ax, ay, bx, by): (f64, f64, f64, f64) = (x0 - x1, y0 - y1, x2 - x1, y2 - y1);
        let (la, lb): (f64, f64) = (ax.hypot(ay), bx.hypot(by));
        let cross: f64 = ax * by - ay * bx;
        if radius <= 0.0 || cross.abs() < 1e-9 {
            return self.line_to(x1, y1);
        }
        let (ux, uy, vx, vy): (f64, f64, f64, f64) = (ax / la, ay / la, bx / lb, by / lb);
        let theta: f64 = (ux * vx + uy * vy).clamp(-1.0, 1.0).acos();
        let tangent: f64 = radius / (theta / 2.0).tan();
        let (t1x, t1y): (f64, f64) = (x1 + ux * tangent, y1 + uy * tangent);
        let (t2x, t2y): (f64, f64) = (x1 + vx * tangent, y1 + vy * tangent);
        let turn: f64 = std::f64::consts::PI - theta;
        let k: f64 = 4.0 / 3.0 * (turn / 4.0).tan() * radius;
        self.line_to(t1x, t1y);
        self.cubic(
            (t1x - ux * k, t1y - uy * k),
            (t2x - vx * k, t2y - vy * k),
            (t2x, t2y),
        );
    }

    fn rect(&mut self, x: f64, y: f64, width: f64, height: f64) {
        self.move_to(x, y);
        self.line_to(x + width, y);
        self.line_to(x + width, y + height);
        self.line_to(x, y + height);
        self.close_path();
    }

    fn fill(&mut self) {
        let Some(path) = self.peek_path() else {
            return;
        };
        let paint: Paint<'static> = self.paint(self.style.fill, self.style.fill_alpha);
        self.pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            self.style.clip.as_ref(),
        );
    }

    fn stroke(&mut self) {
        let Some(path) = self.peek_path() else {
            return;
        };
        let paint: Paint<'static> = self.paint(self.style.stroke, self.style.stroke_alpha);
        let stroke: Stroke = self.stroke_settings();
        self.pixmap.stroke_path(
            &path,
            &paint,
            &stroke,
            Transform::identity(),
            self.style.clip.as_ref(),
        );
    }

    fn fill_rect(&mut self, x: f64, y: f64, width: f64, height: f64) {
        let kept: PathBuilder = std::mem::replace(&mut self.path, PathBuilder::new());
        let pen: Option<(f32, f32)> = self.pen.take();
        self.rect(x, y, width, height);
        self.fill();
        self.path = kept;
        self.pen = pen;
    }

    fn stroke_rect(&mut self, x: f64, y: f64, width: f64, height: f64) {
        let kept: PathBuilder = std::mem::replace(&mut self.path, PathBuilder::new());
        let pen: Option<(f32, f32)> = self.pen.take();
        self.rect(x, y, width, height);
        self.stroke();
        self.path = kept;
        self.pen = pen;
    }

    fn clear_rect(&mut self, x: f64, y: f64, width: f64, height: f64) {
        let kept: Style = self.style.clone();
        self.style.fill = 255;
        self.style.fill_alpha = 1.0;
        self.style.alpha = 1.0;
        self.fill_rect(x, y, width, height);
        self.style = kept;
    }

    fn fill_text(&mut self, text: &str, x: f64, y: f64) {
        let ink: u8 = self.style.fill;
        self.label(text, x, y, ink);
    }

    fn stroke_text(&mut self, text: &str, x: f64, y: f64) {
        let ink: u8 = self.style.stroke;
        self.label(text, x, y, ink);
    }

    fn measure_text(&self, text: &str) -> TextMetrics {
        let face: &eink::text::Typeface = if self.style.bold {
            &self.faces.bold
        } else {
            &self.faces.regular
        };
        TextMetrics {
            width: f64::from(face.width(text, self.style.size as f32)),
            height: self.style.size,
        }
    }

    fn set_transform(&mut self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) {
        self.style.transform =
            Transform::from_row(a as f32, b as f32, c as f32, d as f32, e as f32, f as f32);
    }

    fn reset_transform(&mut self) {
        self.style.transform = Transform::identity();
    }

    fn translate(&mut self, x: f64, y: f64) {
        self.style.transform = self.style.transform.pre_translate(x as f32, y as f32);
    }

    fn scale(&mut self, x: f64, y: f64) {
        self.style.transform = self.style.transform.pre_scale(x as f32, y as f32);
    }

    fn rotate(&mut self, angle: f64) {
        self.style.transform = self
            .style
            .transform
            .pre_concat(Transform::from_rotate(angle.to_degrees() as f32));
    }

    fn clip(&mut self) {
        let Some(path) = self.peek_path() else {
            return;
        };
        let mut mask: Mask = match self.style.clip.take() {
            Some(mask) => mask,
            None => match Mask::new(self.pixmap.width(), self.pixmap.height()) {
                Some(mut fresh) => {
                    fresh.data_mut().fill(255);
                    fresh
                }
                None => return,
            },
        };
        mask.intersect_path(&path, FillRule::Winding, true, Transform::identity());
        self.style.clip = Some(mask);
    }

    fn set_shadow_blur(&mut self, _blur: f64) {}

    fn set_shadow_color(&mut self, _color: &str) {}

    fn set_shadow_offset(&mut self, _x: f64, _y: f64) {}
}

#[cfg(test)]
mod arcs;
#[cfg(test)]
mod tests;
