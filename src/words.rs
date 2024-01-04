use std::{
    fmt,
    fs::File,
    io::{self, BufRead},
    path::{Path, PathBuf},
};

use crate::rand;

pub struct WordSet {
    words: Vec<String>,
}

impl WordSet {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, WordSetError> {
        let file = File::open(&path)
            .map_err(|e| WordSetError::Open(path.as_ref().into(), e.to_string()))?;
        let words: Vec<_> = io::BufReader::new(file)
            .lines()
            .filter_map(Result::ok)
            .map(|s| s.trim().into())
            .collect();
        Ok(Self { words })
    }

    pub fn choose_with(&self, amount: usize, punct: bool, numbers: bool) -> Vec<String> {
        let mut chosen = self.choose(amount);

        if numbers {
            let indices = rand::choose_multiple(0..chosen.len(), amount / 16);
            for i in indices {
                chosen[i] = rand::u8(..).to_string();
            }
        }

        if punct {
            punctuate(&mut chosen);
        }

        chosen
    }

    pub fn choose(&self, amount: usize) -> Vec<String> {
        rand::choose_multiple(self.words.iter().cloned(), amount)
    }
}

#[derive(Debug)]
pub enum WordSetError {
    Open(PathBuf, String),
}

impl fmt::Display for WordSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open(path, e) => write!(
                f,
                "Failed to open word set file '{}': {}",
                path.display(),
                e
            ),
        }
    }
}

fn punctuate(words: &mut [String]) {
    const TERMINAL: &[char] = &['.', '?', '!'];
    const PAUSE: &[char] = &[',', ';', ':'];
    const SEP: &[&str] = &["-", "/", "..."];
    const DELIM: &[(char, char)] = &[
        ('"', '"'),
        ('\'', '\''),
        ('(', ')'),
        ('[', ']'),
        ('{', '}'),
        ('<', '>'),
    ];

    let mut last_terminal = false;
    for word in words {
        if last_terminal {
            if let Some(f) = word.get_mut(..1) {
                f.make_ascii_uppercase();
            }
            last_terminal = false;
        }

        // A bit ad-hoc, could improve this.
        let r = rand::f64();
        if r < 0.3 {
            if r > 0.2 {
                word.push(TERMINAL[rand::usize(..TERMINAL.len())]);
                last_terminal = true;
            } else if r > 0.1 {
                word.push(PAUSE[rand::usize(..PAUSE.len())]);
            } else if r > 0.05 {
                let (l, r) = DELIM[rand::usize(..DELIM.len())];
                *word = format!("{}{}{}", l, word, r);
            } else {
                *word = SEP[rand::usize(..SEP.len())].to_string();
            }
        }
    }
}
