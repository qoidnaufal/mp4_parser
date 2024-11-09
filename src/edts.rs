use std::io::{self, Read, Seek};

use crate::{
    box_start, elst::ElstBox, skip_bytes_to, BoxHeader, BoxType, Mp4Box, ReadBox, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EdtsBox {
    pub elst: Option<ElstBox>,
}

impl EdtsBox {
    fn get_type(&self) -> BoxType {
        BoxType::EdtsBox
    }

    fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE;
        if let Some(ref elst_box) = self.elst {
            size += elst_box.box_size();
        }
        size
    }
}

impl Mp4Box for EdtsBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for EdtsBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let mut edts = Self::default();
        let header = BoxHeader::read(reader)?;

        if header.size > size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "edts box contains a box with a larger size than itself",
            ));
        }

        if header.name == BoxType::ElstBox {
            let elst = ElstBox::read_box(reader, header.size)?;
            edts.elst.replace(elst);
        }

        skip_bytes_to(reader, start + size)?;

        Ok(edts)
    }
}
