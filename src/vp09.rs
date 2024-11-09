use std::io::{self, Read, Seek};

use crate::{
    box_start, read_box_header_ext, skip_bytes_to, vpcc::VpccBox, BigEndian, BoxHeader, BoxType,
    Mp4Box, RawBox, ReadBox,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Vp09Box {
    pub version: u8,
    pub flags: u32,
    pub start_code: u16,
    pub data_reference_index: u16,
    pub reserved0: [u8; 16],
    pub width: u16,
    pub height: u16,
    pub horizresolution: (u16, u16),
    pub vertresolution: (u16, u16),
    pub reserved1: [u8; 4],
    pub frame_count: u16,
    pub compressorname: [u8; 32],
    pub depth: u16, // This is usually 24, even for HDR with bit_depth=10
    pub end_code: u16,
    pub vpcc: RawBox<VpccBox>,
}

impl Mp4Box for Vp09Box {
    fn box_type(&self) -> BoxType {
        BoxType::Vp09Box
    }

    fn box_size(&self) -> u64 {
        0x6A
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for Vp09Box {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let (version, flags) = read_box_header_ext(reader)?;

        let start_code = BigEndian::read_u16(reader)?;
        let data_reference_index = BigEndian::read_u16(reader)?;
        let reserved0 = {
            let mut buf = [0u8; 16];
            reader.read_exact(&mut buf)?;
            buf
        };
        let width = BigEndian::read_u16(reader)?;
        let height = BigEndian::read_u16(reader)?;
        let horizresolution = (BigEndian::read_u16(reader)?, BigEndian::read_u16(reader)?);
        let vertresolution = (BigEndian::read_u16(reader)?, BigEndian::read_u16(reader)?);
        let reserved1 = {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            buf
        };
        let frame_count = BigEndian::read_u16(reader)?;
        let compressorname = {
            let mut buf = [0u8; 32];
            reader.read_exact(&mut buf)?;
            buf
        };
        let depth = BigEndian::read_u16(reader)?;
        let end_code = BigEndian::read_u16(reader)?;

        let vpcc = {
            let header = BoxHeader::read(reader)?;
            if header.size > size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "vp09 contains a box with a larger size than itself",
                ));
            }
            RawBox::<VpccBox>::read_box(reader, header.size)?
        };

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            start_code,
            data_reference_index,
            reserved0,
            width,
            height,
            horizresolution,
            vertresolution,
            reserved1,
            frame_count,
            compressorname,
            depth,
            end_code,
            vpcc,
        })
    }
}
