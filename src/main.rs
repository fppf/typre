#[allow(dead_code)]
mod rand;

mod config;
mod db;
mod dump;
mod result;
mod test;
mod theme;
mod ui;
mod words;

use std::{path::PathBuf, process};

use config::Config;
use db::Db;
use theme::Theme;
use words::WordSet;

fn main() {
    let args = parse_args().unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        process::exit(1);
    });

    let config_path = args
        .config
        .or_else(|| config::default_config_dir().map(|dir| dir.join("config.toml")));
    let config = match config_path {
        Some(path) if path.exists() => Config::load(&path).unwrap_or_else(|e| {
            eprintln!("Could not load config '{}'...", path.display());
            eprintln!("  {}", e);
            process::exit(1);
        }),
        _ => {
            eprintln!("Configuration required, either in $XDG_CONFIG_HOME/typre/config.toml or via --config PATH.");
            process::exit(1);
        }
    };

    let db = Db::new(&config.db_path).unwrap_or_else(|e| {
        eprintln!("Could not load database '{}'...", config.db_path.display());
        eprintln!("  {}", e);
        process::exit(1);
    });

    if let Some(path) = args.csv {
        dump::csv(&db, path).unwrap();
        process::exit(0);
    }

    if args.list_sets {
        println!("Available word sets");
        println!(
            "  {}",
            config.sets.keys().cloned().collect::<Vec<_>>().join(", ")
        );
    }

    if args.list_themes {
        if args.list_sets {
            println!();
        }
        println!("Available themes");
        println!("  {}", config.themes.names().collect::<Vec<_>>().join(", "));
    }

    if args.list_sets || args.list_themes {
        process::exit(0);
    }

    if args.set.is_none() {
        eprintln!("Must provide word set with --set SETNAME.");
        process::exit(1);
    }

    if args.word_count == 0 {
        eprintln!("Word count must be > 0.");
        process::exit(1);
    }

    let set_name = args.set.unwrap();
    let set_path = config.sets.get(&set_name).unwrap_or_else(|| {
        eprintln!("Word set '{}' is not available.", set_name);
        process::exit(1);
    });
    let set = WordSet::load(&set_path).unwrap_or_else(|e| {
        eprintln!(
            "Could not load word set '{}' from path '{}'...",
            set_name,
            set_path.display()
        );
        eprintln!("  {}", e);
        process::exit(1);
    });

    let mut theme = match args.theme {
        Some(name) => config.themes.get(&name).unwrap_or_else(|| {
            eprintln!("Warning: could not load theme '{}'...", name);
            Theme::default()
        }),
        None => config.theme,
    };

    if !args.bg && (args.no_bg || !config.show_bg) {
        theme.bg.take();
    }

    match test::run_test(&set, args.word_count, args.punct, args.numbers, theme)
        .expect("UI crashed")
    {
        Some(raw) => {
            let result = result::process_raw(&set_name, &raw);
            println!("{:#?}", result);
            db.save_result(&result).unwrap_or_else(|e| {
                eprintln!("Could not save result to database...");
                eprintln!("  {}", e);
            });
        }
        None => println!("No test started."),
    }
}

const HELP: &str = "\
typre

USAGE:
  typre [OPTIONS] --set WORDSET

OPTIONS:
  --set WORDSET      Select the word set to use.
  --count NUMBER     Set the number of words [default: 50].
  --punct            Enable randomly added punctuation.
  --numbers          Enable randomly added numbers.
  --config PATH      Set the configuration path.
  
  --theme THEME      Set the theme or override configuration [default: red & green].
  --bg, --no-bg      Enable/disable background color.
  
  --csv PATH         Dump database to CSV.
  --list-sets        List the available word sets.
  --list-themes      List the available themes.
  -h, --help         Display this message.
";

struct Args {
    word_count: usize,
    set: Option<String>,
    config: Option<PathBuf>,
    punct: bool,
    numbers: bool,
    theme: Option<String>,
    bg: bool,
    no_bg: bool,
    csv: Option<PathBuf>,
    list_sets: bool,
    list_themes: bool,
}

fn parse_args() -> Result<Args, pico_args::Error> {
    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        print!("{}", HELP);
        process::exit(0);
    }

    let args = Args {
        set: pargs.opt_value_from_str("--set")?,
        word_count: pargs.opt_value_from_str("--count")?.unwrap_or(50),
        punct: pargs.contains("--punct"),
        numbers: pargs.contains("--numbers"),
        config: pargs.opt_value_from_str("--config")?,
        theme: pargs.opt_value_from_str("--theme")?,
        bg: pargs.contains("--bg"),
        no_bg: pargs.contains("--no-bg"),
        csv: pargs.opt_value_from_str("--csv")?,
        list_sets: pargs.contains("--list-sets"),
        list_themes: pargs.contains("--list-themes"),
    };

    if args.bg && args.no_bg {
        eprintln!("Error: --bg and --no-bg are mutually exclusive.");
        process::exit(1);
    }

    let remaining = pargs.finish();
    if !remaining.is_empty() {
        eprintln!(
            "Unrecognized argument{}: {}",
            if remaining.len() > 1 { "s" } else { "" },
            remaining
                .iter()
                .map(|arg| arg.to_string_lossy())
                .collect::<Vec<_>>()
                .join(", ")
        );
        process::exit(1);
    }

    Ok(args)
}
