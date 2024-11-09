use std::io::{self, Read, Seek};

use crate::{
    box_start, read_box_header_ext, skip_bytes_to, BigEndian, BoxType, Mp4Box, ReadBox,
    HEADER_EXT_SIZE, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MehdBox {
    pub version: u8,
    pub flags: u32,
    pub fragment_duration: u64,
}

impl MehdBox {
    fn get_type(&self) -> BoxType {
        BoxType::MehdBox
    }

    fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + HEADER_EXT_SIZE;

        if self.version == 1 {
            size += 8;
        } else if self.version == 0 {
            size += 4;
        }

        size
    }
}

impl Mp4Box for MehdBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for MehdBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let (version, flags) = read_box_header_ext(reader)?;
        let fragment_duration = if version == 1 {
            BigEndian::read_u64(reader)?
        } else if version == 0 {
            BigEndian::read_u32(reader)? as u64
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "version must be 0 or 1",
            ));
        };

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            fragment_duration,
        })
    }
}