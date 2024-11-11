use std::io::{self, Read, Seek};

use crate::{
    box_start, read_box_header_ext, skip_bytes, skip_bytes_to, BigEndian, BoxType, FixedPointU16,
    FixedPointU8, Matrix, Mp4Box, ReadBox, HEADER_EXT_SIZE, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MvhdBox {
    pub version: u8,
    pub flags: u32,
    pub creation_time: u64,
    pub modification_time: u64,
    pub timescale: u32,
    pub duration: u64,
    pub rate: FixedPointU16,
    pub volume: FixedPointU8,
    pub matrix: Matrix,
    pub next_track_id: u32,
}

impl Default for MvhdBox {
    fn default() -> Self {
        Self {
            version: 0,
            flags: 0,
            creation_time: 0,
            modification_time: 0,
            timescale: 1000,
            duration: 0,
            rate: FixedPointU16::new(1),
            volume: FixedPointU8::new(1),
            matrix: Matrix::default(),
            next_track_id: 1,
        }
    }
}

impl MvhdBox {
    fn get_type(&self) -> BoxType {
        BoxType::MvhdBox
    }

    fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + HEADER_EXT_SIZE;

        if self.version == 1 {
            size += 28;
        } else if self.version == 0 {
            size += 16;
        }

        size += 80;
        size
    }
}

impl Mp4Box for MvhdBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for MvhdBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let (version, flags) = read_box_header_ext(reader)?;
        let (creation_time, modification_time, timescale, duration) = if version == 1 {
            let num0 = BigEndian::read_u64(reader)?;
            let num1 = BigEndian::read_u64(reader)?;
            let num2 = BigEndian::read_u32(reader)?;
            let num3 = BigEndian::read_u64(reader)?;

            (num0, num1, num2, num3)
        } else if version == 0 {
            let num0 = BigEndian::read_u32(reader)? as u64;
            let num1 = BigEndian::read_u32(reader)? as u64;
            let num2 = BigEndian::read_u32(reader)?;
            let num3 = BigEndian::read_u32(reader)? as u64;

            (num0, num1, num2, num3)
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "version must be 0 or 1",
            ));
        };

        let num0 = BigEndian::read_u32(reader)?;
        let rate = FixedPointU16::new_raw(num0);

        let num1 = BigEndian::read_u16(reader)?;
        let volume = FixedPointU8::new_raw(num1);

        let _ = BigEndian::read_u16(reader)?; // reserved = 0
        let _ = BigEndian::read_u16(reader)?; // reserved = 0

        let matrix = Matrix::read_i32(reader)?;

        skip_bytes(reader, 24)?;

        let next_track_id = BigEndian::read_u32(reader)?;

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            creation_time,
            modification_time,
            timescale,
            duration,
            rate,
            volume,
            matrix,
            next_track_id,
        })
    }
}
