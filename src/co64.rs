use std::io::{self, Read, Seek};

use crate::{
    box_start, read_box_header_ext, skip_bytes_to, BigEndian, BoxType, Mp4Box, ReadBox,
    HEADER_EXT_SIZE, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Co64Box {
    pub version: u8,
    pub flags: u32,
    pub entries: Vec<u64>,
}

impl Co64Box {
    fn get_type(&self) -> BoxType {
        BoxType::Co64Box
    }

    fn get_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 4 + (8 * self.entries.len() as u64)
    }
}

impl Mp4Box for Co64Box {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for Co64Box {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let (version, flags) = read_box_header_ext(reader)?;

        let header_size = HEADER_SIZE + HEADER_EXT_SIZE;
        let other_size = size_of::<u32>(); // entry_count
        let entry_size = size_of::<u64>(); // chunk_offset
        let entry_count = BigEndian::read_u32(reader)?;

        if u64::from(entry_count)
            > size
                .saturating_sub(header_size)
                .saturating_sub(other_size as u64)
                / entry_size as u64
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "co64 entry_count indicates more entries than could fit in the box",
            ));
        }

        let mut entries = Vec::with_capacity(entry_count as _);

        for _ in 0..entry_count {
            let chunk_offset = BigEndian::read_u64(reader)?;
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
