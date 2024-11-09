use std::io::{self, Read, Seek};

use crate::{
    box_start, read_box_header_ext, skip_bytes_to, BigEndian, BoxType, FixedPointU16, FixedPointU8,
    Matrix, Mp4Box, ReadBox, HEADER_EXT_SIZE, HEADER_SIZE,
};

enum TrackFlag {
    TrackEnabled = 0x000001,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TkhdBox {
    pub version: u8,
    pub flags: u32,
    pub creation_time: u64,
    pub modification_time: u64,
    pub track_id: u32,
    pub duration: u64,
    pub layer: u16,
    pub alternate_group: u16,
    pub volume: FixedPointU8,
    pub matrix: Matrix,
    pub width: FixedPointU16,
    pub height: FixedPointU16,
}

impl Default for TkhdBox {
    fn default() -> Self {
        Self {
            version: 0,
            flags: TrackFlag::TrackEnabled as u32,
            creation_time: 0,
            modification_time: 0,
            track_id: 0,
            duration: 0,
            layer: 0,
            alternate_group: 0,
            volume: FixedPointU8::new(1),
            matrix: Matrix::default(),
            width: FixedPointU16::new(0),
            height: FixedPointU16::new(0),
        }
    }
}

impl TkhdBox {
    fn get_type(&self) -> BoxType {
        BoxType::TkhdBox
    }

    fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + HEADER_EXT_SIZE;
        if self.version == 1 {
            size += 32;
        } else if self.version == 0 {
            size += 20;
        }
        size += 60;
        size
    }

    pub fn set_width(&mut self, width: u16) {
        self.width = FixedPointU16::new(width)
    }

    pub fn set_height(&mut self, height: u16) {
        self.height = FixedPointU16::new(height)
    }
}

impl Mp4Box for TkhdBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for TkhdBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let (version, flags) = read_box_header_ext(reader)?;
        let (creation_time, modification_time, track_id, _, duration) = if version == 1 {
            let mut num64 = [0u64; 2];
            let mut num32 = [0u32; 2];

            for i in 0..2 {
                num64[i] = BigEndian::read_u64(reader)?;
            }

            for i in 0..2 {
                num32[i] = BigEndian::read_u32(reader)?;
            }

            let num = BigEndian::read_u64(reader)?;

            (num64[0], num64[1], num32[0], num32[1], num)
        } else if version == 0 {
            let mut nums = [0u32; 5];

            for i in 0..5 {
                nums[i] = BigEndian::read_u32(reader)?;
            }

            (
                nums[0] as u64,
                nums[1] as u64,
                nums[2],
                nums[3],
                nums[4] as u64,
            )
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "version must be 0 or 1",
            ));
        };

        let _ = BigEndian::read_u64(reader)?; // reserved

        let mut array_u16 = [0u16; 3];
        for i in 0..array_u16.len() {
            array_u16[i] = BigEndian::read_u16(reader)?;
        }

        let layer = array_u16[0];
        let alternate_group = array_u16[1];
        let volume = FixedPointU8::new_raw(array_u16[2]);

        let _ = BigEndian::read_u16(reader)?; // reserved

        let matrix = Matrix::read_i32(reader)?;
        let width = FixedPointU16::new_raw(BigEndian::read_u32(reader)?);
        let height = FixedPointU16::new_raw(BigEndian::read_u32(reader)?);

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            creation_time,
            modification_time,
            track_id,
            duration,
            layer,
            alternate_group,
            volume,
            matrix,
            width,
            height,
        })
    }
}
