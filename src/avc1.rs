use std::io::{self, Read, Seek};

use crate::{
    box_start, skip_bytes, skip_bytes_to, BigEndian, BoxHeader, BoxType, FixedPointU16, Mp4Box,
    RawBox, ReadBox, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Avc1Box {
    pub data_reference_index: u16,
    pub width: u16,
    pub height: u16,
    pub horizresolution: FixedPointU16,
    pub vertresolution: FixedPointU16,
    pub frame_count: u16,
    pub depth: u16, // This is usually 24, even for HDR with bit_depth=10
    pub avcc: RawBox<AvcCBox>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AvcCBox {
    pub configuration_version: u8,
    pub avc_profile_indication: u8,
    pub profile_compatibility: u8,
    pub avc_level_indication: u8,
    pub length_size_minus_one: u8,
    pub sequence_parameter_sets: Vec<NalUnit>,
    pub picture_parameter_sets: Vec<NalUnit>,
    pub ext: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NalUnit {
    pub bytes: Vec<u8>,
}

impl Default for Avc1Box {
    fn default() -> Self {
        Self {
            data_reference_index: 0,
            width: 0,
            height: 0,
            horizresolution: FixedPointU16::new(0x48),
            vertresolution: FixedPointU16::new(0x48),
            frame_count: 1,
            depth: 0x0018,
            avcc: RawBox::default(),
        }
    }
}

impl Avc1Box {
    fn get_type(&self) -> BoxType {
        BoxType::Avc1Box
    }

    fn get_size(&self) -> u64 {
        HEADER_SIZE + 8 + 70 + self.avcc.box_size()
    }
}

impl Mp4Box for Avc1Box {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for Avc1Box {
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
        skip_bytes(reader, 0x20)?; // compressorname
        let depth = BigEndian::read_u16(reader)?;
        let _ = BigEndian::read_i16(reader)?; // pre-defined

        let end = start + size;

        loop {
            let current = reader.stream_position()?;
            if current >= end {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "avcc not found"));
            }
            let header = BoxHeader::read(reader)?;
            if header.size > size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "avc1 box contains a box with a larger size than itsel",
                ));
            }
            if header.name == BoxType::AvcCBox {
                let avcc = RawBox::<AvcCBox>::read_box(reader, header.size)?;
                skip_bytes_to(reader, start + size)?;

                return Ok(Self {
                    data_reference_index,
                    width,
                    height,
                    horizresolution,
                    vertresolution,
                    frame_count,
                    depth,
                    avcc,
                });
            } else {
                skip_bytes_to(reader, current + header.size)?;
            }
        }
    }
}

impl AvcCBox {
    pub fn new(sps: &[u8], pps: &[u8]) -> Self {
        Self {
            configuration_version: 1,
            avc_profile_indication: sps[1],
            profile_compatibility: sps[2],
            avc_level_indication: sps[3],
            length_size_minus_one: 0xFF, // length_size = 4
            sequence_parameter_sets: vec![NalUnit::from(sps)],
            picture_parameter_sets: vec![NalUnit::from(pps)],
            ext: Vec::new(),
        }
    }
}

impl Mp4Box for AvcCBox {
    fn box_type(&self) -> BoxType {
        BoxType::AvcCBox
    }

    fn box_size(&self) -> u64 {
        let mut size = HEADER_SIZE + 7;
        for sps in &self.sequence_parameter_sets {
            size += sps.size() as u64;
        }
        for pps in &self.picture_parameter_sets {
            size += pps.size() as u64;
        }
        size
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for AvcCBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let content_start = reader.stream_position()?;

        let configuration_version = BigEndian::read_u8(reader)?;
        let avc_profile_indication = BigEndian::read_u8(reader)?;
        let profile_compatibility = BigEndian::read_u8(reader)?;
        let avc_level_indication = BigEndian::read_u8(reader)?;
        let length_size_minus_one = BigEndian::read_u8(reader)? & 0x3;

        let num_of_spss = BigEndian::read_u8(reader)? & 0x1F;
        let mut sequence_parameter_sets = Vec::with_capacity(num_of_spss as _);

        for _ in 0..num_of_spss {
            let nal_unit = NalUnit::read(reader)?;
            sequence_parameter_sets.push(nal_unit)
        }

        let num_of_ppss = BigEndian::read_u8(reader)?;
        let mut picture_parameter_sets = Vec::with_capacity(num_of_ppss as _);

        for _ in 0..num_of_ppss {
            let nal_unit = NalUnit::read(reader)?;
            picture_parameter_sets.push(nal_unit)
        }

        let content_end = reader.stream_position()?;
        let remainder = size - HEADER_SIZE - (content_end - content_start);
        let mut ext = vec![0u8; remainder as _];
        reader.read_exact(&mut ext)?;

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            configuration_version,
            avc_profile_indication,
            profile_compatibility,
            avc_level_indication,
            length_size_minus_one,
            sequence_parameter_sets,
            picture_parameter_sets,
            ext,
        })
    }
}

impl From<&[u8]> for NalUnit {
    fn from(value: &[u8]) -> Self {
        Self {
            bytes: value.to_vec(),
        }
    }
}

impl NalUnit {
    fn size(&self) -> usize {
        2 + self.bytes.len()
    }

    fn read<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        let length = BigEndian::read_u16(reader)? as usize;
        let mut bytes = vec![0u8; length];
        reader.read_exact(&mut bytes)?;

        Ok(Self { bytes })
    }
}
