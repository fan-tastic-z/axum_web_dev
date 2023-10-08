pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    DataFailParse(String),
    FailToB64Decode,
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{self}")
    }
}

impl std::error::Error for Error {}
