use std::{collections::HashMap, fmt, str::FromStr};

use termion::color;

#[derive(Default, Debug)]
pub struct Themes {
    themes: HashMap<String, Theme>,
}

impl Themes {
    pub fn from_value(value: &toml::Value) -> Result<Self, ThemeError> {
        let theme_table = value
            .get("themes")
            .ok_or(ThemeError::MissingThemeTable)?
            .as_table()
            .ok_or(ThemeError::InvalidThemeTable)?;

        let mut themes = HashMap::new();
        for (name, theme) in theme_table.iter() {
            let theme = Theme::from_value(theme).map_err(|e| ThemeError::Load(name.clone(), e))?;
            themes.insert(name.clone(), theme);
        }

        Ok(Self { themes })
    }

    pub fn get(&self, name: &str) -> Option<Theme> {
        self.themes.get(name).copied()
    }

    pub fn names(&self) -> impl Iterator<Item = String> + '_ {
        self.themes.keys().cloned()
    }
}

#[derive(Debug)]
pub enum ThemeError {
    Load(String, ThemeLoadError),
    MissingThemeTable,
    InvalidThemeTable,
}

impl fmt::Display for ThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Load(name, e) => write!(f, "Error loading theme '{}': {:?}", name, e),
            Self::MissingThemeTable => write!(f, "No theme table found"),
            Self::InvalidThemeTable => write!(
                f,
                "Invalid theme table: must be formatted as [themes.<theme-name>] (...)"
            ),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub bg: Option<Color>,
    pub correct: Color,
    pub error: Color,
    pub extra: Color,
    pub empty: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Some(Color::Black),
            correct: Color::Green,
            error: Color::Red,
            extra: Color::Red,
            empty: Color::White,
        }
    }
}

impl Theme {
    fn from_value(value: &toml::Value) -> Result<Self, ThemeLoadError> {
        fn load_color(
            name: &'static str,
            table: &toml::value::Table,
        ) -> Result<Color, ThemeLoadError> {
            let color = table.get(name).ok_or(ThemeLoadError::Missing(name))?;
            match color.as_str() {
                Some(color) => Ok(color.parse().map_err(ThemeLoadError::ParseColor)?),
                None => Err(ThemeLoadError::NotStr(name)),
            }
        }

        fn load_color_opt(
            name: &'static str,
            table: &toml::value::Table,
        ) -> Result<Option<Color>, ThemeLoadError> {
            match load_color(name, table) {
                Ok(color) => Ok(Some(color)),
                Err(ThemeLoadError::Missing(_)) => Ok(None),
                Err(e) => Err(e),
            }
        }

        let table = value.as_table().unwrap();

        let bg = load_color_opt("bg", table)?;
        let correct = load_color("correct", table)?;
        let error = load_color("error", table)?;
        let extra = load_color("extra", table)?;
        let empty = load_color("empty", table)?;

        Ok(Self {
            bg,
            correct,
            error,
            extra,
            empty,
        })
    }
}

#[derive(Debug)]
pub enum ThemeLoadError {
    Missing(&'static str),
    NotStr(&'static str),
    ParseColor(ParseColorError),
}

impl fmt::Display for ThemeLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "Missing field '{}'", field),
            Self::NotStr(field) => write!(f, "Color for field '{}' must be a quoted string", field),
            Self::ParseColor(e) => write!(f, "Invalid color: {}", e),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Ansi(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug)]
pub enum ParseColorError {
    InvalidAnsi(String),
    InvalidRgb(String),
}

impl fmt::Display for ParseColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAnsi(s) => write!(f, "Could not parse '{}' as an ANSI color", s),
            Self::InvalidRgb(s) => write!(f, "Could not parse '{}' as a 24-bit RGB color", s),
        }
    }
}

impl FromStr for Color {
    type Err = ParseColorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn parse_color(s: &str) -> Result<Color, ParseColorError> {
            Ok(if let Some(rgb) = s.strip_prefix('#') {
                if rgb.len() != 6 {
                    return Err(ParseColorError::InvalidRgb(s.into()));
                }
                let (r, rgb) = rgb.split_at(2);
                let (g, rgb) = rgb.split_at(2);
                let (b, rgb) = rgb.split_at(2);
                assert!(rgb.is_empty());

                let r =
                    u8::from_str_radix(r, 16).map_err(|_| ParseColorError::InvalidRgb(s.into()))?;
                let g =
                    u8::from_str_radix(g, 16).map_err(|_| ParseColorError::InvalidRgb(s.into()))?;
                let b =
                    u8::from_str_radix(b, 16).map_err(|_| ParseColorError::InvalidRgb(s.into()))?;

                Color::Rgb(r, g, b)
            } else {
                let c: u8 = s
                    .parse()
                    .map_err(|_| ParseColorError::InvalidAnsi(s.into()))?;
                Color::Ansi(c)
            })
        }

        Ok(match &*s.to_lowercase() {
            "black" => Self::Black,
            "red" => Self::Red,
            "green" => Self::Green,
            "yellow" => Self::Yellow,
            "blue" => Self::Blue,
            "magenta" => Self::Magenta,
            "cyan" => Self::Cyan,
            "white" => Self::White,
            c => parse_color(c)?,
        })
    }
}

impl color::Color for Color {
    fn write_fg(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Black => color::Black.write_fg(f),
            Self::Red => color::Red.write_fg(f),
            Self::Green => color::Green.write_fg(f),
            Self::Yellow => color::Yellow.write_fg(f),
            Self::Blue => color::Blue.write_fg(f),
            Self::Magenta => color::Magenta.write_fg(f),
            Self::Cyan => color::Cyan.write_fg(f),
            Self::White => color::White.write_fg(f),
            Self::Ansi(c) => color::AnsiValue(c).write_fg(f),
            Self::Rgb(r, g, b) => color::Rgb(r, g, b).write_fg(f),
        }
    }

    fn write_bg(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Black => color::Black.write_bg(f),
            Self::Red => color::Red.write_bg(f),
            Self::Green => color::Green.write_bg(f),
            Self::Yellow => color::Yellow.write_bg(f),
            Self::Blue => color::Blue.write_bg(f),
            Self::Magenta => color::Magenta.write_bg(f),
            Self::Cyan => color::Cyan.write_bg(f),
            Self::White => color::White.write_bg(f),
            Self::Ansi(c) => color::AnsiValue(c).write_bg(f),
            Self::Rgb(r, g, b) => color::Rgb(r, g, b).write_bg(f),
        }
    }
}
