use std::io::{self, Read, Seek};

use crate::{
    box_start, hdlr::HdlrBox, ilst::IlstBox, skip_box, BigEndian, BoxHeader, BoxType, FourCC,
    Mp4Box, ReadBox, HEADER_EXT_SIZE, HEADER_SIZE,
};

const MDIR: FourCC = FourCC { value: *b"mdir" };

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetaBox {
    Mdir {
        ilst: Option<IlstBox>,
    },
    Unknown {
        hdlr: HdlrBox,
        data: Vec<(BoxType, Vec<u8>)>,
    },
}

impl MetaBox {
    fn get_type(&self) -> BoxType {
        BoxType::MetaBox
    }

    fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + HEADER_EXT_SIZE;
        match self {
            Self::Mdir { ilst } => {
                size += HdlrBox::default().box_size();
                if let Some(ilstbox) = ilst {
                    size += ilstbox.box_size();
                }
            }
            Self::Unknown { hdlr, data } => {
                size += hdlr.box_size()
                    + data
                        .iter()
                        .map(|(_, d)| d.len() as u64 + HEADER_SIZE)
                        .sum::<u64>();
            }
        }
        size
    }
}

impl Mp4Box for MetaBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl Default for MetaBox {
    fn default() -> Self {
        Self::Unknown {
            hdlr: Default::default(),
            data: Default::default(),
        }
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for MetaBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let extended_header = BigEndian::read_u32(reader)?;

        if extended_header != 0 {
            let num = BigEndian::read_u32(reader)?;
            let possible_hdlr = BoxType::from(num);

            if possible_hdlr == BoxType::HdlrBox {
                reader.seek(io::SeekFrom::Current(-8))?;
            } else {
                let v = (extended_header >> 24) as u8;
                let msg = format!("MetaBox with version {} is unsupported", v);
                return Err(io::Error::new(io::ErrorKind::Unsupported, msg.as_str()));
            }
        }

        let mut current = reader.stream_position()?;
        let end = start + size;
        let content_start = current;

        let mut hdlr: Option<HdlrBox> = None;

        while current < end {
            let header = BoxHeader::read(reader)?;
            match header.name {
                BoxType::HdlrBox => {
                    hdlr.replace(HdlrBox::read_box(reader, header.size)?);
                }
                _ => {
                    skip_box(reader, header.size)?;
                }
            }

            current = reader.stream_position()?;
        }

        let Some(hdlr) = hdlr else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "HdlrBox is not found",
            ));
        };

        reader.seek(io::SeekFrom::Start(content_start))?;
        current = reader.stream_position()?;

        let mut ilst: Option<IlstBox> = None;

        if hdlr.handler_type == MDIR {
            while current < end {
                let header = BoxHeader::read(reader)?;
                match header.name {
                    BoxType::IlstBox => {
                        ilst.replace(IlstBox::read_box(reader, header.size)?);
                    }
                    _ => {
                        skip_box(reader, header.size)?;
                    }
                }

                current = reader.stream_position()?;
            }

            Ok(Self::Mdir { ilst })
        } else {
            let mut data = Vec::new();

            while current < end {
                let header = BoxHeader::read(reader)?;
                if header.name == BoxType::HdlrBox {
                    skip_box(reader, header.size)?;
                } else {
                    let mut box_data = vec![0; (header.size - HEADER_SIZE) as _];
                    reader.read_exact(&mut box_data)?;
                    data.push((header.name, box_data));
                }

                current = reader.stream_position()?;
            }

            Ok(Self::Unknown { hdlr, data })
        }
    }
}
