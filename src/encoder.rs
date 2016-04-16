use std::collections;
use std::io;

use byteorder::{LittleEndian, WriteBytesExt};

use super::Event;

use encoder_error::{ErrorCode, EncoderError, EncoderResult};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Size { U64(u64), Streaming(usize, usize, usize) }

pub struct Encoder<'a> {
    writer: &'a mut io::Write,
    dictionary: collections::HashMap<&'a str, usize>,
    stack: Vec<Size>
}

impl<'a> Encoder<'a> {
    pub fn new(writer: &'a mut io::Write, dictionary: &'a [&'a str]) -> Encoder<'a> {
        let mut map = collections::HashMap::new();

        for (i, e) in dictionary.iter().enumerate() {
            map.insert(*e, i);
        }

        return Encoder { writer: writer, dictionary: map, stack: vec![Size::U64(1)] };
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
    fn push_stack(&mut self, remaining: Size) {
        match remaining {
            Size::U64(s) => self.stack.push(Size::U64(s - 1)),
            Size::Streaming(n, modulo, required) => self.stack.push(Size::Streaming(n + 1, modulo, required))
        }
    }

    #[inline]
    fn push_stack_structure(&mut self, remaining: Size, structure_size: Size) {
        self.push_stack(remaining);
        self.stack.push(structure_size);
    }


    pub fn write(&mut self, event: &Event) -> EncoderResult<()> {
        match self.stack.pop() {
            Some(Size::U64(0)) => {
                if *event != Event::End {
                    return Err(EncoderError::StreamError(ErrorCode::MissingEnd));
                } else {
                    if self.stack.len() != 0 {
                        return Ok(());
                    } else {
                        return Err(EncoderError::StreamError(ErrorCode::EndOfStream));
                    }
                }
            }
            Some(remaining) => {
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
                            Some(length) if length < 0b00001111 => {
                                try!(self.writer.write_u8(0b00100000 | length as u8));

                                self.push_stack_structure(remaining, Size::U64(length as u64));

                                return Ok(());
                            }
                            x => {
                                let structure_size = match x {
                                    None => Size::Streaming(0, 1, 0),
                                    Some(size) => Size::U64(size as u64)
                                };

                                self.push_stack_structure(remaining, structure_size);

                                try!(self.writer.write_u8(0x0A));

                                return self.write_length(structure_size);
                            }
                        }
                    }
                    Event::StartStruct(v) => {
                        let stack_size = match v {
                            None => Size::Streaming(0, 1, 1),
                            Some(size) => Size::U64(1 + size as u64)
                        };

                        let structure_size = match v {
                            None => Size::Streaming(0, 1, 1),
                            Some(size) => Size::U64(size as u64)
                        };

                        self.push_stack_structure(remaining, stack_size);

                        try!(self.writer.write_u8(0x0B));

                        return self.write_length(structure_size);
                    }
                    Event::StartMap(v) => {
                        match v {
                            Some(length) if length < 0b00001111 => {
                                try!(self.writer.write_u8(0b00110000 | length as u8));

                                self.push_stack_structure(remaining, Size::U64(2 * length as u64));

                                return Ok(());
                            }
                            x => {
                                let structure_size = match x {
                                    None => Size::Streaming(0, 2, 0),
                                    Some(size) => Size::U64(2 * size as u64)
                                };

                                self.push_stack_structure(remaining, structure_size);

                                try!(self.writer.write_u8(0x0C));

                                return self.write_length(structure_size);
                            }
                        }
                    }
                    Event::StartOpenStruct(v) => {
                        let stack_size = match v {
                            None => Size::Streaming(0, 2, 1),
                            Some(size) => Size::U64(1 + 2 * size as u64)
                        };

                        let structure_size = match v {
                            None => Size::Streaming(0, 2, 1),
                            Some(size) => Size::U64(size as u64)
                        };

                        self.push_stack_structure(remaining, stack_size);

                        try!(self.writer.write_u8(0x0D));

                        return self.write_length(structure_size);
                    }
                    Event::End => {
                        return match remaining {
                            Size::Streaming(n, modulo, required) if n >= required && n % modulo == required => Ok(()),
                            Size::U64(0) => Ok(()),
                            _ => Err(EncoderError::StreamError(ErrorCode::InvalidEnd))
                        };
                    }
                }

                self.push_stack(remaining);
            }
            None => {
                return Err(EncoderError::StreamError(ErrorCode::EndOfStream));
            }
        }

        return Ok(());
    }
}

// Binary(&'a [u8]),
// String(&'a str),
// StartArray(Option<usize>),
// StartStruct(Option<usize>),
// StartMap(Option<usize>),
// StartOpenStruct(Option<usize>),

// 0x08 => Event::Binary(try!(self.read_binary())),
// 0x09 => Event::String(try!(self.read_string())),
// 0x0A => {
//     let length = try!(self.read_length());
//
//     self.push_stack(remaining - 1);
//     self.push_stack(length);
//
//     return Ok(Some(Event::StartArray(Some(length))));
// }
// 0x0B => {
//     let length = try!(self.read_length());
//
//     self.push_stack(remaining - 1);
//     self.push_stack(length + 1);
//
//     return Ok(Some(Event::StartStruct(Some(length))));
// }
// 0x0C => {
//     let length = try!(self.read_length());
//
//     self.push_stack(remaining - 1);
//     self.push_stack(2 * length);
//
//     return Ok(Some(Event::StartMap(Some(length))));
// }
// 0x0D => {
//     let length = try!(self.read_length());
//
//     self.push_stack(remaining - 1);
//     self.push_stack(2 * length + 1);
//
//     return Ok(Some(Event::StartOpenStruct(Some(length))));
// }
// x if x & 0b10000000 == 0b10000000 => Event::String(try!(self.read_dictionary(x as usize & 0b01111111))),
// x if x & 0b11100000 == 0b01100000 => Event::String(try!(self.read_string_data(x as usize & 0b00011111))),
// x if x & 0b11110000 == 0b00100000 => {
//     let length = x as usize & 0b00001111;
//
//     self.push_stack(remaining - 1);
//     self.push_stack(length);
//
//     return Ok(Some(Event::StartArray(Some(length))));
// },
// x if x & 0b11110000 == 0b00110000 => {
//     let length = x as usize & 0b00001111;
//
//     self.push_stack(remaining - 1);
//     self.push_stack(2 * length);
//
//     return Ok(Some(Event::StartMap(Some(length))));
// }
// _ => {
//     return Err(DecoderError::StreamError(ErrorCode::InvalidType));
// }
