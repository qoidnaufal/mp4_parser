use std::io::{self, Read, Seek};

use crate::{
    box_start, edts::EdtsBox, mdia::MdiaBox, meta::MetaBox, skip_box, skip_bytes_to, tkhd::TkhdBox,
    BoxHeader, BoxType, Mp4Box, ReadBox, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrakBox {
    pub tkhd: TkhdBox,
    pub edts: Option<EdtsBox>,
    pub meta: Option<MetaBox>,
    pub mdia: MdiaBox,
}

impl TrakBox {
    fn get_type(&self) -> BoxType {
        BoxType::TrakBox
    }

    fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE;
        size += self.tkhd.box_size();

        if let Some(ref edts) = self.edts {
            size += edts.box_size()
        }
        size += self.mdia.box_size();

        size
    }
}

impl Mp4Box for TrakBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for TrakBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;

        let mut tkhd = None;
        let mut edts = None;
        let mut meta = None;
        let mut mdia = None;

        let mut current = reader.stream_position()?;
        let end = start + size;

        while current < end {
            let header = BoxHeader::read(reader)?;

            if header.size > size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "trak box contains a box with a larger size than itself",
                ));
            }

            match header.name {
                BoxType::TkhdBox => {
                    tkhd.replace(TkhdBox::read_box(reader, header.size)?);
                }
                BoxType::EdtsBox => {
                    edts.replace(EdtsBox::read_box(reader, header.size)?);
                }
                BoxType::MetaBox => {
                    meta.replace(MetaBox::read_box(reader, header.size)?);
                }
                BoxType::MdiaBox => {
                    mdia.replace(MdiaBox::read_box(reader, header.size)?);
                }
                _ => {
                    skip_box(reader, header.size)?;
                }
            }

            current = reader.stream_position()?;
        }

        let Some(tkhd) = tkhd else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "tkhd not found"));
        };
        let Some(mdia) = mdia else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "mdia not found"));
        };

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            tkhd,
            edts,
            meta,
            mdia,
        })
    }
}
