use std::{ops::Range, path::Path};

use rusqlite::{params, Connection};

use crate::result::TestResult;

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            r#"BEGIN;
               CREATE TABLE IF NOT EXISTS results (
                 id INTEGER PRIMARY KEY,
                 timestamp INTEGER NOT NULL,
                 duration INTEGER NOT NULL,
                 word_set TEXT NOT NULL,
                 word_count INTEGER NOT NULL,
                 punct INTEGER NOT NULL,
                 numbers INTEGER NOT NULL,
                 wpm REAL NOT NULL,
                 acc REAL NOT NULL,
                 cons REAL NOT NULL,
                 errors INTEGER NOT NULL,
                 quit INTEGER NOT NULL,
                 history BLOB NOT NULL
               );
               COMMIT;"#,
        )?;
        Ok(Self { conn })
    }

    pub fn save_result(&self, result: &TestResult) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO results (timestamp, duration,
                                  word_set, word_count,
                                  punct, numbers,
                                  wpm, acc, cons, errors,
                                  quit,
                                  history)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                result.timestamp,
                result.duration,
                result.word_set,
                result.word_count,
                result.punct,
                result.numbers,
                result.wpm,
                result.acc,
                result.cons,
                result.errors,
                result.quit,
                bincode::encode_to_vec(&result.history, bincode::config::standard()).unwrap(),
            ],
        )?;
        Ok(())
    }

    pub fn get_results_range(&self, range: Range<u64>) -> Result<Vec<TestResult>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT timestamp,
                    duration,
                    word_set,
                    word_count,
                    punct,
                    numbers,
                    wpm,
                    acc,
                    cons,
                    errors,
                    quit,
                    punct,
                    numbers,
                    history
             FROM results
             WHERE timestamp >= ?1 AND timestamp <= ?2",
        )?;
        let rows = stmt.query_map([range.start, range.end], |row| {
            Ok(TestResult {
                timestamp: row.get("timestamp")?,
                duration: row.get("duration")?,
                word_set: row.get("word_set")?,
                word_count: row.get("word_count")?,
                punct: row.get("punct")?,
                numbers: row.get("numbers")?,
                wpm: row.get("wpm")?,
                acc: row.get("acc")?,
                cons: row.get("cons")?,
                errors: row.get("errors")?,
                quit: row.get("quit")?,
                history: {
                    let history: Vec<u8> = row.get("history")?;
                    bincode::decode_from_slice(&history, bincode::config::standard())
                        .unwrap()
                        .0
                },
            })
        })?;
        let mut results = Vec::new();
        for result in rows {
            results.push(result?);
        }
        Ok(results)
    }
}
