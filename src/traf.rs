use std::io::{self, Read, Seek};

use crate::{
    box_start, skip_box, skip_bytes_to, tfdt::TfdtBox, tfhd::TfhdBox, trun::TrunBox, BoxHeader,
    BoxType, Mp4Box, ReadBox, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrafBox {
    pub tfhd: TfhdBox,
    pub tfdt: Option<TfdtBox>,
    pub truns: Vec<TrunBox>,
}

impl TrafBox {
    fn get_type(&self) -> BoxType {
        BoxType::TrafBox
    }

    fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE;
        size += self.tfhd.box_size();

        if let Some(ref tfdt) = self.tfdt {
            size += tfdt.box_size()
        }

        for trun in &self.truns {
            size += trun.box_size()
        }

        size
    }
}

impl Mp4Box for TrafBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for TrafBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;

        let mut tfhd = None;
        let mut tfdt = None;
        let mut truns = Vec::new();

        let mut current = reader.stream_position()?;
        let end = start + size;

        while current < end {
            let header = BoxHeader::read(reader)?;

            if header.size > size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "traf box contains a box with a larger size than itself",
                ));
            }

            match header.name {
                BoxType::TfhdBox => {
                    tfhd.replace(TfhdBox::read_box(reader, header.size)?);
                }
                BoxType::TfdtBox => {
                    tfdt.replace(TfdtBox::read_box(reader, header.size)?);
                }
                BoxType::TrunBox => {
                    truns.push(TrunBox::read_box(reader, header.size)?);
                }
                _ => {
                    skip_box(reader, header.size)?;
                }
            }

            current = reader.stream_position()?;
        }

        let Some(tfhd) = tfhd else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "tfhd box not found",
            ));
        };

        skip_bytes_to(reader, start + size)?;

        Ok(Self { tfhd, tfdt, truns })
    }
}
