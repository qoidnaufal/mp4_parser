use std::io::{self, Read, Seek};

use crate::{
    box_start, meta::MetaBox, skip_box, skip_bytes_to, BoxHeader, BoxType, Mp4Box, ReadBox,
    HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UdtaBox {
    pub meta: Option<MetaBox>,
}

impl UdtaBox {
    fn get_type(&self) -> BoxType {
        BoxType::UdtaBox
    }

    fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE;

        if let Some(ref meta) = self.meta {
            size += meta.box_size()
        }

        size
    }
}

impl Mp4Box for UdtaBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for UdtaBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;

        let mut meta = None;

        let mut current = reader.stream_position()?;
        let end = start + size;

        while current < end {
            let header = BoxHeader::read(reader)?;

            if header.size > size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "udta box contains a box with a larger size than itself",
                ));
            }

            match header.name {
                BoxType::MetaBox => {
                    meta.replace(MetaBox::read_box(reader, header.size)?);
                }
                _ => {
                    skip_box(reader, header.size)?;
                }
            }

            current = reader.stream_position()?;
        }

        skip_bytes_to(reader, start + size)?;

        Ok(Self { meta })
    }
}
