pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
	// -- Time
	DateFailParse(String),

	// -- Base64
	FailToB64uDecode,
}

impl core::fmt::Display for Error {
	fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(fmt, "{self}")
	}
}

impl std::error::Error for Error {}
