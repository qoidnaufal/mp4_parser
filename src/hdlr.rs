use std::io::{self, Read, Seek};

use crate::{
    box_start, read_box_header_ext, skip_bytes, skip_bytes_to, BigEndian, BoxType, FourCC, Mp4Box,
    ReadBox, HEADER_EXT_SIZE, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HdlrBox {
    pub version: u8,
    pub flags: u32,
    pub handler_type: FourCC,
    pub name: String,
}

impl HdlrBox {
    fn get_type(&self) -> BoxType {
        BoxType::HdlrBox
    }

    fn get_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 20 + self.name.len() as u64 + 1
    }
}

impl Mp4Box for HdlrBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for HdlrBox {
    fn read_box(reader: &mut R, size: u64) -> std::io::Result<Self> {
        let start = box_start(reader)?;
        let (version, flags) = read_box_header_ext(reader)?;

        let _ = BigEndian::read_u32(reader)?; // pre-defined
        let handler = BigEndian::read_u32(reader)?;

        skip_bytes(reader, 12)?; // reserved

        let buf_size =
            size.checked_sub(HEADER_SIZE + HEADER_EXT_SIZE + 20)
                .ok_or(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "hdlr size too small",
                ))?;

        let mut buf = vec![0u8; buf_size as _];
        reader.read_exact(&mut buf)?;
        if let Some(end) = buf.iter().position(|b| *b == b'\0') {
            buf.truncate(end);
        }
        let name = String::from_utf8(buf).unwrap_or_default();

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            handler_type: FourCC::from(handler),
            name,
        })
    }
}
