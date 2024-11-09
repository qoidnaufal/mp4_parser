use std::io::{self, Read, Seek};

use crate::{
    box_start, read_box_header_ext, skip_bytes_to, BigEndian, BoxType, Mp4Box, ReadBox, RgbColor,
    HEADER_EXT_SIZE, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VmhdBox {
    pub version: u8,
    pub flags: u32,
    pub graphics_mode: u16,
    pub op_color: RgbColor,
}

impl VmhdBox {
    fn get_type(&self) -> BoxType {
        BoxType::VmhdBox
    }

    fn get_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 8
    }
}

impl Mp4Box for VmhdBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for VmhdBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let (version, flags) = read_box_header_ext(reader)?;
        let graphics_mode = BigEndian::read_u16(reader)?;
        let op_color = RgbColor {
            red: BigEndian::read_u16(reader)?,
            green: BigEndian::read_u16(reader)?,
            blue: BigEndian::read_u16(reader)?,
        };

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            graphics_mode,
            op_color,
        })
    }
}
