use crate::test::TestRawResult;

pub fn process_raw(word_set: &str, raw: &TestRawResult) -> TestResult {
    let duration = raw.duration.as_secs() as u32;

    let wpm = 0.0;
    let acc = 0.0;
    let cons = 0.0;
    let errors = 0;

    let history = History {
        wpm: Vec::new(),
        err: Vec::new(),
    };

    TestResult {
        timestamp: raw.start,
        duration,
        word_set: word_set.into(),
        word_count: raw.word_count as u32,
        punct: raw.punct,
        numbers: raw.numbers,
        wpm,
        acc,
        cons,
        errors,
        quit: raw.quit,
        history,
    }
}

#[derive(Debug)]
pub struct TestResult {
    pub timestamp: u64,
    pub duration: u32,
    pub word_set: String,
    pub word_count: u32,
    pub punct: bool,
    pub numbers: bool,
    pub wpm: f32,
    pub acc: f32,
    pub cons: f32,
    pub errors: u32,
    pub quit: bool,
    pub history: History,
}

#[derive(Debug)]
pub struct History {
    pub wpm: Vec<u16>,
    pub err: Vec<u16>,
}

impl bincode::Encode for History {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        bincode::Encode::encode(&self.wpm, encoder)?;
        bincode::Encode::encode(&self.err, encoder)?;
        Ok(())
    }
}

impl bincode::Decode for History {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        Ok(Self {
            wpm: bincode::Decode::decode(decoder)?,
            err: bincode::Decode::decode(decoder)?,
        })
    }
}
