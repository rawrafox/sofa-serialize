use std::io;

use byteorder;

#[derive(Debug, PartialEq)]
pub enum ErrorCode {
    EndOfStream,
    InvalidDictionaryIndex,
    InvalidEnd,
    MissingEnd
}

#[derive(Debug)]
pub enum EncoderError {
    StreamError(ErrorCode),
    IoError(io::Error)
}

impl From<byteorder::Error> for EncoderError {
    fn from(error: byteorder::Error) -> EncoderError {
        return EncoderError::IoError(From::from(error));
    }
}

impl From<io::Error> for EncoderError {
    fn from(error: io::Error) -> EncoderError {
        return EncoderError::IoError(error);
    }
}

impl PartialEq for EncoderError {
    fn eq(&self, other: &EncoderError) -> bool {
        return match (self, other) {
            (&EncoderError::StreamError(ref m0), &EncoderError::StreamError(ref m1)) => m0 == m1,
            _ => false
        };
    }
}

pub type EncoderResult<T> = Result<T, EncoderError>;
