use std::io::{self, Read, Seek};

use crate::{
    box_start, read_box_header_ext, skip_bytes_to, BigEndian, BoxType, Mp4Box, ReadBox,
    HEADER_EXT_SIZE, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdhdBox {
    pub version: u8,
    pub flags: u32,
    pub creation_time: u64,
    pub modification_time: u64,
    pub timescale: u32,
    pub duration: u64,
    pub language: String,
}

impl Default for MdhdBox {
    fn default() -> Self {
        Self {
            version: 0,
            flags: 0,
            creation_time: 0,
            modification_time: 0,
            timescale: 1000,
            duration: 0,
            language: String::from("und"),
        }
    }
}

impl MdhdBox {
    fn get_type(&self) -> BoxType {
        BoxType::MdhdBox
    }

    fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + HEADER_EXT_SIZE;
        if self.version == 1 {
            size += 28;
        } else if self.version == 0 {
            size += 16;
        }
        size += 4;
        size
    }
}

impl Mp4Box for MdhdBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for MdhdBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let (version, flags) = read_box_header_ext(reader)?;
        let (creation_time, modification_time, timescale, duration) = if version == 1 {
            (
                BigEndian::read_u64(reader)?,
                BigEndian::read_u64(reader)?,
                BigEndian::read_u32(reader)?,
                BigEndian::read_u64(reader)?,
            )
        } else if version == 0 {
            (
                BigEndian::read_u32(reader)? as u64,
                BigEndian::read_u32(reader)? as u64,
                BigEndian::read_u32(reader)?,
                BigEndian::read_u32(reader)? as u64,
            )
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "version must be 0 or 1",
            ));
        };
        let language_code = BigEndian::read_u16(reader)?;
        let language = language_string(language_code);

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            creation_time,
            modification_time,
            timescale,
            duration,
            language,
        })
    }
}

fn language_string(language_code: u16) -> String {
    let mut lang = [0u16; 3];
    lang[0] = ((language_code >> 10) & 0x1F) + 0x60;
    lang[1] = ((language_code >> 5) & 0x1F) + 0x60;
    lang[2] = ((language_code) & 0x1F) + 0x60;

    std::char::decode_utf16(lang.iter().copied())
        .map(|r| r.unwrap_or(std::char::REPLACEMENT_CHARACTER))
        .collect::<String>()
}
