use std::io::{self, Write};

use termion::{
    clear, color, cursor,
    raw::{IntoRawMode, RawTerminal},
    screen::AlternateScreen,
    style,
};
use unicode_width::UnicodeWidthChar;

use crate::theme::Theme;

/// Interface for rendering the typing box.
pub struct WordsRender {
    /// Render destination.
    screen: AlternateScreen<RawTerminal<io::Stdout>>,
    /// Words to be rendered.
    words: Vec<Word>,
    /// Stores the computed word wrap.
    lines: Vec<Line>,
    /// Current word.
    word: usize,
    /// Current line.
    line: usize,
    /// Current position on line.
    pos: usize,
    /// Styling for the test.
    theme: Theme,
}

impl WordsRender {
    pub fn new(words: &[&str], theme: Theme) -> io::Result<Self> {
        let stdout = io::stdout().into_raw_mode()?;
        let mut render = Self {
            screen: AlternateScreen::from(stdout),
            words: words.iter().map(|&word| word.into()).collect(),
            lines: Vec::new(),
            word: 0,
            line: 0,
            pos: 0,
            theme,
        };
        render.update_lines()?;
        Ok(render)
    }

    pub fn start(&mut self) -> io::Result<()> {
        write!(self.screen, "{}", cursor::SteadyBar)?;
        self.bg()?;
        self.render()
    }

    pub fn end(&mut self) -> io::Result<()> {
        write!(self.screen, "{}", cursor::SteadyBlock)?;
        self.flush()
    }

    pub fn correct(&mut self, c: char) -> io::Result<()> {
        self.get_word_mut().push(c, Style::Correct);
        self.cursor_forward()
    }

    pub fn error(&mut self, c: char) -> io::Result<()> {
        self.get_word_mut().push(c, Style::Error);
        self.cursor_forward()
    }

    pub fn extra(&mut self, c: char) -> io::Result<()> {
        self.get_word_mut().push(c, Style::Extra);
        self.cursor_forward()
    }

    pub fn undo(&mut self) -> io::Result<()> {
        if self.get_word_mut().pop() {
            self.cursor_back()?;
        }
        Ok(())
    }

    pub fn next_word(&mut self) -> io::Result<()> {
        self.word += 1;
        self.cursor_forward()
    }

    pub fn render(&mut self) -> io::Result<()> {
        self.update_lines()?;
        let (col, row) = termion::terminal_size()?;
        let width = col / 2;

        write!(self.screen, "{}", clear::All)?;
        self.bg()?;
        for (i, line) in self.lines.iter().enumerate() {
            write!(
                self.screen,
                "{}",
                cursor::Goto(width - width / 2, row / 2 + i as u16),
            )?;
            for word in &self.words[line.start..line.end] {
                for &(c, style) in &word.chars {
                    match style {
                        Style::Correct => {
                            write!(self.screen, "{}{}", color::Fg(self.theme.correct), c)?;
                        }
                        Style::Error => {
                            write!(self.screen, "{}{}", color::Fg(self.theme.error), c)?;
                        }
                        Style::Extra => {
                            write!(
                                self.screen,
                                "{}{}{}{}",
                                color::Fg(self.theme.extra),
                                style::Underline,
                                c,
                                style::NoUnderline,
                            )?;
                        }
                        Style::Empty => {
                            write!(self.screen, "{}{}", color::Fg(self.theme.empty), c)?;
                        }
                    }
                }
                write!(self.screen, " ")?;
            }
        }
        write!(
            self.screen,
            "{}",
            cursor::Goto(
                width - width / 2 + self.pos as u16,
                row / 2 + self.line as u16
            )
        )?;
        self.flush()
    }

    fn update_lines(&mut self) -> io::Result<()> {
        let (col, _) = termion::terminal_size()?;
        let width = col / 2;
        self.lines = wrap(&self.words, width as usize);
        Ok(())
    }

    fn cursor_forward(&mut self) -> io::Result<()> {
        self.update_lines()?;
        let c = self.get_word().last().unwrap();
        let width = c.width().unwrap();
        if self.pos + width < self.lines[self.line].len {
            self.pos += width;
        } else if self.line < self.lines.len() - 1 {
            self.line += 1;
            self.pos = self.get_word().width();
        }
        Ok(())
    }

    fn cursor_back(&mut self) -> io::Result<()> {
        self.update_lines()?;
        let c = self.get_word().last().unwrap();
        let width = c.width().unwrap();
        if self.pos >= width {
            self.pos -= width;
        } else if self.line > 0 {
            self.line -= 1;
            self.pos = self.lines[self.line].len;
        }
        Ok(())
    }

    fn get_word(&self) -> &Word {
        &self.words[self.word]
    }

    fn get_word_mut(&mut self) -> &mut Word {
        &mut self.words[self.word]
    }

    fn bg(&mut self) -> io::Result<()> {
        if let Some(bg) = self.theme.bg {
            write!(self.screen, "{}", color::Bg(bg))?;
        }
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.screen.flush()
    }
}

#[derive(Debug)]
struct Word {
    initial: Vec<char>,
    chars: Vec<(char, Style)>,
    pos: usize,
}

impl Word {
    fn width(&self) -> usize {
        self.chars.iter().map(|(c, _)| c.width().unwrap()).sum()
    }

    fn last(&self) -> Option<char> {
        self.chars.last().map(|(c, _)| *c)
    }

    fn push(&mut self, c: char, style: Style) {
        match self.chars.get_mut(self.pos) {
            Some(e) => *e = (c, style),
            None => self.chars.push((c, style)),
        }
        self.pos += 1;
    }

    fn pop(&mut self) -> bool {
        if self.pos > 0 {
            match self.chars.get_mut(self.pos - 1) {
                Some(e) => {
                    if self.pos - 1 < self.initial.len() {
                        *e = (self.initial[self.pos - 1], Style::Empty);
                    } else {
                        self.chars.pop();
                    }
                    self.pos -= 1;
                    true
                }
                None => false,
            }
        } else {
            false
        }
    }
}

impl From<&str> for Word {
    fn from(s: &str) -> Self {
        Self {
            initial: s.chars().collect(),
            chars: s.chars().map(|c| (c, Style::Empty)).collect(),
            pos: 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Style {
    Correct,
    Error,
    Extra,
    Empty,
}

#[derive(Clone, Copy)]
struct Line {
    /// Index of word starting the line.
    start: usize,
    /// Index of word ending the line.
    end: usize,
    /// Length of the line in chars.
    len: usize,
}

fn wrap(words: &[Word], width: usize) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut line_width = 0;
    for (i, word) in words.iter().enumerate() {
        let word_width = word.width();
        if i > start && line_width + word_width > width {
            lines.push(Line {
                start,
                end: i,
                len: line_width,
            });
            start = i;
            line_width = 0;
        }
        line_width += word_width + 1;
    }
    lines.push(Line {
        start,
        end: words.len(),
        len: line_width,
    });
    lines
}
