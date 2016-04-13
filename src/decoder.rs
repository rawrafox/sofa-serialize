use std::cmp;
use std::io;
use std::str;

use byteorder::{self, LittleEndian, ReadBytesExt};

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

#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub enum Event<'a> {
    Nil,
    Boolean(bool),
    U8(u8), U16(u16), U32(u32), U64(u64),
    I8(i8), I16(i16), I32(i32), I64(i64), Fixnum(&'a [u8]),
    F32(f32), F64(f64),
    Binary(&'a [u8]),
    String(&'a str),
    StartArray(Option<usize>),
    StartStruct(Option<usize>),
    StartMap(Option<usize>),
    StartOpenStruct(Option<usize>),
    End
}

pub trait BorrowRead<'a> : io::Read {
    fn fill_buffer(&self) -> &'a [u8];
    fn consume(&mut self, len: usize);
}

impl<'a> BorrowRead<'a> for &'a [u8] {
    fn fill_buffer(&self) -> &'a [u8] {
        return self;
    }

    fn consume(&mut self, len: usize) {
        *self = &(*self)[len..];
    }
}

impl<'a> BorrowRead<'a> for io::Cursor<&'a [u8]> {
    fn fill_buffer(&self) -> &'a [u8] {
        let len = cmp::min(self.position(), self.get_ref().len() as u64);

        return &self.get_ref()[len as usize..];
    }

    fn consume(&mut self, len: usize) {
        let pos = self.position();

        self.set_position(pos + len as u64);
    }
}

pub struct Decoder<'a> {
    reader: &'a mut BorrowRead<'a>,
    dictionary: &'a [&'a str],
    stack: Vec<usize>
}

