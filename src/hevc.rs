use std::io::{self, Read, Seek};

use crate::{
    box_start, skip_bytes, skip_bytes_to, BigEndian, BoxHeader, BoxType, FixedPointU16, Mp4Box,
    RawBox, ReadBox, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HevcBox {
    pub data_reference_index: u16,
    pub width: u16,
    pub height: u16,
    pub horizresolution: FixedPointU16,
    pub vertresolution: FixedPointU16,
    pub frame_count: u16,
    pub depth: u16, // This is usually 24, even for HDR with bit depth=10
    pub hvcc: RawBox<HevcDecoderConfigurationRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HevcDecoderConfigurationRecord {
    pub configuration_version: u8,
    pub general_profile_space: u8,
    pub general_tier_flag: bool,
    pub general_profile_idc: u8,
    pub general_profile_compatibility_flags: u32,
    pub general_constraint_indicator_flag: u64,
    pub general_level_idc: u8,
    pub min_spatial_segmentation_idc: u16,
    pub parallelism_type: u8,
    pub chroma_format_idc: u8,
    pub bit_depth_luma_minus8: u8,
    pub bit_depth_chroma_minus8: u8,
    pub avg_frame_rate: u16,
    pub constant_frame_rate: u8,
    pub num_temporal_layers: u8,
    pub temporal_id_nested: bool,
    pub length_size_minus_one: u8,
    pub arrays: Vec<HvcCArray>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HvcCArray {
    pub completeness: bool,
    pub nal_unit_type: u8,
    pub nalus: Vec<HvcCArrayNalu>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HvcCArrayNalu {
    pub size: u16,
    pub data: Vec<u8>,
}

impl Default for HevcBox {
    fn default() -> Self {
        Self {
            data_reference_index: 0,
            width: 0,
            height: 0,
            horizresolution: FixedPointU16::new(0x48),
            vertresolution: FixedPointU16::new(0x48),
            frame_count: 1,
            depth: 0x0018,
            hvcc: RawBox::default(),
        }
    }
}

impl HevcBox {
    fn get_type(&self) -> BoxType {
        BoxType::Hvc1Box
    }

    fn get_size(&self) -> u64 {
        HEADER_SIZE + 8 + 70 + self.hvcc.box_size()
    }
}

impl Mp4Box for HevcBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for HevcBox {
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

        let header = BoxHeader::read(reader)?;

        if header.size > size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "hvc1 box contains a box with a larger size than itself",
            ));
        }

        if header.name == BoxType::HvcCBox {
            let hvcc = RawBox::<HevcDecoderConfigurationRecord>::read_box(reader, header.size)?;

            skip_bytes_to(reader, start + size)?;

            Ok(Self {
                data_reference_index,
                width,
                height,
                horizresolution,
                vertresolution,
                frame_count,
                depth,
                hvcc,
            })
        } else {
            Err(io::Error::new(io::ErrorKind::InvalidData, "hvcc not found"))
        }
    }
}

impl HevcDecoderConfigurationRecord {
    pub fn new() -> Self {
        Self {
            configuration_version: 1,
            ..Default::default()
        }
    }
}

impl Mp4Box for HevcDecoderConfigurationRecord {
    fn box_type(&self) -> BoxType {
        BoxType::HvcCBox
    }

    fn box_size(&self) -> u64 {
        HEADER_SIZE
            + 23
            + self
                .arrays
                .iter()
                .map(|a| 3 + a.nalus.iter().map(|x| 2 + x.data.len() as u64).sum::<u64>())
                .sum::<u64>()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for HevcDecoderConfigurationRecord {
    fn read_box(reader: &mut R, _: u64) -> io::Result<Self> {
        let configuration_version = BigEndian::read_u8(reader)?;
        let params = BigEndian::read_u8(reader)?;
        let general_profile_space = params >> 6;
        let general_tier_flag = ((params & 0b00100000) >> 5) > 0;
        let general_profile_idc = params & 0b00011111;

        let general_profile_compatibility_flags = BigEndian::read_u32(reader)?;
        let general_constraint_indicator_flag = BigEndian::read_u48(reader)?;
        let general_level_idc = BigEndian::read_u8(reader)?;
        let min_spatial_segmentation_idc = BigEndian::read_u16(reader)? & 0x0FFF;
        let parallelism_type = BigEndian::read_u8(reader)? & 0b11;
        let chroma_format_idc = BigEndian::read_u8(reader)? & 0b11;
        let bit_depth_luma_minus8 = BigEndian::read_u8(reader)? & 0b111;
        let bit_depth_chroma_minus8 = BigEndian::read_u8(reader)? & 0b111;
        let avg_frame_rate = BigEndian::read_u16(reader)?;

        let params = BigEndian::read_u8(reader)?;
        let constant_frame_rate = params & 0b11000000 >> 6;
        let num_temporal_layers = params & 0b00111000 >> 3;
        let temporal_id_nested = (params & 0b00000100 >> 2) > 0;
        let length_size_minus_one = params & 0b000011;

        let num_of_arrays = BigEndian::read_u8(reader)?;
        let mut arrays = Vec::with_capacity(num_of_arrays as _);

        for _ in 0..num_of_arrays {
            let params = BigEndian::read_u8(reader)?;
            let num_nalus = BigEndian::read_u16(reader)?;
            let mut nalus: Vec<HvcCArrayNalu> = Vec::with_capacity(num_nalus as _);

            for _ in 0..num_nalus {
                let size = BigEndian::read_u16(reader)?;
                let mut data = vec![0u8; size as _];

                reader.read_exact(&mut data)?;

                nalus.push(HvcCArrayNalu { size, data })
            }

            arrays.push(HvcCArray {
                completeness: (params & 0b10000000) > 0,
                nal_unit_type: params & 0b11111,
                nalus,
            });
        }

        Ok(Self {
            configuration_version,
            general_profile_space,
            general_tier_flag,
            general_profile_idc,
            general_profile_compatibility_flags,
            general_constraint_indicator_flag,
            general_level_idc,
            min_spatial_segmentation_idc,
            parallelism_type,
            chroma_format_idc,
            bit_depth_luma_minus8,
            bit_depth_chroma_minus8,
            avg_frame_rate,
            constant_frame_rate,
            num_temporal_layers,
            temporal_id_nested,
            length_size_minus_one,
            arrays,
        })
    }
}
