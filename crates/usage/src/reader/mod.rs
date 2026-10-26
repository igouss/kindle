use franken_markdown::ast::{Block, DefinitionItem, Inline, List, ListItem, Table};

use crate::screen::Faces;

mod pages;

pub use pages::{Page, paginate, thumb};

pub const INDENT: i32 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weight {
    Regular,
    Bold,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
    pub text: String,
    pub weight: Weight,
    pub glued: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Title,
    Meta,
    Heading,
    Body,
    Code,
    Rule,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    pub size: f32,
    pub leading: i32,
}

impl Kind {
    pub fn metrics(self) -> Metrics {
        match self {
            Kind::Title => Metrics {
                size: 40.0,
                leading: 50,
            },
            Kind::Meta => Metrics {
                size: 26.0,
                leading: 38,
            },
            Kind::Heading => Metrics {
                size: 30.0,
                leading: 40,
            },
            Kind::Body => Metrics {
                size: 26.0,
                leading: 35,
            },
            Kind::Code => Metrics {
                size: 22.0,
                leading: 29,
            },
            Kind::Rule => Metrics {
                size: 0.0,
                leading: 18,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub kind: Kind,
    pub indent: i32,
    pub marker: Option<String>,
    pub words: Vec<Word>,
    pub gap: i32,
}

impl Line {
    pub fn height(&self, at_top: bool) -> i32 {
        self.kind.metrics().leading + if at_top { 0 } else { self.gap }
    }
}

pub struct Layout<'a> {
    faces: &'a Faces,
    width: i32,
    lines: Vec<Line>,
}

pub struct Source<'a> {
    pub place: &'a str,
    pub title: &'a str,
    pub meta: &'a str,
    pub description: &'a str,
}

pub fn lay(faces: &Faces, width: u32, source: &Source<'_>) -> Vec<Line> {
    let mut layout: Layout<'_> = Layout {
        faces,
        width: width as i32,
        lines: vec![],
    };
    layout.words(
        Kind::Meta,
        0,
        None,
        0,
        &plain(source.place, Weight::Regular),
    );
    layout.words(Kind::Title, 0, None, 0, &plain(source.title, Weight::Bold));
    layout.words(Kind::Meta, 0, None, 0, &plain(source.meta, Weight::Regular));
    layout.rule(0, 0);
    let document: franken_markdown::ast::Document = franken_markdown::parse(source.description);
    for (i, block) in document.blocks.iter().enumerate() {
        layout.block(block, 0, if i == 0 { 8 } else { 16 });
    }
    layout.lines
}

fn plain(text: &str, weight: Weight) -> Vec<Word> {
    let mut flow: Flow = Flow::default();
    flow.text(text, weight);
    flow.words
}

#[derive(Default)]
struct Flow {
    words: Vec<Word>,
    spaced: bool,
}

impl Flow {
    fn text(&mut self, text: &str, weight: Weight) {
        let mut chunks: std::str::SplitWhitespace<'_> = text.split_whitespace();
        let mut spaced: bool = self.spaced || text.starts_with(char::is_whitespace);
        for chunk in chunks.by_ref() {
            self.words.push(Word {
                text: chunk.to_owned(),
                weight,
                glued: !spaced && !self.words.is_empty(),
            });
            spaced = true;
        }
        self.spaced = text.ends_with(char::is_whitespace) || (self.spaced && text.is_empty());
    }

    fn inlines(&mut self, inlines: &[Inline], weight: Weight) {
        for inline in inlines {
            match inline {
                Inline::Text(text)
                | Inline::Code(text)
                | Inline::Html(text)
                | Inline::Math(text)
                | Inline::DisplayMath(text) => self.text(text, weight),
                Inline::Strong(inner) => self.inlines(inner, Weight::Bold),
                Inline::Emphasis(inner) | Inline::Strikethrough(inner) => {
                    self.inlines(inner, weight)
                }
                Inline::Link { content, .. } => self.inlines(content, weight),
                Inline::Image { alt, .. } => self.text(alt, weight),
                Inline::FootnoteRef { id } => self.text(&format!("[{id}]"), weight),
                Inline::SoftBreak | Inline::HardBreak => self.spaced = true,
            }
        }
    }
}

