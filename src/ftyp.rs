use std::io::{self, Read, Seek};

use crate::{box_start, skip_bytes_to, BigEndian, BoxType, FourCC, Mp4Box, ReadBox, HEADER_SIZE};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FtypBox {
    major_brand: FourCC,
    minor_version: u32,
    compatible_brands: Vec<FourCC>,
}

impl FtypBox {
    fn get_type(&self) -> BoxType {
        BoxType::FtypBox
    }

    fn get_size(&self) -> u64 {
        HEADER_SIZE + 8 + (4 * self.compatible_brands.len() as u64)
    }
}

impl Mp4Box for FtypBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for FtypBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;

        if size < 16 || size % 4 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ftyp size too small or not aligned",
            ));
        }

        let brand_count = (size - 16) / 4;
        let major = BigEndian::read_u32(reader)?;
        let minor = BigEndian::read_u32(reader)?;
        let mut compatible_brands = Vec::new();

        for _ in 0..brand_count {
            let b = BigEndian::read_u32(reader)?;
            compatible_brands.push(FourCC::from(b));
        }

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            major_brand: FourCC::from(major),
            minor_version: minor,
            compatible_brands,
        })
    }
}
