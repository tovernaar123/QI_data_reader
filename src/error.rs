use std::num::ParseFloatError;

use pyo3::PyErr;

use crate::header::NodeKind;

#[derive(thiserror::Error, Debug)]
pub enum QiError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Zip(#[from] rawzip::Error),

    #[error(transparent)]
    Numpy(#[from] PyErr),
    #[error("invalid header line {line} ({reason})")]
    InvalidHeaderLine {
        line: String,
        reason: &'static str,
    },
    #[error("Expected vrariant {exp} got ({got:?})")]
    InvalidNodeVariant {exp:String, got:NodeKind},

    #[error("Header is invalid utf8, {err}.")]
    InvalidUtf8Header{err:String},

    #[error("Expected header to have key {key} but it did not.")]
    HeaderMissingKey{key:String},
    #[error("Expected File {id}, but this file was not present in the archive.")]
    NoFile{id:String},

    #[error("No such such Channel:  {channel}")]
    InvalidChannel{channel:String},

    #[error(transparent)]
    ParseFloatError(#[from] ParseFloatError)
}

impl From<QiError> for PyErr {
    fn from(err: QiError) -> PyErr {
        use pyo3::exceptions::PyIOError;
        PyIOError::new_err(err.to_string())
    }
}