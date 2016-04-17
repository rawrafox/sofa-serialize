use std::collections;
use std::io;

use byteorder::{LittleEndian, WriteBytesExt};

use super::{Event, Size};

use encoder_error::{ErrorCode, EncoderError, EncoderResult};

#[derive(Clone, Copy, Debug, PartialEq)]
enum StackSize { Streaming(u64, u64, u64), U64(u64) }

impl StackSize {
    fn from_size(size: Size, modulo: u64, required: u64) -> StackSize {
        return match size {
            Size::Streaming => StackSize::Streaming(0, modulo, required),
            Size::U64(size) => StackSize::U64(modulo * size + required as u64)
        };
    }
}

pub struct Encoder<'a> {
    writer: &'a mut io::Write,
    dictionary: collections::HashMap<&'a str, usize>,
    stack: Vec<StackSize>,
    invalid_state: bool
}

impl<'a> Encoder<'a> {
    pub fn new(writer: &'a mut io::Write, dictionary: &'a [&'a str]) -> Encoder<'a> {
        let mut map = collections::HashMap::new();

        for (i, e) in dictionary.iter().enumerate() {
            map.insert(*e, i);
        }

        return Encoder {
            writer: writer,
            dictionary: map,
            stack: vec![StackSize::U64(1)],
            invalid_state: false
        };
    }

    #[inline]
    fn write_length(&mut self, length: Size) -> EncoderResult<()> {
        match length {
            Size::U64(length) if length < 0xEF => {
                try!(self.writer.write_u8(length as u8))
            }
            _ => panic!("Not implemented yet")
        }

        return Ok(());
    }

    #[inline]
    fn write_string(&mut self, s: &str) -> EncoderResult<()> {
        if let Some(i) = self.dictionary.get(s) {
            if *i <= 0b01111111 {
                try!(self.writer.write_u8(*i as u8 | 0b10000000));
            } else {
                panic!("Not implemented yet");
            }

            return Ok(());
        }

        try!(self.writer.write_u8(0x09));
        try!(self.write_length(Size::U64(s.len() as u64)));
        try!(self.writer.write_all(s.as_bytes()));

        return Ok(());
    }

    #[inline]
    fn remove_one_from_stack(&mut self) -> EncoderResult<()> {
        let remaining = match self.stack.pop() {
            Some(StackSize::U64(0)) => {
                self.invalid_state = true;

                return Err(EncoderError::StreamError(ErrorCode::MissingEnd));
            }
            Some(StackSize::U64(s)) => StackSize::U64(s - 1),
            Some(StackSize::Streaming(n, modulo, required)) => StackSize::Streaming(n + 1, modulo, required),
            None => {
                self.invalid_state = true;

                return Err(EncoderError::StreamError(ErrorCode::EndOfStream));
            }
        };

        self.stack.push(remaining);

        return Ok(());
    }

    pub fn write(&mut self, event: &Event) -> EncoderResult<()> {
        if self.invalid_state {
            return Err(EncoderError::StreamError(ErrorCode::InvalidState));
        }

        if event == &Event::End {
            return match self.stack.pop() {
                Some(StackSize::U64(0)) => Ok(()),
                Some(StackSize::Streaming(n, m, r)) if n >= r && n % m == r => Ok(()),
                _ => {
                    self.invalid_state = true;
                    Err(EncoderError::StreamError(ErrorCode::InvalidEnd))
                }
            };
        }

        try!(self.remove_one_from_stack());

        match *event {
            Event::Nil => {
                try!(self.writer.write_u8(0x01))
            }
            Event::Boolean(false) => {
                try!(self.writer.write_u8(0x02))
            }
            Event::Boolean(true) => {
                try!(self.writer.write_u8(0x03))
            }
            Event::U8(v) => {
                try!(self.writer.write_u8(0x10));
                try!(self.writer.write_u8(v));
            }
            Event::U16(v) => {
                try!(self.writer.write_u8(0x11));
                try!(self.writer.write_u16::<LittleEndian>(v));
            }
            Event::U32(v) => {
                try!(self.writer.write_u8(0x12));
                try!(self.writer.write_u32::<LittleEndian>(v));
            }
            Event::U64(v) => {
                try!(self.writer.write_u8(0x13));
                try!(self.writer.write_u64::<LittleEndian>(v));
            }
            Event::I8(v) => {
                try!(self.writer.write_u8(0x14));
                try!(self.writer.write_i8(v));
            }
            Event::I16(v) => {
                try!(self.writer.write_u8(0x15));
                try!(self.writer.write_i16::<LittleEndian>(v));
            }
            Event::I32(v) => {
                try!(self.writer.write_u8(0x16));
                try!(self.writer.write_i32::<LittleEndian>(v));
            }
            Event::I64(v) => {
                try!(self.writer.write_u8(0x17));
                try!(self.writer.write_i64::<LittleEndian>(v));
            }
            Event::Fixnum(_) => {
                panic!("Not implemented yet");
            }
            Event::F32(v) => {
                try!(self.writer.write_u8(0x1A));
                try!(self.writer.write_f32::<LittleEndian>(v));
            }
            Event::F64(v) => {
                try!(self.writer.write_u8(0x1B));
                try!(self.writer.write_f64::<LittleEndian>(v));
            }
            Event::Binary(v) => {
                try!(self.writer.write_u8(0x08));
                try!(self.write_length(Size::U64(v.len() as u64)));
                try!(self.writer.write_all(v));
            }
            Event::String(v) => try!(self.write_string(v)),
            Event::StartArray(v) => {
                match v {
                    Size::U64(length) if length < 0b00001111 => {
                        try!(self.writer.write_u8(0b00100000 | length as u8));
                    }
                    length => {
                        try!(self.writer.write_u8(0x0A));
                        try!(self.write_length(length));
                    }
                }

                self.stack.push(StackSize::from_size(v, 1, 0));
            }
            Event::StartStruct(v) => {
                try!(self.writer.write_u8(0x0B));
                try!(self.write_length(v));

                self.stack.push(StackSize::from_size(v, 1, 1));
            }
            Event::StartMap(v) => {
                match v {
                    Size::U64(length) if length < 0b00001111 => {
                        try!(self.writer.write_u8(0b00110000 | length as u8));
                    }
                    length => {
                        try!(self.writer.write_u8(0x0C));
                        try!(self.write_length(length));
                    }
                }

                self.stack.push(StackSize::from_size(v, 2, 0));
            }
            Event::StartOpenStruct(v) => {
                try!(self.writer.write_u8(0x0D));
                try!(self.write_length(v));

                self.stack.push(StackSize::from_size(v, 2, 1));
            }
            Event::End => unreachable!()
        }

        return Ok(());
    }
}
