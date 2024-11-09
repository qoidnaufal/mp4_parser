use std::io::{self, Read, Seek};

use crate::{
    box_start, read_box_header_ext, skip_box, skip_bytes_to, BigEndian, BoxHeader, BoxType, Mp4Box,
    ReadBox, HEADER_EXT_SIZE, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlBox {
    pub version: u8,
    pub flags: u32,
    pub location: String,
}

impl Default for UrlBox {
    fn default() -> Self {
        Self {
            version: 0,
            flags: 1,
            location: String::default(),
        }
    }
}

impl UrlBox {
    fn get_type(&self) -> BoxType {
        BoxType::UrlBox
    }

    fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + HEADER_EXT_SIZE;
        if !self.location.is_empty() {
            size += self.location.bytes().len() as u64 + 1;
        }
        size
    }
}

impl Mp4Box for UrlBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for UrlBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let (version, flags) = read_box_header_ext(reader)?;
        let buf_size = size
            .checked_sub(HEADER_SIZE + HEADER_EXT_SIZE)
            .ok_or(io::Error::new(
                io::ErrorKind::InvalidData,
                "url size too small",
            ))?;

        let mut buf = vec![0u8; buf_size as _];
        reader.read_exact(&mut buf)?;

        if let Some(end) = buf.iter().position(|b| *b == b'0') {
            buf.truncate(end);
        }
        let location = String::from_utf8(buf).unwrap_or_default();

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            location,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrefBox {
    pub version: u8,
    pub flags: u32,
    pub url: Option<UrlBox>,
}

impl Default for DrefBox {
    fn default() -> Self {
        Self {
            version: 0,
            flags: 0,
            url: Some(UrlBox::default()),
        }
    }
}

impl DrefBox {
    fn get_type(&self) -> BoxType {
        BoxType::DrefBox
    }

    fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + HEADER_EXT_SIZE + 4;
        if let Some(ref url) = self.url {
            size += url.box_size();
        }
        size
    }
}

impl Mp4Box for DrefBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for DrefBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let mut current = reader.stream_position()?;
        let (version, flags) = read_box_header_ext(reader)?;
        let end = start + size;

        let mut url: Option<UrlBox> = None;

        let entry_count = BigEndian::read_u32(reader)?;
        for _ in 0..entry_count {
            if current >= end {
                break;
            }

            let header = BoxHeader::read(reader)?;

            if header.size > size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "dinf box contains a box wiht larger size than itself",
                ));
            }

            match header.name {
                BoxType::UrlBox => {
                    url.replace(UrlBox::read_box(reader, header.size)?);
                }
                _ => {
                    skip_box(reader, header.size)?;
                }
            }

            current = reader.stream_position()?;
        }

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            url,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DinfBox {
    dref: DrefBox,
}

impl DinfBox {
    fn get_type(&self) -> BoxType {
        BoxType::DinfBox
    }

    fn get_size(&self) -> u64 {
        HEADER_SIZE + self.dref.box_size()
    }
}

impl Mp4Box for DinfBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for DinfBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let mut dref: Option<DrefBox> = None;
        let mut current = reader.stream_position()?;
        let end = start + size;

        while current < end {
            let header = BoxHeader::read(reader)?;

            if header.size > size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "dinf box contains a box with a larger size than itself",
                ));
            }

            match header.name {
                BoxType::DrefBox => {
                    dref.replace(DrefBox::read_box(reader, header.size)?);
                }
                _ => {
                    skip_box(reader, header.size)?;
                }
            }

            current = reader.stream_position()?;
        }

        let Some(drefbox) = dref else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "DrefBox is not found",
            ));
        };

        skip_bytes_to(reader, start + size)?;

        Ok(Self { dref: drefbox })
    }
}
