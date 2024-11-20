use std::io::{self, Read, Seek};

use crate::{
    box_start, mfhd::MfhdBox, skip_box, skip_bytes_to, traf::TrafBox, BoxHeader, BoxType, Mp4Box,
    ReadBox, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MoofBox {
    pub start: u64,
    pub mfhd: MfhdBox,
    pub trafs: Vec<TrafBox>,
}

impl MoofBox {
    fn get_type(&self) -> BoxType {
        BoxType::MoofBox
    }

    fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + self.mfhd.box_size();
        for traf in &self.trafs {
            size += traf.box_size()
        }

        size
    }
}

impl Mp4Box for MoofBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for MoofBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;

        let mut mfhd = None;
        let mut trafs = Vec::new();

        let mut current = reader.stream_position()?;
        let end = start + size;

        while current < end {
            let header = BoxHeader::read(reader)?;

            if header.size > size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "moof box contains a box with a larger size than itself",
                ));
            }

            match header.name {
                BoxType::MfhdBox => {
                    mfhd.replace(MfhdBox::read_box(reader, header.size)?);
                }
                BoxType::TrafBox => {
                    let traf = TrafBox::read_box(reader, header.size)?;
                    trafs.push(traf);
                }
                _ => {
                    skip_box(reader, header.size)?;
                }
            }
            current = reader.stream_position()?;
        }

        let Some(mfhd) = mfhd else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "mfhd box not found",
            ));
        };

        skip_bytes_to(reader, start + size)?;

        Ok(Self { start, mfhd, trafs })
    }
}
