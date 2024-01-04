use std::{
    io,
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use termion::{event::Key, input::TermRead};

use crate::{theme::Theme, ui::WordsRender, words::WordSet};

pub fn run_test(
    word_set: &WordSet,
    word_count: usize,
    punct: bool,
    numbers: bool,
    theme: Theme,
) -> io::Result<Option<TestRawResult>> {
    let words = word_set.choose_with(word_count, punct, numbers);
    let words: Vec<_> = words.iter().map(|x| &**x).collect();
    Test::new(&words, punct, numbers, theme).run()
}

struct Test<'a> {
    words: &'a [&'a str],
    punct: bool,
    numbers: bool,
    theme: Theme,

    timer: Timer,
    input: String,
    word: usize,
    pos: usize,
}

impl<'a> Test<'a> {
    fn new(words: &'a [&'a str], punct: bool, numbers: bool, theme: Theme) -> Self {
        assert!(!words.is_empty());
        Self {
            words,
            punct,
            numbers,
            theme,
            timer: Timer::new(),
            input: String::new(),
            word: 0,
            pos: 0,
        }
    }

    fn run(mut self) -> io::Result<Option<TestRawResult>> {
        let mut steps = Vec::new();

        let (send, recv) = mpsc::channel();
        thread::spawn(move || {
            let stdin = &mut io::stdin();
            loop {
                match send.send(stdin.keys().find_map(Result::ok).unwrap()) {
                    Ok(_) => (),
                    Err(_) => return,
                }
            }
        });

        let mut render = WordsRender::new(self.words, self.theme)?;
        render.start()?;
        let quit = loop {
            render.render()?;

            let key = recv.recv_timeout(Duration::from_millis(200));
            if key.is_err() {
                continue;
            }

            match key.unwrap() {
                Key::Ctrl('c' | 'd' | 'q' | 'z') | Key::Esc => break true,
                Key::Char(c) => {
                    if !self.timer.running() {
                        self.timer.start();
                        steps.push(Step::start(0));
                    }

                    if c == ' ' && self.input == self.words[self.word] {
                        steps.push(Step::complete(self.word));
                        self.input.clear();
                        self.pos = 0;
                        self.word += 1;

                        // Test over.
                        if self.word == self.words.len() {
                            break false;
                        }

                        steps.push(Step::start(self.word));
                        render.next_word()?;
                    } else {
                        self.input.push(c);
                        let diff = diff_at(&self.input, self.words[self.word], self.pos);
                        match diff {
                            Diff::Correct(c) => render.correct(c)?,
                            Diff::Error(_, c) => render.error(c)?,
                            Diff::Extra(c) => render.extra(c)?,
                        }
                        steps.push(Step::input(diff));
                        self.pos += 1;
                    }
                }
                Key::Backspace if self.pos > 0 => {
                    self.input.pop();
                    self.pos -= 1;
                    render.undo()?;
                }
                _ => (),
            }
        };
        render.end()?;

        Ok(self.timer.stop().map(|(start, duration)| TestRawResult {
            word_count: self.words.len(),
            punct: self.punct,
            numbers: self.numbers,
            steps,
            start,
            duration,
            quit,
        }))
    }
}

#[derive(Debug)]
pub struct TestRawResult {
    pub word_count: usize,
    pub punct: bool,
    pub numbers: bool,
    pub steps: Vec<Step>,
    pub start: u64,
    pub duration: Duration,
    pub quit: bool,
}

#[derive(Debug)]
pub struct Step {
    pub kind: StepKind,
    pub instant: Instant,
}

#[derive(Debug)]
pub enum StepKind {
    Input(Diff),
    Start(usize),
    Complete(usize),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Diff {
    Correct(char),
    Error(char, char),
    Extra(char),
}

impl Step {
    #[inline]
    fn input(diff: Diff) -> Self {
        Self {
            kind: StepKind::Input(diff),
            instant: Instant::now(),
        }
    }

    #[inline]
    fn start(word: usize) -> Self {
        Self {
            kind: StepKind::Start(word),
            instant: Instant::now(),
        }
    }

    #[inline]
    fn complete(word: usize) -> Self {
        Self {
            kind: StepKind::Complete(word),
            instant: Instant::now(),
        }
    }
}

struct Timer {
    start: Option<(Instant, u64)>,
}

impl Timer {
    fn new() -> Self {
        Self { start: None }
    }

    fn start(&mut self) {
        self.start = Some((
            Instant::now(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        ));
    }

    fn stop(&mut self) -> Option<(u64, Duration)> {
        self.start
            .take()
            .map(|(start, timestamp)| (timestamp, start.elapsed()))
    }

    fn running(&self) -> bool {
        self.start.is_some()
    }
}

fn diff_at(input: &str, target: &str, i: usize) -> Diff {
    let a = input.chars().nth(i);
    let b = target.chars().nth(i);
    match (a, b) {
        (Some(c), Some(d)) if c == d => Diff::Correct(c),
        (Some(c), Some(d)) => Diff::Error(c, d),
        (Some(c), None) => Diff::Extra(c),
        (_, _) => unreachable!(),
    }
}
