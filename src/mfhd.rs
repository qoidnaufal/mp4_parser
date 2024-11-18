use std::io::{Read, Seek};

use crate::{
    box_start, read_box_header_ext, skip_bytes_to, BigEndian, BoxType, Mp4Box, ReadBox,
    HEADER_EXT_SIZE, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MfhdBox {
    pub version: u8,
    pub flags: u32,
    pub sequence_number: u32,
}

impl Default for MfhdBox {
    fn default() -> Self {
        Self {
            version: 0,
            flags: 0,
            sequence_number: 1,
        }
    }
}

impl MfhdBox {
    fn get_type(&self) -> BoxType {
        BoxType::MfhdBox
    }

    fn get_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 4
    }
}

impl Mp4Box for MfhdBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for MfhdBox {
    fn read_box(reader: &mut R, size: u64) -> std::io::Result<Self> {
        let start = box_start(reader)?;
        let (version, flags) = read_box_header_ext(reader)?;
        let sequence_number = BigEndian::read_u32(reader)?;

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            sequence_number,
        })
    }
}
