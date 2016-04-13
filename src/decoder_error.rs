use std::io;

use byteorder;

#[derive(Debug, PartialEq)]
pub enum ErrorCode {
    EndOfStream,
    InvalidDictionaryIndex,
    InvalidLength,
    InvalidType,
    InvalidUTF8,
    UnexpectedEOF
}

#[derive(Debug)]
pub enum DecoderError {
    StreamError(ErrorCode),
    IoError(io::Error)
}

impl From<byteorder::Error> for DecoderError {
    fn from(error: byteorder::Error) -> DecoderError {
        return DecoderError::IoError(From::from(error));
    }
}

impl From<io::Error> for DecoderError {
    fn from(error: io::Error) -> DecoderError {
        return DecoderError::IoError(error);
    }
}

impl PartialEq for DecoderError {
    fn eq(&self, other: &DecoderError) -> bool {
        return match (self, other) {
            (&DecoderError::StreamError(ref m0), &DecoderError::StreamError(ref m1)) => m0 == m1,
            _ => false
        };
    }
}

pub type DecoderResult<T> = Result<T, DecoderError>;
