use std::io::{self, Read, Seek};

use crate::{
    box_start, skip_bytes_to, BigEndian, BoxType, Mp4Box, ReadBox, RgbaColor, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tx3gBox {
    pub data_reference_index: u16,
    pub display_flags: u32,
    pub horizontal_justification: i8,
    pub vertical_justification: i8,
    pub bg_color_rgba: RgbaColor,
    pub box_record: [i16; 4],
    pub style_record: [u8; 12],
}

impl Default for Tx3gBox {
    fn default() -> Self {
        Self {
            data_reference_index: 0,
            display_flags: 0,
            horizontal_justification: 1,
            vertical_justification: -1,
            bg_color_rgba: RgbaColor {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 255,
            },
            box_record: [0i16; 4],
            style_record: [0, 0, 0, 0, 0, 1, 0, 16, 255, 255, 255, 255],
        }
    }
}

impl Tx3gBox {
    fn get_type(&self) -> BoxType {
        BoxType::Tx3gBox
    }

    fn get_size(&self) -> u64 {
        HEADER_SIZE + 6 + 32
    }
}

impl Mp4Box for Tx3gBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for Tx3gBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;

        let _ = BigEndian::read_u32(reader)?; // reserved
        let _ = BigEndian::read_u16(reader)?; // reserved
        let data_reference_index = BigEndian::read_u16(reader)?;

        let display_flags = BigEndian::read_u32(reader)?;
        let horizontal_justification = BigEndian::read_i8(reader)?;
        let vertical_justification = BigEndian::read_i8(reader)?;
        let bg_color_rgba = RgbaColor {
            red: BigEndian::read_u8(reader)?,
            green: BigEndian::read_u8(reader)?,
            blue: BigEndian::read_u8(reader)?,
            alpha: BigEndian::read_u8(reader)?,
        };
        let box_record: [i16; 4] = [
            BigEndian::read_i16(reader)?,
            BigEndian::read_i16(reader)?,
            BigEndian::read_i16(reader)?,
            BigEndian::read_i16(reader)?,
        ];
        let style_record: [u8; 12] = [
            BigEndian::read_u8(reader)?,
            BigEndian::read_u8(reader)?,
            BigEndian::read_u8(reader)?,
            BigEndian::read_u8(reader)?,
            BigEndian::read_u8(reader)?,
            BigEndian::read_u8(reader)?,
            BigEndian::read_u8(reader)?,
            BigEndian::read_u8(reader)?,
            BigEndian::read_u8(reader)?,
            BigEndian::read_u8(reader)?,
            BigEndian::read_u8(reader)?,
            BigEndian::read_u8(reader)?,
        ];

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            data_reference_index,
            display_flags,
            horizontal_justification,
            vertical_justification,
            bg_color_rgba,
            box_record,
            style_record,
        })
    }
}
