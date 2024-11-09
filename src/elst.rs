use std::io::{self, Read, Seek};

use crate::{
    box_start, read_box_header_ext, skip_bytes_to, BigEndian, BoxType, Mp4Box, ReadBox,
    HEADER_EXT_SIZE, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ElstBox {
    pub version: u8,
    pub flags: u32,
    pub entries: Vec<ElstEntry>,
}

impl ElstBox {
    fn get_type(&self) -> BoxType {
        BoxType::ElstBox
    }

    fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + HEADER_EXT_SIZE + 4;
        if self.version == 1 {
            size += self.entries.len() as u64 * 20;
        } else if self.version == 0 {
            size += self.entries.len() as u64 * 12;
        }
        size
    }
}

impl Mp4Box for ElstBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for ElstBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let (version, flags) = read_box_header_ext(reader)?;
        let header_size = HEADER_SIZE + HEADER_EXT_SIZE;
        let entry_count = BigEndian::read_u32(reader)?;

        let other_size = size_of::<i32>(); // entry_count

        let entry_size = {
            let mut entry_size = 0;
            entry_size += if version == 1 {
                size_of::<u64>() + size_of::<i64>() // segment_duration + media_time
            } else {
                size_of::<u32>() + size_of::<i32>() // segment_duration + media_time
            };
            entry_size += size_of::<i16>() + size_of::<i16>(); // media_rate_integer + media_rate_fraction
            entry_size
        };

        if u64::from(entry_count)
            > size
                .saturating_sub(header_size)
                .saturating_sub(other_size as u64)
                / entry_size as u64
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "elst entry_count indicates more entries than could fit in the box",
            ));
        }

        let mut entries: Vec<ElstEntry> = Vec::with_capacity(entry_count as _);

        for _ in 0..entry_count {
            let (segment_duration, media_time) = if version == 1 {
                let mut nums_arr = [0u64; 2];
                for i in 0..nums_arr.len() {
                    nums_arr[i] = BigEndian::read_u64(reader)?;
                }
                (nums_arr[0], nums_arr[1])
            } else {
                let mut nums_arr = [0u32; 2];
                for i in 0..nums_arr.len() {
                    nums_arr[i] = BigEndian::read_u32(reader)?;
                }
                (nums_arr[0] as u64, nums_arr[1] as u64)
            };

            let entry = ElstEntry {
                segment_duration,
                media_time,
                media_rate: BigEndian::read_u16(reader)?,
                media_rate_fraction: BigEndian::read_u16(reader)?,
            };
            entries.push(entry);
        }

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            entries,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ElstEntry {
    pub segment_duration: u64,
    pub media_time: u64,
    pub media_rate: u16,
    pub media_rate_fraction: u16,
}
