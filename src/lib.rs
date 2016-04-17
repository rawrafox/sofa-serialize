extern crate byteorder;
extern crate rustc_serialize;

pub mod decoder;
pub mod decoder_error;

pub mod encoder;
pub mod encoder_error;

pub use decoder::Decoder;
pub use encoder::Encoder;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub enum Size {
    Streaming, U64(u64)
}

#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub enum Event<'a> {
    Nil,
    Boolean(bool),
    U8(u8), U16(u16), U32(u32), U64(u64),
    I8(i8), I16(i16), I32(i32), I64(i64), Fixnum(&'a [u8]),
    F32(f32), F64(f64),
    Binary(&'a [u8]),
    String(&'a str),
    StartArray(Size),
    StartStruct(Size),
    StartMap(Size),
    StartOpenStruct(Size),
    End
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::Decoder;
    use super::Encoder;

    use super::{Event, Size};

    macro_rules! basic_test {
        ($identifier:ident, $input:expr) => {
            basic_test!($identifier, $input, vec![]);
        };
        ($identifier:ident, $input:expr, $dictionary:expr) => {
            #[test]
            fn $identifier() {
                let dictionary: Vec<&'static str> = $dictionary;

                let mut cursor = io::Cursor::new(Vec::new());

                {
                    let mut encoder = Encoder::new(&mut cursor, &dictionary[..]);

                    for event in $input {
                        encoder.write(&event).unwrap();
                    }
                }

                let stream = cursor.into_inner();

                let mut cursor = io::Cursor::new(&*stream);

                let decoder = Decoder::new(&mut cursor, &dictionary[..]);
                let events: Vec<Event> = decoder.collect();

                assert_eq!(events, $input);
            }
        };
    }

    basic_test!(transcodes_nil, vec![Event::Nil]);
    basic_test!(transcodes_false, vec![Event::Boolean(false)]);
    basic_test!(transcodes_true, vec![Event::Boolean(true)]);
    basic_test!(transcodes_binary, vec![Event::Binary(&vec![0x01, 0x02, 0x03, 0x04])]);
    basic_test!(transcodes_string, vec![Event::String("🍪")]);
    basic_test!(transcodes_dictionary_string, vec![Event::String("🍪")], vec!["🍪"]);
    basic_test!(transcodes_array, vec![Event::StartArray(Size::U64(1)), Event::Nil, Event::End]);
    basic_test!(transcodes_struct, vec![Event::StartStruct(Size::U64(1)), Event::String("🍪"), Event::Boolean(false), Event::End], vec!["🍪"]);
    basic_test!(transcodes_map, vec![Event::StartMap(Size::U64(1)), Event::String("🍪"), Event::Boolean(false), Event::End], vec!["🍪"]);
    basic_test!(transcodes_open_struct, vec![Event::StartOpenStruct(Size::U64(1)), Event::String("🍪"), Event::String("🍪"), Event::Boolean(false), Event::End], vec!["🍪"]);
    basic_test!(transcodes_u8, vec![Event::U8(0x50)]);
    basic_test!(transcodes_u16, vec![Event::U16(0x5150)]);
    basic_test!(transcodes_u32, vec![Event::U32(0x53525150)]);
    basic_test!(transcodes_u64, vec![Event::U64(0x5756555453525150)]);
    basic_test!(transcodes_i8, vec![Event::I8(0x50)]);
    basic_test!(transcodes_i16, vec![Event::I16(0x5150)]);
    basic_test!(transcodes_i32, vec![Event::I32(0x53525150)]);
    basic_test!(transcodes_i64, vec![Event::I64(0x5756555453525150)]);
    basic_test!(transcodes_f32, vec![Event::F32(1.0)]);
    basic_test!(transcodes_f64, vec![Event::F64(1.0)]);
}
