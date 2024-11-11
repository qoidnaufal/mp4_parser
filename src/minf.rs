use std::io::{self, Read, Seek};

use crate::{
    box_start, dinf::DinfBox, skip_box, skip_bytes_to, smhd::SmhdBox, stbl::StblBox, vmhd::VmhdBox,
    BoxHeader, BoxType, Mp4Box, ReadBox, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MinfBox {
    pub vmhd: Option<VmhdBox>,
    pub smhd: Option<SmhdBox>,
    pub dinf: DinfBox,
    pub stbl: StblBox,
}

impl MinfBox {
    fn get_type(&self) -> BoxType {
        BoxType::MinfBox
    }

    fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE;
        if let Some(ref vmhd) = self.vmhd {
            size += vmhd.box_size()
        }
        if let Some(ref smhd) = self.smhd {
            size += smhd.box_size()
        }
        size += self.dinf.box_size();
        size += self.stbl.box_size();

        size
    }
}

impl Mp4Box for MinfBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for MinfBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;

        let mut vmhd = None;
        let mut smhd = None;
        let mut dinf = None;
        let mut stbl = None;

        let mut current = reader.stream_position()?;
        let end = start + size;

        while current < end {
            let header = BoxHeader::read(reader)?;

            if header.size > size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "minf box contains a box with a larger size than itself",
                ));
            }

            match header.name {
                BoxType::VmhdBox => {
                    vmhd.replace(VmhdBox::read_box(reader, header.size)?);
                }
                BoxType::SmhdBox => {
                    smhd.replace(SmhdBox::read_box(reader, header.size)?);
                }
                BoxType::DinfBox => {
                    dinf.replace(DinfBox::read_box(reader, header.size)?);
                }
                BoxType::StblBox => {
                    stbl.replace(StblBox::read_box(reader, header.size)?);
                }
                _ => {
                    skip_box(reader, header.size)?;
                }
            }

            current = reader.stream_position()?;
        }

        let Some(dinf) = dinf else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "dinf not found"));
        };
        let Some(stbl) = stbl else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "stbl not found"));
        };

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            vmhd,
            smhd,
            dinf,
            stbl,
        })
    }
}
