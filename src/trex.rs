use std::io::{self, Read, Seek};

use crate::{
    box_start, read_box_header_ext, skip_bytes_to, BigEndian, BoxType, Mp4Box, ReadBox,
    HEADER_EXT_SIZE, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrexBox {
    pub version: u8,
    pub flags: u32,
    pub track_id: u32,
    pub default_sample_description_index: u32,
    pub default_sample_duration: u32,
    pub default_sample_size: u32,
    pub default_sample_flags: u32,
}

impl TrexBox {
    fn get_type(&self) -> BoxType {
        BoxType::TrexBox
    }

    fn get_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 20
    }
}

impl Mp4Box for TrexBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for TrexBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let (version, flags) = read_box_header_ext(reader)?;

        let track_id = BigEndian::read_u32(reader)?;
        let default_sample_description_index = BigEndian::read_u32(reader)?;
        let default_sample_duration = BigEndian::read_u32(reader)?;
        let default_sample_size = BigEndian::read_u32(reader)?;
        let default_sample_flags = BigEndian::read_u32(reader)?;

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            track_id,
            default_sample_description_index,
            default_sample_duration,
            default_sample_size,
            default_sample_flags,
        })
    }
}
