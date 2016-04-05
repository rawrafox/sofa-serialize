use std::collections::BTreeMap;
use std::io;
use std::io::Write;
use std::{i8, i16, i32, i64};

use byteorder::{LittleEndian, WriteBytesExt};

pub struct Encoder<'a> {
    writer: &'a mut (io::Write + 'a),
    dictionary: BTreeMap<String, u64>
}

impl<'a> Encoder<'a> {
    pub fn new(writer: &'a mut io::Write, dictionary: Vec<String>) -> Encoder<'a> {
        let mut map = BTreeMap::new();

        for (i, e) in dictionary.iter().enumerate() {
            map.insert(e.clone(), i as u64);
        }

        return Encoder { writer: writer, dictionary: map };
    }

    fn emit_length(&mut self, length: u64) -> io::Result<()> {
        match length {
            x if x < 0xF0 => {
                try!(self.writer.write_u8(x as u8));
            }
            x if x <= 0xFF => {
                try!(self.writer.write_u8(0xF0));
                try!(self.writer.write_u8(x as u8));
            }
            x if x <= 0xFFFF => {
                try!(self.writer.write_u8(0xF1));
                try!(self.writer.write_u16::<LittleEndian>(x as u16));
            }
            x if x <= 0xFFFFFFFF => {
                try!(self.writer.write_u8(0xF2));
                try!(self.writer.write_u32::<LittleEndian>(x as u32));
            }
            x => {
                try!(self.writer.write_u8(0xF3));
                try!(self.writer.write_u64::<LittleEndian>(x as u64));
            }
        }

        return Ok(());
    }

    fn emit_dictionary_string(&mut self, n: u64) -> io::Result<()> {
        match n {
            x if x <= 0xFF => {
                try!(self.writer.write_u8(0xF4));
                try!(self.writer.write_u8(x as u8));
            }
            x if x <= 0xFFFF => {
                try!(self.writer.write_u8(0xF5));
                try!(self.writer.write_u16::<LittleEndian>(x as u16));
            }
            x if x <= 0xFFFFFFFF => {
                try!(self.writer.write_u8(0xF6));
                try!(self.writer.write_u32::<LittleEndian>(x as u32));
            }
            x => {
                try!(self.writer.write_u8(0xF7));
                try!(self.writer.write_u64::<LittleEndian>(x as u64));
            }
        }

        return Ok(());
    }

    pub fn emit_string_fragment(&mut self, string: &str) -> io::Result<()> {
        let slice = string.as_bytes();

        let dictionary_index = self.dictionary.get(string).map(|x| *x);

        match dictionary_index {
            Some(i) => try!(self.emit_dictionary_string(i)),
            None => {
                try!(self.emit_length(slice.len() as u64));
                try!(self.writer.write_all(slice));
            }
        }

        return Ok(());
    }

    pub fn emit_nil(&mut self) -> io::Result<()> {
        try!(self.writer.write_u8(0x01));

        return Ok(());
    }

    pub fn emit_bool(&mut self, v: bool) -> io::Result<()> {
        try!(self.writer.write_u8(if v { 0x03 } else { 0x02} ));

        return Ok(());
    }

    pub fn emit_u8(&mut self, v: u8) -> io::Result<()> {
        try!(self.writer.write_u8(0x10));
        try!(self.writer.write_u8(v));

        return Ok(());
    }

    pub fn emit_u16(&mut self, v: u16) -> io::Result<()> {
        try!(self.writer.write_u8(0x11));
        try!(self.writer.write_u16::<LittleEndian>(v));

        return Ok(());
    }

    pub fn emit_u32(&mut self, v: u32) -> io::Result<()> {
        try!(self.writer.write_u8(0x12));
        try!(self.writer.write_u32::<LittleEndian>(v));

        return Ok(());
    }

    pub fn emit_u64(&mut self, v: u64) -> io::Result<()> {
        try!(self.writer.write_u8(0x13));
        try!(self.writer.write_u64::<LittleEndian>(v));

        return Ok(());
    }

    pub fn emit_i8(&mut self, v: i8) -> io::Result<()> {
        try!(self.writer.write_u8(0x20));
        try!(self.writer.write_i8(v));

        return Ok(());
    }

    pub fn emit_i16(&mut self, v: i16) -> io::Result<()> {
        try!(self.writer.write_u8(0x21));
        try!(self.writer.write_i16::<LittleEndian>(v));

        return Ok(());
    }

    pub fn emit_i32(&mut self, v: i32) -> io::Result<()> {
        try!(self.writer.write_u8(0x22));
        try!(self.writer.write_i32::<LittleEndian>(v));

        return Ok(());
    }

    pub fn emit_i64(&mut self, v: i64) -> io::Result<()> {
        try!(self.writer.write_u8(0x23));
        try!(self.writer.write_i64::<LittleEndian>(v));

        return Ok(());
    }

    pub fn emit_fixnum_from_u64(&mut self, v: u64) -> io::Result<()> {
        // TODO: This does not produce canonical fixnums for numbers 5 and 6 bytes long
        // TODO: This should probably use some sort of clz thing
        if v < 16  {
            try!(self.writer.write_u8(0x80 + (v as u8 & 0x1F)));
        } else if v <= i8::MAX as u64 {
            try!(self.writer.write_u8(0x24));
            try!(self.writer.write_i8(v as i8));
        } else if v <= i16::MAX as u64 {
            try!(self.writer.write_u8(0x25));
            try!(self.writer.write_i16::<LittleEndian>(v as i16));
        } else if v <= i32::MAX as u64 {
            try!(self.writer.write_u8(0x26));
            try!(self.writer.write_i32::<LittleEndian>(v as i32));
        } else if v <= i64::MAX as u64 {
            try!(self.writer.write_u8(0x27));
            try!(self.writer.write_i64::<LittleEndian>(v as i64));
        } else {
            try!(self.writer.write_u8(0x28));
            try!(self.emit_length(9));
            try!(self.writer.write_u8(0x00));
            try!(self.writer.write_u64::<LittleEndian>(v));
        }

        return Ok(());
    }

    pub fn emit_fixnum_from_i64(&mut self, v: i64) -> io::Result<()> {
        // TODO: This does not produce canonical fixnums for numbers 5 and 6 bytes long
        // TODO: This should probably use some sort of clz thing
        if v >= -16 && v < 16 {
            try!(self.writer.write_u8(0x80 + (v as u8 & 0x1F)));
        } else if v >= i8::MIN as i64 && v <= i8::MAX as i64 {
            try!(self.writer.write_u8(0x24));
            try!(self.writer.write_i8(v as i8));
        } else if v >= i16::MIN as i64 && v <= i16::MAX as i64 {
            try!(self.writer.write_u8(0x25));
            try!(self.writer.write_i16::<LittleEndian>(v as i16));
        } else if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
            try!(self.writer.write_u8(0x26));
            try!(self.writer.write_i32::<LittleEndian>(v as i32));
        } else {
            try!(self.writer.write_u8(0x27));
            try!(self.writer.write_i64::<LittleEndian>(v as i64));
        }

        return Ok(());
    }

