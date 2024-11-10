use std::io::{self, Read, Seek};

use crate::{
    box_start, read_box_header_ext, skip_bytes_to, BigEndian, BoxType, Mp4Box, ReadBox,
    HEADER_EXT_SIZE, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StscBox {
    pub version: u8,
    pub flags: u32,
    pub entries: Vec<StscEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StscEntry {
    pub first_chunk: u32,
    pub samples_per_chunk: u32,
    pub sample_description_index: u32,
    pub first_sample: u32,
}

impl StscBox {
    fn get_type(&self) -> BoxType {
        BoxType::StscBox
    }

    fn get_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 4 + (12 * self.entries.len() as u64)
    }
}

impl Mp4Box for StscBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for StscBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let (version, flags) = read_box_header_ext(reader)?;

        let header_size = HEADER_SIZE + HEADER_EXT_SIZE;
        let other_size = size_of::<u32>(); // entry_count
                                           // ..............first_cunk........samples_per_chunk..sample_description_index
        let entry_size = size_of::<u32>() + size_of::<u32>() + size_of::<u32>();
        let entry_count = BigEndian::read_u32(reader)?;

        if u64::from(entry_count)
            > size
                .saturating_sub(header_size)
                .saturating_sub(other_size as u64)
                / entry_size as u64
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "stsc entry_count indicates more entries than it could fit in the box",
            ));
        }

        let mut entries = Vec::with_capacity(entry_count as _);

        for _ in 0..entry_count {
            let entry = StscEntry {
                first_chunk: BigEndian::read_u32(reader)?,
                samples_per_chunk: BigEndian::read_u32(reader)?,
                sample_description_index: BigEndian::read_u32(reader)?,
                first_sample: 0,
            };
            entries.push(entry);
        }

        let mut sample_id = 1;

        for i in 0..entry_count {
            let (first_chunk, samples_per_chunk) = {
                let entry = &mut entries[i as usize];
                entry.first_sample = sample_id;
                (entry.first_chunk, entry.samples_per_chunk)
            };

            if i < entry_count - 1 {
                let next_entry = &entries[i as usize + 1];
                sample_id = next_entry
                    .first_chunk
                    .checked_sub(first_chunk)
                    .and_then(|n| n.checked_mul(samples_per_chunk))
                    .and_then(|n| n.checked_add(sample_id))
                    .ok_or(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "attempt to calculate stsc sample_id with overflow",
                    ))?;
            }
        }

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            entries,
        })
    }
}