impl<'a> Decoder<'a> {
    pub fn new(reader: &'a mut BorrowRead<'a>, dictionary: &'a [&'a str]) -> Decoder<'a> {
        return Decoder { reader: reader, dictionary: dictionary, stack: vec![1] };
    }

    #[inline]
    fn read_length(&mut self) -> DecoderResult<usize> {
        let result = match try!(self.reader.read_u8()) {
            x if x < 0xEF => x as usize,
            0xF1 => try!(self.reader.read_u8()) as usize,
            0xF2 => try!(self.reader.read_u16::<LittleEndian>()) as usize,
            0xF3 => try!(self.reader.read_u32::<LittleEndian>()) as usize,
            0xF4 => try!(self.reader.read_u64::<LittleEndian>()) as usize,
            _ => {
                return Err(DecoderError::StreamError(ErrorCode::InvalidLength));
            }
        };

        return Ok(result);
    }

    #[inline]
    fn read_binary(&mut self) -> DecoderResult<&'a [u8]> {
        let length = try!(self.read_length());

        let buffer = self.reader.fill_buffer();

        if length > buffer.len() {
            return Err(DecoderError::StreamError(ErrorCode::UnexpectedEOF));
        }

        let buffer = &buffer[..length];

        self.reader.consume(length);

        return Ok(buffer);
    }

    #[inline]
    fn read_string(&mut self) -> DecoderResult<&'a str> {
        let length = match try!(self.reader.read_u8()) {
            x if x < 0xEF => x as usize,
            0xF1 => try!(self.reader.read_u8()) as usize,
            0xF2 => try!(self.reader.read_u16::<LittleEndian>()) as usize,
            0xF3 => try!(self.reader.read_u32::<LittleEndian>()) as usize,
            0xF4 => try!(self.reader.read_u64::<LittleEndian>()) as usize,
            0xF5 => {
                let index = try!(self.reader.read_u8());

                return self.read_dictionary(index as usize);
            }
            0xF6 => {
                let index = try!(self.reader.read_u16::<LittleEndian>());

                return self.read_dictionary(index as usize);
            }
            0xF7 => {
                let index = try!(self.reader.read_u32::<LittleEndian>());

                return self.read_dictionary(index as usize);
            }
            0xF8 => {
                let index = try!(self.reader.read_u64::<LittleEndian>());

                return self.read_dictionary(index as usize);
            }
            _ => {
                return Err(DecoderError::StreamError(ErrorCode::InvalidLength));
            }
        };

        return self.read_string_data(length);
    }

    #[inline]
    fn read_string_data(&mut self, length: usize) -> DecoderResult<&'a str> {
        let buffer = self.reader.fill_buffer();

        if length > buffer.len() {
            return Err(DecoderError::StreamError(ErrorCode::UnexpectedEOF));
        }

        let buffer = &buffer[..length];

        self.reader.consume(length);

        return match str::from_utf8(buffer) {
            Ok(s) => Ok(s),
            Err(_) => Err(DecoderError::StreamError(ErrorCode::InvalidUTF8))
        };
    }

    #[inline]
    fn read_dictionary(&mut self, index: usize) -> DecoderResult<&'a str> {
        return match self.dictionary.get(index) {
            Some(s) => Ok(s),
            None => Err(DecoderError::StreamError(ErrorCode::InvalidDictionaryIndex))
        };
    }

    #[inline]
    fn push_stack(&mut self, remaining: usize) {
        self.stack.push(remaining);
    }

    pub fn read(&mut self) -> DecoderResult<Option<Event<'a>>> {
        match self.stack.pop() {
            Some(remaining) => {
                if remaining == 0 {
                    if self.stack.len() == 0 {
                        return Ok(None)
                    } else {
                        return Ok(Some(Event::End))
                    }
                }

                let result = match try!(self.reader.read_u8()) {
                    0x01 => Event::Nil,
                    0x02 => Event::Boolean(false),
                    0x03 => Event::Boolean(true),
                    0x08 => Event::Binary(try!(self.read_binary())),
                    0x09 => Event::String(try!(self.read_string())),
                    0x0A => {
                        let length = try!(self.read_length());

                        self.push_stack(remaining - 1);
                        self.push_stack(length);

                        return Ok(Some(Event::StartArray(Some(length))));
                    }
                    0x0B => {
                        let length = try!(self.read_length());

                        self.push_stack(remaining - 1);
                        self.push_stack(length + 1);

                        return Ok(Some(Event::StartStruct(Some(length))));
                    }
                    0x0C => {
                        let length = try!(self.read_length());

                        self.push_stack(remaining - 1);
                        self.push_stack(2 * length);

                        return Ok(Some(Event::StartMap(Some(length))));
                    }
                    0x0D => {
                        let length = try!(self.read_length());

                        self.push_stack(remaining - 1);
                        self.push_stack(2 * length + 1);

                        return Ok(Some(Event::StartOpenStruct(Some(length))));
                    }
                    0x10 => Event::U8(try!(self.reader.read_u8())),
                    0x11 => Event::U16(try!(self.reader.read_u16::<LittleEndian>())),
                    0x12 => Event::U32(try!(self.reader.read_u32::<LittleEndian>())),
                    0x13 => Event::U64(try!(self.reader.read_u64::<LittleEndian>())),
                    0x14 => Event::I8(try!(self.reader.read_i8())),
                    0x15 => Event::I16(try!(self.reader.read_i16::<LittleEndian>())),
                    0x16 => Event::I32(try!(self.reader.read_i32::<LittleEndian>())),
                    0x17 => Event::I64(try!(self.reader.read_i64::<LittleEndian>())),
                    0x18 => panic!("Not implemented yet"), // Fixnum
                    0x1A => Event::F32(try!(self.reader.read_f32::<LittleEndian>())),
                    0x1B => Event::F64(try!(self.reader.read_f64::<LittleEndian>())),
                    x if x & 0b10000000 == 0b10000000 => Event::String(try!(self.read_dictionary(x as usize & 0b01111111))),
                    x if x & 0b11100000 == 0b01100000 => Event::String(try!(self.read_string_data(x as usize & 0b00011111))),
                    x if x & 0b11110000 == 0b00100000 => {
                        let length = x as usize & 0b00001111;

                        self.push_stack(remaining - 1);
                        self.push_stack(length);

                        return Ok(Some(Event::StartArray(Some(length))));
                    },
                    x if x & 0b11110000 == 0b00110000 => {
                        let length = x as usize & 0b00001111;

                        self.push_stack(remaining - 1);
                        self.push_stack(2 * length);

                        return Ok(Some(Event::StartMap(Some(length))));
                    }
                    _ => {
                        return Err(DecoderError::StreamError(ErrorCode::InvalidType));
                    }
                };

                self.push_stack(remaining - 1);

                return Ok(Some(result));
            }
            None => {
                return Err(DecoderError::StreamError(ErrorCode::EndOfStream));
            }
        }
    }
}