    pub fn emit_f32(&mut self, v: f32) -> io::Result<()> {
        try!(self.writer.write_u8(0x32));
        try!(self.writer.write_f32::<LittleEndian>(v));

        return Ok(());
    }

    pub fn emit_f64(&mut self, v: f64) -> io::Result<()> {
        try!(self.writer.write_u8(0x33));
        try!(self.writer.write_f64::<LittleEndian>(v));

        return Ok(());
    }

    pub fn emit_binary(&mut self, v: &[u8]) -> io::Result<()> {
        try!(self.writer.write_u8(0x40));
        try!(self.emit_length(v.len() as u64));
        try!(self.writer.write_all(v));

        return Ok(());
    }

    pub fn emit_string(&mut self, v: &str) -> io::Result<()> {
        if v.len() < 0x0F && !self.dictionary.contains_key(v) {
            try!(self.writer.write_u8(0xC0 + (v.len() as u8)));
            try!(self.writer.write_all(v.as_bytes()));
        } else {
            try!(self.writer.write_u8(0x41));
            try!(self.emit_string_fragment(v));
        }

        return Ok(());
    }

    pub fn emit_array_header(&mut self, length: u64) -> io::Result<()> {
        if length < 0x0F {
            try!(self.writer.write_u8(0xB0 + (length as u8)));
        } else {
            try!(self.writer.write_u8(0x50));
            try!(self.emit_length(length));
        }

        return Ok(());
    }

    pub fn emit_map_header(&mut self, length: u64) -> io::Result<()> {
        if length < 0x0F {
            try!(self.writer.write_u8(0xA0 + (length as u8)));
        } else {
            try!(self.writer.write_u8(0x51));
            try!(self.emit_length(length));
        }

        return Ok(());
    }

    pub fn emit_struct_header(&mut self, ty: &str, length: u64) -> io::Result<()> {
        try!(self.writer.write_u8(0x60));
        try!(self.emit_string_fragment(ty));
        try!(self.emit_length(length));

        return Ok(());
    }

    pub fn emit_object_header(&mut self, ty: &str, length: u64) -> io::Result<()> {
        try!(self.writer.write_u8(0x61));
        try!(self.emit_string_fragment(ty));
        try!(self.emit_length(length));

        return Ok(());
    }

    pub fn emit_guid(&mut self, v: [u8; 16]) -> io::Result<()> {
        try!(self.writer.write_u8(0x70));
        try!(self.writer.write_all(&v));

        return Ok(());
    }
}

#[cfg(test)]
mod tests {
    use super::Encoder;

    #[test]
    fn encodes_short_length() {
        let mut result = Vec::new();

        {
            Encoder::new(&mut result, vec![]).emit_length(0x00).unwrap();
        }

        assert_eq!(result, vec![0x00]);

        let mut result = Vec::new();

        {
            Encoder::new(&mut result, vec![]).emit_length(0xEF).unwrap();
        }

        assert_eq!(result, vec![0xEF]);
    }

    #[test]
    fn encodes_u8_length() {
        let mut result = Vec::new();

        {
            Encoder::new(&mut result, vec![]).emit_length(0xF0).unwrap();
        }

        assert_eq!(result, vec![0xF0, 0xF0]);

        let mut result = Vec::new();

        {
            Encoder::new(&mut result, vec![]).emit_length(0xFF).unwrap();
        }

        assert_eq!(result, vec![0xF0, 0xFF]);
    }

    #[test]
    fn encodes_u16_length() {
        let mut result = Vec::new();

        {
            Encoder::new(&mut result, vec![]).emit_length(0xFFFF).unwrap();
        }

        assert_eq!(result, vec![0xF1, 0xFF, 0xFF]);
    }

    #[test]
    fn encodes_u32_length() {
        let mut result = Vec::new();

        {
            Encoder::new(&mut result, vec![]).emit_length(0xFFFFFFFF).unwrap();
        }

        assert_eq!(result, vec![0xF2, 0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn encodes_u64_length() {
        let mut result = Vec::new();

        {
            Encoder::new(&mut result, vec![]).emit_length(0xFFFFFFFFFFFFFFFF).unwrap();
        }

        assert_eq!(result, vec![0xF3, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    }
}
