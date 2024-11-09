use std::io::{self, Read, Seek};

use crate::{
    box_start, read_box_header_ext, skip_bytes_to, BigEndian, BoxType, FixedPointI8, Mp4Box,
    ReadBox, HEADER_EXT_SIZE, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmhdBox {
    pub version: u8,
    pub flags: u32,
    pub balance: FixedPointI8,
}

impl Default for SmhdBox {
    fn default() -> Self {
        Self {
            version: 0,
            flags: 0,
            balance: FixedPointI8::new_raw(0),
        }
    }
}

impl SmhdBox {
    fn get_type(&self) -> BoxType {
        BoxType::SmhdBox
    }

    fn get_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 4
    }
}

impl Mp4Box for SmhdBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for SmhdBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let (version, flags) = read_box_header_ext(reader)?;
        let balance = FixedPointI8::new_raw(BigEndian::read_i16(reader)?);

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            balance,
        })
    }
}
