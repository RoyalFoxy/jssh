use std::fmt::Write;
use syntect::{
    easy::HighlightLines,
    highlighting::{Color, Style, Theme},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl Highlighter {
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme: Theme = serde_json::from_str(include_str!("../theme.json")).unwrap();

        Self { syntax_set, theme }
    }

    pub fn highlight(&mut self, source: &str) -> String {
        let syntax = self.syntax_set.find_syntax_by_extension("js").unwrap();
        let mut highlight_lines = HighlightLines::new(syntax, &self.theme);

        let mut highlighted_string = String::new();

        for line in LinesWithEndings::from(source) {
            let ranges: Vec<(Style, &str)> = highlight_lines
                .highlight_line(line, &self.syntax_set)
                .unwrap();
            let escaped = Highlighter::color(&ranges[..]);
            write!(highlighted_string, "{escaped}").unwrap();
        }

        highlighted_string
    }

    fn color_to_hex(Color { r, g, b, .. }: Color) -> u32 {
        ((r as u32) << 16) + ((g as u32) << 8) + (b as u32)
    }

    fn color(v: &[(Style, &str)]) -> String {
        let mut string: String = String::new();

        for &(ref style, text) in v.iter() {
            let fg = style.foreground;

            match Highlighter::color_to_hex(fg) {
                0x000000 => write!(string, "\x1b[30m{}\x1b[0m", text).unwrap(),
                0xff0000 => write!(string, "\x1b[31m{}\x1b[0m", text).unwrap(),
                0x00ff00 => write!(string, "\x1b[32m{}\x1b[0m", text).unwrap(),
                0xffff00 => write!(string, "\x1b[33m{}\x1b[0m", text).unwrap(),
                0x0000ff => write!(string, "\x1b[34m{}\x1b[0m", text).unwrap(),
                0xff00ff => write!(string, "\x1b[35m{}\x1b[0m", text).unwrap(),
                0x00ffff => write!(string, "\x1b[36m{}\x1b[0m", text).unwrap(),
                0xffffff => write!(string, "\x1b[37m{}\x1b[0m", text).unwrap(),
                _ => unreachable!(),
            }
        }
        string
    }
}
