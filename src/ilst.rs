use std::{
    collections::HashMap,
    io::{self, Read, Seek},
};

use crate::{
    box_start, data::DataBox, skip_box, skip_bytes_to, BoxHeader, BoxType, DataType, Metadata,
    MetadataKey, Mp4Box, ReadBox, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IlstItemBox {
    pub data: DataBox,
}

impl IlstItemBox {
    fn get_size(&self) -> u64 {
        HEADER_SIZE + self.data.box_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for IlstItemBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let mut data: Option<DataBox> = None;
        let mut current = reader.stream_position()?;
        let end = start + size;

        while current < end {
            let header = BoxHeader::read(reader)?;
            if header.size > size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "ilst item box contains a box with a larger size than itself",
                ));
            }

            match header.name {
                BoxType::DataBox => {
                    data.replace(DataBox::read_box(reader, header.size)?);
                }
                _ => {
                    skip_box(reader, header.size)?;
                }
            }

            current = reader.stream_position()?;
        }

        let Some(data) = data else {
            return Err(io::Error::new(io::ErrorKind::Other, "Box not found"));
        };

        skip_bytes_to(reader, start + size)?;

        Ok(Self { data })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IlstBox {
    pub items: HashMap<MetadataKey, IlstItemBox>,
}

impl IlstBox {
    fn get_type(&self) -> BoxType {
        BoxType::IlstBox
    }

    fn get_size(&self) -> u64 {
        HEADER_SIZE + self.items.values().map(|item| item.get_size()).sum::<u64>()
    }
}

impl Mp4Box for IlstBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for IlstBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let mut items = HashMap::new();
        let mut current = reader.stream_position()?;
        let end = start + size;

        while current < end {
            let header = BoxHeader::read(reader)?;

            if header.size > size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "ilst box contains a box with a larger size than itself",
                ));
            }

            match header.name {
                BoxType::NameBox => {
                    items.insert(
                        MetadataKey::Title,
                        IlstItemBox::read_box(reader, header.size)?,
                    );
                }
                BoxType::DayBox => {
                    items.insert(
                        MetadataKey::Year,
                        IlstItemBox::read_box(reader, header.size)?,
                    );
                }
                BoxType::CovrBox => {
                    items.insert(
                        MetadataKey::Poster,
                        IlstItemBox::read_box(reader, header.size)?,
                    );
                }
                BoxType::DescBox => {
                    items.insert(
                        MetadataKey::Summary,
                        IlstItemBox::read_box(reader, header.size)?,
                    );
                }
                _ => {
                    skip_box(reader, header.size)?;
                }
            }

            current = reader.stream_position()?;
        }

        skip_bytes_to(reader, start + size)?;

        Ok(Self { items })
    }
}

impl<'a> Metadata<'a> for IlstBox {
    fn title(&self) -> Option<std::borrow::Cow<'_, str>> {
        self.items
            .get(&MetadataKey::Title)
            .map(|t| String::from_utf8_lossy(&t.data.data))
    }

    fn year(&self) -> Option<u32> {
        self.items
            .get(&MetadataKey::Year)
            .and_then(|t| match t.data.data_type {
                DataType::Binary if t.data.data.len() == 4 => {
                    let mut buf = [0u8; 4];
                    t.data
                        .data
                        .iter()
                        .enumerate()
                        .for_each(|(i, n)| buf[i] = *n);
                    let num = u32::from_be_bytes(buf);
                    Some(num)
                }
                DataType::Text => String::from_utf8_lossy(&t.data.data).parse::<u32>().ok(),
                _ => None,
            })
    }

    fn poster(&self) -> Option<&[u8]> {
        self.items
            .get(&MetadataKey::Poster)
            .map(|t| t.data.data.as_slice())
    }

    fn summary(&self) -> Option<std::borrow::Cow<'_, str>> {
        self.items
            .get(&MetadataKey::Summary)
            .map(|t| String::from_utf8_lossy(&t.data.data))
    }
}