impl<'a> Iterator for Decoder<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Event<'a>> {
        return self.read().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::Decoder;
    use super::Event;

    macro_rules! basic_test {
        ($identifier:ident, $input:expr, $output:expr) => {
            basic_test!($identifier, $input, $output, vec![]);
        };
        ($identifier:ident, $input:expr, $output:expr, $dictionary:expr) => {
            #[test]
            fn $identifier() {
                let data = $input;
                let dictionary: Vec<&'static str> = $dictionary;
                let mut cursor = io::Cursor::new(&*data);

                let decoder = Decoder::new(&mut cursor, &dictionary[..]);
                let events: Vec<Event> = decoder.collect();

                assert_eq!(events, $output);
            }
        };
    }

    basic_test!(decodes_nil, vec![0x01], vec![Event::Nil]);
    basic_test!(decodes_false, vec![0x02], vec![Event::Boolean(false)]);
    basic_test!(decodes_true, vec![0x03], vec![Event::Boolean(true)]);
    basic_test!(decodes_binary, vec![0x08, 0x04, 0x01, 0x02, 0x03, 0x04], vec![Event::Binary(&vec![0x01, 0x02, 0x03, 0x04])]);
    basic_test!(decodes_string, vec![0x64, 0xF0, 0x9F, 0x8D, 0xAA], vec![Event::String("🍪")]);
    basic_test!(decodes_dictionary_string, vec![0x80], vec![Event::String("🍪")], vec!["🍪"]);
    basic_test!(decodes_noncanonical_string, vec![0x09, 0x04, 0xF0, 0x9F, 0x8D, 0xAA], vec![Event::String("🍪")]);
    basic_test!(decodes_array, vec![0x21, 0x01], vec![Event::StartArray(Some(1)), Event::Nil, Event::End]);
    basic_test!(decodes_noncanonical_array, vec![0x0A, 0x02, 0x01, 0x01], vec![Event::StartArray(Some(2)), Event::Nil, Event::Nil, Event::End]);
    basic_test!(decodes_struct, vec![0x0B, 0x01, 0x80, 0x02], vec![Event::StartStruct(Some(1)), Event::String("🍪"), Event::Boolean(false), Event::End], vec!["🍪"]);
    basic_test!(decodes_map, vec![0x0C, 0x01, 0x80, 0x02], vec![Event::StartMap(Some(1)), Event::String("🍪"), Event::Boolean(false), Event::End], vec!["🍪"]);
    basic_test!(decodes_noncanonical_map, vec![0x0C, 0x01, 0x80, 0x02], vec![Event::StartMap(Some(1)), Event::String("🍪"), Event::Boolean(false), Event::End], vec!["🍪"]);
    basic_test!(decodes_open_struct, vec![0x0D, 0x01, 0x80, 0x80, 0x02], vec![Event::StartOpenStruct(Some(1)), Event::String("🍪"), Event::String("🍪"), Event::Boolean(false), Event::End], vec!["🍪"]);
    basic_test!(decodes_u8, vec![0x10, 0x50], vec![Event::U8(0x50)]);
    basic_test!(decodes_u16, vec![0x11, 0x50, 0x51], vec![Event::U16(0x5150)]);
    basic_test!(decodes_u32, vec![0x12, 0x50, 0x51, 0x52, 0x53], vec![Event::U32(0x53525150)]);
    basic_test!(decodes_u64, vec![0x13, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57], vec![Event::U64(0x5756555453525150)]);
    basic_test!(decodes_i8, vec![0x14, 0x50], vec![Event::I8(0x50)]);
    basic_test!(decodes_i16, vec![0x15, 0x50, 0x51], vec![Event::I16(0x5150)]);
    basic_test!(decodes_i32, vec![0x16, 0x50, 0x51, 0x52, 0x53], vec![Event::I32(0x53525150)]);
    basic_test!(decodes_i64, vec![0x17, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57], vec![Event::I64(0x5756555453525150)]);
    basic_test!(decodes_f32, vec![0x1A, 0x00, 0x00, 0x80, 0x3F], vec![Event::F32(1.0)]);
    basic_test!(decodes_f64, vec![0x1B, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F], vec![Event::F64(1.0)]);

}
