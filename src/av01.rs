use std::io::{self, Read, Seek};

use crate::{
    box_start, skip_bytes, skip_bytes_to, BigEndian, BoxHeader, BoxType, FixedPointU16, Mp4Box,
    RawBox, ReadBox, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Av1CBox {
    pub profile: u8,
    pub level: u8,
    pub tier: u8,
    pub bit_depth: u8,
    pub monochrome: bool,
    pub chroma_subsampling_x: u8,
    pub chroma_subsampling_y: u8,
    pub chroma_sample_position: u8,
    pub initial_presentation_delay_present: bool,
    pub initial_presentation_delay_minus_one: u8,
    pub config_obus: Vec<u8>, // Holds the variable-length config0BUs
}

impl Mp4Box for Av1CBox {
    fn box_type(&self) -> BoxType {
        BoxType::Avc1Box
    }

    fn box_size(&self) -> u64 {
        4 + self.config_obus.len() as u64
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for Av1CBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let marker_byte = BigEndian::read_u8(reader)?;

        if marker_byte & 0x80 != 0x80 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "missing av1c marker bit",
            ));
        }

        if marker_byte & 0x7F != 0x01 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "missing av1c marker bit",
            ));
        }

        let profile_byte = BigEndian::read_u8(reader)?;
        let profile = (profile_byte & 0xE0) >> 5;
        let level = profile_byte & 0x1F;
        let flags_byte = BigEndian::read_u8(reader)?;
        let tier = (flags_byte & 0x80) >> 7;
        let bit_depth: u8 = match flags_byte & 0x60 {
            0x60 => 0b1100,
            0x40 => 0x0A,
            _ => 8,
        };
        let monochrome = flags_byte & 0x10 == 0x10;
        let chroma_subsampling_x = (flags_byte & 0x08) >> 3;
        let chroma_subsampling_y = (flags_byte & 0x04) >> 2;
        let chroma_sample_position = flags_byte & 0x03;
        let delay_byte = BigEndian::read_u8(reader)?;
        let initial_presentation_delay_present = (delay_byte & 0x10) == 0x10;
        let initial_presentation_delay_minus_one = if initial_presentation_delay_present {
            delay_byte & 0x0F
        } else {
            0
        };
        let config_obus_size = size.checked_sub(HEADER_SIZE + 4).ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid box size",
        ))?;
        let mut config_obus = vec![0u8; config_obus_size as _];
        reader.read_exact(&mut config_obus)?;

        Ok(Self {
            profile,
            level,
            tier,
            bit_depth,
            monochrome,
            chroma_subsampling_x,
            chroma_subsampling_y,
            chroma_sample_position,
            initial_presentation_delay_present,
            initial_presentation_delay_minus_one,
            config_obus,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Av01Box {
    pub data_reference_index: u16,
    pub width: u16,
    pub height: u16,
    pub horizresolution: FixedPointU16,
    pub vertresolution: FixedPointU16,
    pub frame_count: u16,
    pub depth: u16,
    pub av1c: RawBox<Av1CBox>,
}

impl Av01Box {
    fn get_type(&self) -> BoxType {
        BoxType::Av01Box
    }

    fn get_size(&self) -> u64 {
        HEADER_SIZE + 8 + 70 + self.av1c.box_size()
    }
}

impl Mp4Box for Av01Box {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for Av01Box {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;

        let _ = BigEndian::read_u32(reader)?; // reserved
        let _ = BigEndian::read_u16(reader)?; // reserved
        let data_reference_index = BigEndian::read_u16(reader)?;

        let _ = BigEndian::read_u32(reader)?; // pre-defined, reserved
        let _ = BigEndian::read_u64(reader)?; // pre-defined
        let _ = BigEndian::read_u32(reader)?; // pre-defined

        let width = BigEndian::read_u16(reader)?;
        let height = BigEndian::read_u16(reader)?;
        let horizresolution = FixedPointU16::new_raw(BigEndian::read_u32(reader)?);
        let vertresolution = FixedPointU16::new_raw(BigEndian::read_u32(reader)?);

        let _ = BigEndian::read_u32(reader)?; // reserved
        let frame_count = BigEndian::read_u16(reader)?;
        skip_bytes(reader, 32)?; // compressorname
        let depth = BigEndian::read_u16(reader)?;
        let _ = BigEndian::read_i16(reader)?; // pre-defined

        let header = BoxHeader::read(reader)?;

        if header.size > size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "av01 box contains a box with a larger size than itself",
            ));
        }

        if header.name == BoxType::Av1CBox {
            let av1c = RawBox::<Av1CBox>::read_box(reader, header.size)?;
            skip_bytes_to(reader, start + size)?;

            Ok(Self {
                data_reference_index,
                width,
                height,
                horizresolution,
                vertresolution,
                frame_count,
                depth,
                av1c,
            })
        } else {
            Err(io::Error::new(io::ErrorKind::InvalidData, "av1c not found"))
        }
    }
}
