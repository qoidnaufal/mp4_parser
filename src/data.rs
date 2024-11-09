use std::io::{Read, Seek};

use crate::{box_start, BigEndian, BoxType, DataType, Mp4Box, ReadBox, HEADER_SIZE};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DataBox {
    pub data: Vec<u8>,
    pub data_type: DataType,
}

impl DataBox {
    fn get_type(&self) -> BoxType {
        BoxType::DataBox
    }

    fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE;
        size += 4; // data_type
        size += 4; // reserved
        size += self.data.len() as u64;
        size
    }
}

impl Mp4Box for DataBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for DataBox {
    fn read_box(reader: &mut R, size: u64) -> std::io::Result<Self> {
        let start = box_start(reader)?;
        let num = BigEndian::read_u32(reader)?;
        let data_type = DataType::try_from(num)?;

        let _reserved = BigEndian::read_u32(reader)?;

        let current = reader.stream_position()?;
        let mut data = vec![0u8; (start + size - current) as usize];
        reader.read_exact(&mut data)?;

        Ok(Self { data, data_type })
    }
}
