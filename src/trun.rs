use std::io::{self, Read, Seek};

use crate::{
    box_start, read_box_header_ext, skip_bytes_to, BigEndian, BoxType, Mp4Box, ReadBox,
    HEADER_EXT_SIZE, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrunBox {
    pub version: u8,
    pub flags: u32,
    pub sample_count: u32,
    pub data_offset: Option<i32>,
    pub first_sample_flags: Option<u32>,
    pub sample_duration: Vec<u32>,
    pub sample_sizes: Vec<u32>,
    pub sample_flags: Vec<u32>,
    pub sample_cts: Vec<u32>,
}

impl TrunBox {
    pub const FLAG_DATA_OFFSET: u32 = 0x01;
    pub const FLAG_FIRST_SAMPLE_FLAGS: u32 = 0x04;
    pub const FLAG_SAMPLE_DURATION: u32 = 0x100;
    pub const FLAG_SAMPLE_SIZE: u32 = 0x200;
    pub const FLAG_SAMPLE_FLAGS: u32 = 0x400;
    pub const FLAG_SAMPLE_CTS: u32 = 0x800;

    fn get_type(&self) -> BoxType {
        BoxType::TrunBox
    }

    fn get_size(&self) -> u64 {
        let mut sum = HEADER_SIZE + HEADER_EXT_SIZE + 4;

        if Self::FLAG_DATA_OFFSET & self.flags > 0 {
            sum += 0b100;
        }
        if Self::FLAG_FIRST_SAMPLE_FLAGS & self.flags > 0 {
            sum += 0b100;
        }
        if Self::FLAG_SAMPLE_DURATION & self.flags > 0 {
            sum += 0b100;
        }
        if Self::FLAG_SAMPLE_SIZE & self.flags > 0 {
            sum += 0b100;
        }
        if Self::FLAG_SAMPLE_FLAGS & self.flags > 0 {
            sum += 0b100;
        }
        if Self::FLAG_SAMPLE_CTS & self.flags > 0 {
            sum += 0b100;
        }

        sum
    }
}

impl Mp4Box for TrunBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for TrunBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let (version, flags) = read_box_header_ext(reader)?;

        let header_size = HEADER_SIZE + HEADER_EXT_SIZE;
        let other_size = size_of::<u32>() // sample_count
            + if Self::FLAG_DATA_OFFSET & flags > 0 { size_of::<i32>() } else { 0 } // data_offset
            + if Self::FLAG_FIRST_SAMPLE_FLAGS & flags > 0 { size_of::<u32>() } else { 0 }; // first_sample_flags
        let sample_size = if Self::FLAG_SAMPLE_DURATION & flags > 0 { size_of::<u32>() } else { 0 } // sample_duration
            + if Self::FLAG_SAMPLE_SIZE & flags > 0 { size_of::<u32>() } else { 0 } // sample_size
            + if Self::FLAG_SAMPLE_FLAGS & flags > 0 { size_of::<u32>() } else { 0 } // sample_flags
            + if Self::FLAG_SAMPLE_CTS & flags > 0 { size_of::<u32>() } else { 0 }; // sample_composition_time_offset

        let sample_count = BigEndian::read_u32(reader)?;

        let data_offset = if Self::FLAG_DATA_OFFSET & flags > 0 {
            Some(BigEndian::read_i32(reader)?)
        } else {
            None
        };

        let first_sample_flags = if Self::FLAG_FIRST_SAMPLE_FLAGS & flags > 0 {
            Some(BigEndian::read_u32(reader)?)
        } else {
            None
        };

        let mut sample_duration = Vec::new();
        let mut sample_sizes = Vec::new();
        let mut sample_flags = Vec::new();
        let mut sample_cts = Vec::new();

        if u64::from(sample_count) * sample_size as u64
            > size
                .saturating_sub(header_size)
                .saturating_sub(other_size as u64)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "trun sample_count indicates more values than could fit in the box",
            ));
        }

        if Self::FLAG_SAMPLE_DURATION & flags > 0 {
            sample_duration.reserve(sample_count as usize);
        }
        if Self::FLAG_SAMPLE_SIZE & flags > 0 {
            sample_sizes.reserve(sample_count as usize);
        }
        if Self::FLAG_SAMPLE_FLAGS & flags > 0 {
            sample_flags.reserve(sample_count as usize);
        }
        if Self::FLAG_SAMPLE_CTS & flags > 0 {
            sample_cts.reserve(sample_count as usize);
        }

        for _ in 0..sample_count {
            if Self::FLAG_SAMPLE_DURATION & flags > 0 {
                let duration = BigEndian::read_u32(reader)?;
                sample_duration.push(duration)
            }
            if Self::FLAG_SAMPLE_SIZE & flags > 0 {
                let sample_size = BigEndian::read_u32(reader)?;
                sample_sizes.push(sample_size);
            }
            if Self::FLAG_SAMPLE_FLAGS & flags > 0 {
                let sample_flag = BigEndian::read_u32(reader)?;
                sample_flags.push(sample_flag);
            }
            if Self::FLAG_SAMPLE_CTS & flags > 0 {
                let cts = BigEndian::read_u32(reader)?;
                sample_cts.push(cts);
            }
        }

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            sample_count,
            data_offset,
            first_sample_flags,
            sample_duration,
            sample_sizes,
            sample_flags,
            sample_cts,
        })
    }
}
