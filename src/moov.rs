use std::io::{self, Read, Seek};

use crate::{
    box_start, meta::MetaBox, mvex::MvexBox, mvhd::MvhdBox, skip_box, skip_bytes_to, trak::TrakBox,
    udta::UdtaBox, BoxHeader, BoxType, Mp4Box, ReadBox, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MoovBox {
    pub mvhd: MvhdBox,
    pub meta: Option<MetaBox>,
    pub mvex: Option<MvexBox>,
    pub traks: Vec<TrakBox>,
    pub udta: Option<UdtaBox>,
}

impl MoovBox {
    fn get_type(&self) -> BoxType {
        BoxType::MoovBox
    }

    fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + self.mvhd.box_size();

        for trak in &self.traks {
            size += trak.box_size();
        }

        if let Some(meta) = &self.meta {
            size += meta.box_size();
        }
        if let Some(udta) = &self.udta {
            size += udta.box_size();
        }

        size
    }
}

impl Mp4Box for MoovBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for MoovBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;

        let mut mvhd = None;
        let mut meta = None;
        let mut udta = None;
        let mut mvex = None;
        let mut traks = Vec::new();

        let mut current = reader.stream_position()?;
        let end = start + size;

        while current < end {
            let header = BoxHeader::read(reader)?;

            if header.size > size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "moov box contains a box with a larger size than itsel",
                ));
            }

            match header.name {
                BoxType::MvhdBox => {
                    mvhd.replace(MvhdBox::read_box(reader, header.size)?);
                }
                BoxType::MetaBox => {
                    meta.replace(MetaBox::read_box(reader, header.size)?);
                }
                BoxType::MvexBox => {
                    mvex.replace(MvexBox::read_box(reader, header.size)?);
                }
                BoxType::TrakBox => {
                    let trak = TrakBox::read_box(reader, header.size)?;
                    traks.push(trak);
                }
                BoxType::UdtaBox => {
                    udta.replace(UdtaBox::read_box(reader, header.size)?);
                }
                _ => {
                    skip_box(reader, header.size)?;
                }
            }

            current = reader.stream_position()?;
        }

        let Some(mvhd) = mvhd else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "mvhd not found"));
        };

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            mvhd,
            meta,
            mvex,
            traks,
            udta,
        })
    }
}
