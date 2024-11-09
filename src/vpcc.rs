use std::io::{self, Read, Seek};

use crate::{
    box_start, read_box_header_ext, skip_bytes_to, BigEndian, BoxType, Mp4Box, ReadBox,
    HEADER_EXT_SIZE, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VpccBox {
    pub version: u8,
    pub flags: u32,
    pub profile: u8,
    pub level: u8,
    pub bit_depth: u8,
    pub chroma_subsampling: u8,
    pub video_full_range_flag: bool,
    pub color_primaries: u8,
    pub transfer_characteristics: u8,
    pub matrix_coefficients: u8,
    pub codec_initialization_data_size: u16,
}

impl VpccBox {
    pub const DEFAULT_VERSION: u8 = 0b1;
    pub const DEFAULT_BIT_DEPTH: u8 = 0b1000;
}

impl Mp4Box for VpccBox {
    fn box_type(&self) -> BoxType {
        BoxType::VpccBox
    }

    fn box_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 0b1000
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for VpccBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let (version, flags) = read_box_header_ext(reader)?;

        let profile = BigEndian::read_u8(reader)?;
        let level = BigEndian::read_u8(reader)?;

        let (bit_depth, chroma_subsampling, video_full_range_flag) = {
            let b = BigEndian::read_u8(reader)?;
            (b >> 4, b << 4 >> 5, b & 0x01 == 1)
        };

        let transfer_characteristics = BigEndian::read_u8(reader)?;
        let matrix_coefficients = BigEndian::read_u8(reader)?;
        let codec_initialization_data_size = BigEndian::read_u16(reader)?;

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            profile,
            level,
            bit_depth,
            chroma_subsampling,
            video_full_range_flag,
            color_primaries: 0,
            transfer_characteristics,
            matrix_coefficients,
            codec_initialization_data_size,
        })
    }
}
