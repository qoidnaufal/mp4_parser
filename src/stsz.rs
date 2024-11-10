use std::io::{self, Read, Seek};

use crate::{
    box_start, read_box_header_ext, skip_bytes_to, BigEndian, BoxType, Mp4Box, ReadBox,
    HEADER_EXT_SIZE, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StszBox {
    pub version: u8,
    pub flags: u32,
    pub sample_size: u32,
    pub sample_count: u32,
    pub sample_sizes: Vec<u32>,
}

impl StszBox {
    fn get_type(&self) -> BoxType {
        BoxType::StszBox
    }

    fn get_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 8 + (4 * self.sample_sizes.len() as u64)
    }
}

impl Mp4Box for StszBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for StszBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let (version, flags) = read_box_header_ext(reader)?;

        let header_size = HEADER_SIZE + HEADER_EXT_SIZE;
        let other_size = size_of::<u32>() + size_of::<u32>(); // sample_size + sample_count
        let sample_size = BigEndian::read_u32(reader)?;
        let stsz_item_size = if sample_size == 0 {
            size_of::<u32>()
        } else {
            0
        };
        let sample_count = BigEndian::read_u32(reader)?;
        let mut sample_sizes: Vec<u32> = Vec::new();

        if sample_size == 0 {
            if u64::from(sample_count)
                > size
                    .saturating_sub(header_size)
                    .saturating_sub(other_size as u64)
                    / stsz_item_size as u64
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "stsz sample_count indicates more values than could fir in the box",
                ));
            }
            sample_sizes.reserve(sample_count as _);

            for _ in 0..sample_count {
                let sample_number = BigEndian::read_u32(reader)?;
                sample_sizes.push(sample_number)
            }
        }

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            sample_size,
            sample_count,
            sample_sizes,
        })
    }
}
