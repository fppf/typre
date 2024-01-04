use std::{
    fs::File,
    io::{self, Write},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::db::Db;

pub fn csv<P: AsRef<Path>>(db: &Db, to: P) -> io::Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let results = db.get_results_range(0..now).unwrap();

    let header = [
        "timestamp",
        "duration",
        "word_set",
        "word_count",
        "punct",
        "numbers",
        "wpm",
        "acc",
        "cons",
        "errors",
        "quit",
    ]
    .join(",");

    let mut file = File::create(to)?;
    writeln!(file, "{}", header)?;

    for result in results {
        let row = [
            result.timestamp.to_string(),
            result.duration.to_string(),
            result.word_set,
            result.word_count.to_string(),
            result.punct.to_string(),
            result.numbers.to_string(),
            result.wpm.to_string(),
            result.acc.to_string(),
            result.cons.to_string(),
            result.errors.to_string(),
            result.quit.to_string(),
        ]
        .join(",");
        writeln!(file, "{}", row)?;
    }

    Ok(())
}
