use std::io::{self, Read, Seek};

use crate::{
    box_start, co64::Co64Box, ctts::CttsBox, skip_box, skip_bytes_to, stco::StcoBox, stsc::StscBox,
    stsd::StsdBox, stss::StssBox, stsz::StszBox, stts::SttsBox, BoxHeader, BoxType, Mp4Box,
    ReadBox, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StblBox {
    pub stsd: StsdBox,
    pub stts: SttsBox,
    pub ctts: Option<CttsBox>,
    pub stss: Option<StssBox>,
    pub stsc: StscBox,
    pub stsz: StszBox,
    pub stco: Option<StcoBox>,
    pub co64: Option<Co64Box>,
}

impl StblBox {
    fn get_type(&self) -> BoxType {
        BoxType::StblBox
    }

    fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE;
        size += self.stsd.box_size();
        size += self.stts.box_size();

        if let Some(ref ctts) = self.ctts {
            size += ctts.box_size()
        }
        if let Some(ref stss) = self.stss {
            size += stss.box_size()
        }

        size += self.stsc.box_size();
        size += self.stsz.box_size();

        if let Some(ref stco) = self.stco {
            size += stco.box_size()
        }
        if let Some(ref co64) = self.co64 {
            size += co64.box_size()
        }

        size
    }
}

impl Mp4Box for StblBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R> ReadBox<&mut R> for StblBox
where
    R: Read + Seek,
{
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;

        let mut stsd = None;
        let mut stts = None;
        let mut ctts = None;
        let mut stss = None;
        let mut stsc = None;
        let mut stsz = None;
        let mut stco = None;
        let mut co64 = None;

        let mut current = reader.stream_position()?;
        let end = start + size;

        while current < end {
            let header = BoxHeader::read(reader)?;

            if header.size > size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "stbl box contains a box with a larger size than itself",
                ));
            }

            match header.name {
                BoxType::StsdBox => {
                    stsd.replace(StsdBox::read_box(reader, header.size)?);
                }
                BoxType::SttsBox => {
                    stts.replace(SttsBox::read_box(reader, header.size)?);
                }
                BoxType::CttsBox => {
                    ctts.replace(CttsBox::read_box(reader, header.size)?);
                }
                BoxType::StssBox => {
                    stss.replace(StssBox::read_box(reader, header.size)?);
                }
                BoxType::StscBox => {
                    stsc.replace(StscBox::read_box(reader, header.size)?);
                }
                BoxType::StszBox => {
                    stsz.replace(StszBox::read_box(reader, header.size)?);
                }
                BoxType::StcoBox => {
                    stco.replace(StcoBox::read_box(reader, header.size)?);
                }
                BoxType::Co64Box => {
                    co64.replace(Co64Box::read_box(reader, header.size)?);
                }
                _ => {
                    skip_box(reader, header.size)?;
                }
            }
            current = reader.stream_position()?;
        }

        let Some(stsd) = stsd else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "stsd not found"));
        };
        let Some(stts) = stts else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "stts not found"));
        };
        let Some(stsc) = stsc else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "stsc not found"));
        };
        let Some(stsz) = stsz else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "stsz not found"));
        };
        if stco.is_none() && co64.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "stco & co64 not found",
            ));
        }

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            stsd,
            stts,
            ctts,
            stss,
            stsc,
            stsz,
            stco,
            co64,
        })
    }
}
