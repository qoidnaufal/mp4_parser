use std::io::{self, Read, Seek};

use crate::{
    box_start, mehd::MehdBox, skip_box, skip_bytes_to, trex::TrexBox, BoxHeader, BoxType, Mp4Box,
    ReadBox, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MvexBox {
    pub mehd: Option<MehdBox>,
    pub trexs: Vec<TrexBox>,
}

impl MvexBox {
    fn get_type(&self) -> BoxType {
        BoxType::MvexBox
    }

    fn get_size(&self) -> u64 {
        HEADER_SIZE
            + self.mehd.as_ref().map_or(0, |x| x.box_size())
            + self.trexs.iter().map(|x| x.box_size()).sum::<u64>()
    }
}

impl Mp4Box for MvexBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for MvexBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;

        let mut mehd: Option<MehdBox> = None;
        let mut trexs: Vec<TrexBox> = Vec::new();

        let mut current = reader.stream_position()?;
        let end = start + size;

        while current < end {
            let header = BoxHeader::read(reader)?;
            if header.size > size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "mvex box contains a box with a larger size that itself",
                ));
            }

            match header.name {
                BoxType::MehdBox => {
                    mehd.replace(MehdBox::read_box(reader, header.size)?);
                }
                BoxType::TrexBox => {
                    trexs.push(TrexBox::read_box(reader, header.size)?);
                }
                _ => {
                    skip_box(reader, header.size)?;
                }
            }

            current = reader.stream_position()?;
        }

        if trexs.is_empty() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "TrexBox not found"));
        }

        skip_bytes_to(reader, start + size)?;

        Ok(Self { mehd, trexs })
    }
}
