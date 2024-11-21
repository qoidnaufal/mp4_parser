use std::io::{self, Read, Seek};

use crate::{
    box_start, co64::Co64Box, read_box_header_ext, skip_bytes_to, BigEndian, BoxType, Mp4Box,
    ReadBox, HEADER_EXT_SIZE, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StcoBox {
    pub version: u8,
    pub flags: u32,
    pub entries: Vec<u32>,
}

impl StcoBox {
    fn get_type(&self) -> BoxType {
        BoxType::StcoBox
    }

    fn get_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 4 + (4 * self.entries.len() as u64)
    }
}

impl Mp4Box for StcoBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for StcoBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let (version, flags) = read_box_header_ext(reader)?;

        let header_size = HEADER_SIZE + HEADER_EXT_SIZE;
        let other_size = size_of::<u32>(); // entry_count
        let entry_size = size_of::<u32>(); // chunk_offset
        let entry_count = BigEndian::read_u32(reader)?;

        if u64::from(entry_count)
            > size
                .saturating_sub(header_size)
                .saturating_sub(other_size as u64)
                / entry_size as u64
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "stco entry_count indicates more entries than could fit in the box",
            ));
        }

        let mut entries = Vec::with_capacity(entry_count as _);

        for _ in 0..entry_count {
            let chunk_offset = BigEndian::read_u32(reader)?;
            entries.push(chunk_offset)
        }

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            entries,
        })
    }
}

impl std::convert::TryFrom<&Co64Box> for StcoBox {
    type Error = std::num::TryFromIntError;

    fn try_from(co64: &Co64Box) -> Result<Self, Self::Error> {
        let entries = co64
            .entries
            .iter()
            .copied()
            .map(u32::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            version: 0,
            flags: 0,
            entries,
        })
    }
}
