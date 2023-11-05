mod error;
mod scheme_01;

pub use self::error::{Error, Result};
use crate::pwd::ContentToHash;

pub const DEFAULT_SCHEME: &str = "01";

pub fn get_scheme(scheme_name: &str) -> Result<Box<dyn Scheme>> {
	match scheme_name {
		"01" => Ok(Box::new(scheme_01::Scheme01)),
		_ => Err(Error::SchemeNotFound(scheme_name.to_string())),
	}
}

pub trait Scheme {
	fn hash(&self, to_hash: &ContentToHash) -> Result<String>;

	fn validate(&self, to_hash: &ContentToHash, raw_pwd_ref: &str) -> Result<()>;
}

/// SchemeStatus is the return value of validate_pwd telling the caller if the
/// password scheme needs to updated.
#[derive(Debug)]
pub enum SchemeStatus {
	Ok,       // The pwd use the latest scheme. All good.
	Outdated, // The pwd use a old scheme. Would need to be re-hashed with default scheme.
}
