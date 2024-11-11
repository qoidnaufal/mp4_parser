use std::io::{self, Read, Seek};

use crate::{
    box_start, hdlr::HdlrBox, mdhd::MdhdBox, minf::MinfBox, skip_box, skip_bytes_to, BoxHeader,
    BoxType, Mp4Box, ReadBox, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MdiaBox {
    pub mdhd: MdhdBox,
    pub hdlr: HdlrBox,
    pub minf: MinfBox,
}

impl MdiaBox {
    fn get_type(&self) -> BoxType {
        BoxType::MdiaBox
    }

    fn get_size(&self) -> u64 {
        HEADER_SIZE + self.mdhd.box_size() + self.hdlr.box_size() + self.minf.box_size()
    }
}

impl Mp4Box for MdiaBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for MdiaBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;

        let mut mdhd = None;
        let mut hdlr = None;
        let mut minf = None;

        let mut current = reader.stream_position()?;
        let end = start + size;

        while current < end {
            let header = BoxHeader::read(reader)?;

            if header.size > size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "mdia box contains a box with a larger size than itself",
                ));
            }

            match header.name {
                BoxType::MdhdBox => {
                    mdhd.replace(MdhdBox::read_box(reader, header.size)?);
                }
                BoxType::HdlrBox => {
                    hdlr.replace(HdlrBox::read_box(reader, header.size)?);
                }
                BoxType::MinfBox => {
                    minf.replace(MinfBox::read_box(reader, header.size)?);
                }
                _ => {
                    skip_box(reader, header.size)?;
                }
            }

            current = reader.stream_position()?;
        }

        let Some(mdhd) = mdhd else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "mdhd not found"));
        };
        let Some(hdlr) = hdlr else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "hdlr not found"));
        };
        let Some(minf) = minf else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "minf not found"));
        };

        skip_bytes_to(reader, start + size)?;

        Ok(Self { mdhd, hdlr, minf })
    }
}