fn clusters(words: &[Word]) -> Vec<&[Word]> {
    let mut clusters: Vec<&[Word]> = vec![];
    let mut start: usize = 0;
    for i in 1..=words.len() {
        if i == words.len() || !words[i].glued {
            clusters.push(&words[start..i]);
            start = i;
        }
    }
    clusters
}

fn inline_words(inlines: &[Inline], weight: Weight, out: &mut Vec<Word>) {
    let mut flow: Flow = Flow {
        words: std::mem::take(out),
        spaced: true,
    };
    flow.inlines(inlines, weight);
    *out = flow.words;
}

impl Layout<'_> {
    fn rule(&mut self, indent: i32, gap: i32) {
        self.lines.push(Line {
            kind: Kind::Rule,
            indent,
            marker: None,
            words: vec![],
            gap,
        });
    }

    fn inlines(
        &mut self,
        kind: Kind,
        indent: i32,
        marker: Option<String>,
        gap: i32,
        inlines: &[Inline],
    ) {
        let weight: Weight = match kind {
            Kind::Title | Kind::Heading => Weight::Bold,
            Kind::Meta | Kind::Body | Kind::Code | Kind::Rule => Weight::Regular,
        };
        let mut words: Vec<Word> = vec![];
        inline_words(inlines, weight, &mut words);
        self.words(kind, indent, marker, gap, &words);
    }

    fn words(&mut self, kind: Kind, indent: i32, marker: Option<String>, gap: i32, words: &[Word]) {
        let size: f32 = kind.metrics().size;
        let room: u32 = (self.width - indent).max(1) as u32;
        let space: u32 = self.faces.regular.width(" ", size);
        let mut line: Vec<Word> = vec![];
        let mut used: u32 = 0;
        let mut first: bool = true;
        let mut marker: Option<String> = marker;
        for cluster in clusters(words) {
            let wide: u32 = cluster.iter().map(|w: &Word| self.measure(w, size)).sum();
            let parts: Vec<Word> = if wide > room {
                cluster
                    .iter()
                    .flat_map(|w: &Word| self.pieces(w, size, room))
                    .collect()
            } else {
                cluster.to_vec()
            };
            let groups: Vec<Vec<Word>> = if wide > room {
                parts.into_iter().map(|w: Word| vec![w]).collect()
            } else {
                vec![parts]
            };
            for group in groups {
                let width: u32 = group.iter().map(|w: &Word| self.measure(w, size)).sum();
                let needed: u32 = if line.is_empty() {
                    width
                } else {
                    used + space + width
                };
                if needed > room && !line.is_empty() {
                    self.lines.push(Line {
                        kind,
                        indent,
                        marker: marker.take(),
                        words: std::mem::take(&mut line),
                        gap: if first { gap } else { 0 },
                    });
                    first = false;
                    used = width;
                } else {
                    used = needed;
                }
                line.extend(group);
            }
        }
        if !line.is_empty() || first {
            self.lines.push(Line {
                kind,
                indent,
                marker,
                words: line,
                gap: if first { gap } else { 0 },
            });
        }
    }

    fn measure(&self, word: &Word, size: f32) -> u32 {
        match word.weight {
            Weight::Regular => self.faces.regular.width(&word.text, size),
            Weight::Bold => self.faces.bold.width(&word.text, size),
        }
    }

    fn pieces(&self, word: &Word, size: f32, room: u32) -> Vec<Word> {
        let face: &eink::text::Typeface = match word.weight {
            Weight::Regular => &self.faces.regular,
            Weight::Bold => &self.faces.bold,
        };
        face.wrap(&word.text, size, room)
            .into_iter()
            .map(|text: String| Word {
                text,
                weight: word.weight,
                glued: false,
            })
            .collect()
    }

    fn block(&mut self, block: &Block, indent: i32, gap: i32) {
        match block {
            Block::Heading { inlines, .. } => {
                self.inlines(Kind::Heading, indent, None, gap + 10, inlines)
            }
            Block::Paragraph(inlines) => self.inlines(Kind::Body, indent, None, gap, inlines),
            Block::CodeBlock { code, .. } => {
                for (i, line) in code.lines().enumerate() {
                    self.code(line, indent + INDENT / 2, if i == 0 { gap } else { 0 });
                }
            }
            Block::BlockQuote(blocks) | Block::FootnoteDefinition { blocks, .. } => {
                for (i, inner) in blocks.iter().enumerate() {
                    self.block(inner, indent + INDENT, if i == 0 { gap } else { 8 });
                }
            }
            Block::List(list) => self.list(list, indent, gap),
            Block::Table(table) => self.table(table, indent, gap),
            Block::ThematicBreak => self.rule(indent, gap),
            Block::HtmlBlock(text) | Block::MathBlock(text) => {
                self.words(Kind::Body, indent, None, gap, &plain(text, Weight::Regular))
            }
            Block::DefinitionList(items) => {
                for (i, item) in items.iter().enumerate() {
                    self.definition(item, indent, if i == 0 { gap } else { 8 });
                }
            }
            Block::PageBreak => {}
        }
    }

    fn code(&mut self, text: &str, indent: i32, gap: i32) {
        let size: f32 = Kind::Code.metrics().size;
        let room: u32 = (self.width - indent).max(1) as u32;
        let mut pieces: Vec<String> = vec![];
        let mut piece: String = String::new();
        for c in text.trim_end().chars() {
            let longer: String = format!("{piece}{c}");
            if self.faces.regular.width(&longer, size) > room && !piece.is_empty() {
                pieces.push(std::mem::replace(&mut piece, c.to_string()));
            } else {
                piece = longer;
            }
        }
        pieces.push(piece);
        for (i, piece) in pieces.into_iter().enumerate() {
            self.lines.push(Line {
                kind: Kind::Code,
                indent,
                marker: None,
                words: vec![Word {
                    text: piece,
                    weight: Weight::Regular,
                    glued: false,
                }],
                gap: if i == 0 { gap } else { 0 },
            });
        }
    }

    fn list(&mut self, list: &List, indent: i32, gap: i32) {
        for (i, item) in list.items.iter().enumerate() {
            let marker: String = match (list.ordered, item.task) {
                (_, Some(true)) => String::from("\u{2611}"),
                (_, Some(false)) => String::from("\u{2610}"),
                (true, None) => format!("{}.", list.start + i as u64),
                (false, None) => String::from("\u{2022}"),
            };
            self.item(item, marker, indent, if i == 0 { gap } else { 8 });
        }
    }

    fn item(&mut self, item: &ListItem, marker: String, indent: i32, gap: i32) {
        let mut marker: Option<String> = Some(marker);
        for (i, block) in item.blocks.iter().enumerate() {
            let inner_gap: i32 = if i == 0 { gap } else { 8 };
            match (block, marker.take()) {
                (Block::Paragraph(inlines), Some(mark)) => {
                    self.inlines(Kind::Body, indent + INDENT, Some(mark), inner_gap, inlines)
                }
                (other, Some(mark)) => {
                    self.words(Kind::Body, indent + INDENT, Some(mark), inner_gap, &[]);
                    self.block(other, indent + INDENT, 0);
                }
                (other, None) => self.block(other, indent + INDENT, inner_gap),
            }
        }
        if let Some(mark) = marker {
            self.words(Kind::Body, indent + INDENT, Some(mark), gap, &[]);
        }
    }

    fn table(&mut self, table: &Table, indent: i32, gap: i32) {
        let mut words: Vec<Word> = vec![];
        for (i, cell) in table.head.iter().enumerate() {
            if i > 0 {
                words.push(Word {
                    text: String::from("\u{b7}"),
                    weight: Weight::Bold,
                    glued: false,
                });
            }
            inline_words(cell, Weight::Bold, &mut words);
        }
        self.words(Kind::Body, indent, None, gap, &words);
        for row in &table.rows {
            let mut words: Vec<Word> = vec![];
            for (i, cell) in row.iter().enumerate() {
                if i > 0 {
                    words.extend(plain("\u{b7}", Weight::Regular));
                }
                inline_words(cell, Weight::Regular, &mut words);
            }
            self.words(Kind::Body, indent, None, 4, &words);
        }
    }

    fn definition(&mut self, item: &DefinitionItem, indent: i32, gap: i32) {
        for (i, term) in item.terms.iter().enumerate() {
            let mut words: Vec<Word> = vec![];
            inline_words(term, Weight::Bold, &mut words);
            self.words(
                Kind::Body,
                indent,
                None,
                if i == 0 { gap } else { 0 },
                &words,
            );
        }
        for definition in &item.definitions {
            self.inlines(Kind::Body, indent + INDENT, None, 0, definition);
        }
    }
}

#[cfg(test)]
mod tests;
