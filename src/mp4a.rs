use std::io::{self, Read, Seek};

use crate::{
    box_start, read_box_header_ext, skip_bytes, skip_bytes_to, AacConfig, BigEndian, BoxHeader,
    BoxType, FixedPointU16, Mp4Box, ReadBox, HEADER_EXT_SIZE, HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mp4aBox {
    pub data_reference_index: u16,
    pub channelcount: u16,
    pub samplesize: u16,
    pub samplerate: FixedPointU16,
    pub esds: Option<EsdsBox>,
}

impl Default for Mp4aBox {
    fn default() -> Self {
        Self {
            data_reference_index: 0,
            channelcount: 2,
            samplesize: 16,
            samplerate: FixedPointU16::new(48000),
            esds: Some(EsdsBox::default()),
        }
    }
}

impl Mp4aBox {
    pub fn new(config: &AacConfig) -> Self {
        Self {
            data_reference_index: 1,
            channelcount: config.chan_conf as u16,
            samplesize: 16,
            samplerate: FixedPointU16::new(config.freq_index.freq() as _),
            esds: Some(EsdsBox::new(config)),
        }
    }

    fn get_type(&self) -> BoxType {
        BoxType::Mp4aBox
    }

    fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + 8 + 20;
        if let Some(ref esds) = self.esds {
            size += esds.box_size();
        }
        size
    }
}

impl Mp4Box for Mp4aBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for Mp4aBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;

        let _ = BigEndian::read_u32(reader)?; // reserved
        let _ = BigEndian::read_u16(reader)?; // reserved

        let data_reference_index = BigEndian::read_u16(reader)?;
        let version = BigEndian::read_u16(reader)?;

        let _ = BigEndian::read_u16(reader)?; // reserved
        let _ = BigEndian::read_u32(reader)?; // reserved

        let channelcount = BigEndian::read_u16(reader)?;
        let samplesize = BigEndian::read_u16(reader)?;

        let _ = BigEndian::read_u32(reader)?; // pre-defined, reserved
        let samplerate = FixedPointU16::new_raw(BigEndian::read_u32(reader)?);

        if version == 1 {
            // skip QTFF
            let _ = BigEndian::read_u64(reader)?;
            let _ = BigEndian::read_u64(reader)?;
        }

        let mut esds: Option<EsdsBox> = None;
        let end = start + size;

        loop {
            let current = reader.stream_position()?;
            if current >= end {
                break;
            }
            let header = BoxHeader::read(reader)?;

            if header.size > size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "mp4a box contains a box with a larger size than itself",
                ));
            }

            if header.name == BoxType::EsdsBox {
                esds.replace(EsdsBox::read_box(reader, header.size)?);
                break;
            } else if header.name == BoxType::WaveBox {
                // Typically contains frma, mp4a, esds, and a terminator atom
            } else {
                let skip_to = current + header.size;
                skip_bytes_to(reader, skip_to)?;
            }
        }

        skip_bytes_to(reader, end)?;

        Ok(Self {
            data_reference_index,
            channelcount,
            samplesize,
            samplerate,
            esds,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EsdsBox {
    pub version: u8,
    pub flags: u32,
    pub es_desc: ESDescriptor,
}

impl EsdsBox {
    fn new(config: &AacConfig) -> Self {
        Self {
            version: 0,
            flags: 0,
            es_desc: ESDescriptor::new(config),
        }
    }
}

impl Mp4Box for EsdsBox {
    fn box_type(&self) -> BoxType {
        BoxType::EsdsBox
    }

    fn box_size(&self) -> u64 {
        HEADER_SIZE
            + HEADER_EXT_SIZE
            + 1
            + size_of_length(ESDescriptor::desc_size()) as u64
            + ESDescriptor::desc_size() as u64
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for EsdsBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let (version, flags) = read_box_header_ext(reader)?;

        let mut es_desc: Option<ESDescriptor> = None;

        let mut current = reader.stream_position()?;
        let end = start + size;

        while current < end {
            let (desc_tag, desc_size) = read_desc(reader)?;
            match desc_tag {
                0x03 => {
                    es_desc.replace(ESDescriptor::read_desc(reader, desc_size)?);
                }
                _ => break,
            }
            current = reader.stream_position()?;
        }

        let Some(es_desc) = es_desc else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ESDescriptor not found",
            ));
        };

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            es_desc,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ESDescriptor {
    pub es_id: u16,
    pub dec_config: DecoderConfigDescriptor,
    pub sl_config: SLConfifDescriptor,
}

impl ESDescriptor {
    fn new(config: &AacConfig) -> Self {
        Self {
            es_id: 1,
            dec_config: DecoderConfigDescriptor::new(config),
            sl_config: SLConfifDescriptor::new(),
        }
    }
}

impl Descriptor for ESDescriptor {
    fn desc_tag() -> u8 {
        0x03
    }

    fn desc_size() -> u32 {
        3 + 1
            + size_of_length(DecoderConfigDescriptor::desc_size())
            + DecoderConfigDescriptor::desc_size()
            + 1
            + size_of_length(SLConfifDescriptor::desc_size())
            + SLConfifDescriptor::desc_size()
    }
}

impl<R: Read + Seek> ReadDesc<&mut R> for ESDescriptor {
    fn read_desc(reader: &mut R, size: u32) -> io::Result<Self> {
        let start = reader.stream_position()?;

        let es_id = BigEndian::read_u16(reader)?;
        let _ = BigEndian::read_u8(reader)?; // XXX flags must be 0

        let mut dec_config: Option<DecoderConfigDescriptor> = None;
        let mut sl_config: Option<SLConfifDescriptor> = None;

        let mut current = reader.stream_position()?;
        let end = start + size as u64;

        while current < end {
            let (desc_tag, desc_size) = read_desc(reader)?;
            match desc_tag {
                0x04 => {
                    dec_config.replace(DecoderConfigDescriptor::read_desc(reader, desc_size)?);
                }
                0x06 => {
                    sl_config.replace(SLConfifDescriptor::read_desc(reader, desc_size)?);
                }
                _ => {
                    skip_bytes(reader, desc_size as _)?;
                }
            }
            current = reader.stream_position()?;
        }

        Ok(Self {
            es_id,
            dec_config: dec_config.unwrap_or_default(),
            sl_config: sl_config.unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DecoderConfigDescriptor {
    pub object_type_indication: u8,
    pub stream_type: u8,
    pub up_stream: u8,
    pub buffer_size_db: u32,
    pub max_bitrate: u32,
    pub avg_bitrate: u32,
    pub dec_specific: DecoderSpecificDescriptor,
}

impl DecoderConfigDescriptor {
    fn new(config: &AacConfig) -> Self {
        Self {
            object_type_indication: 0x40, // XXX AAC
            stream_type: 0x05,            // XXX Audio
            up_stream: 0,
            buffer_size_db: 0,
            max_bitrate: config.bitrate, // XXX
            avg_bitrate: config.bitrate,
            dec_specific: DecoderSpecificDescriptor::new(config),
        }
    }
}

impl Descriptor for DecoderConfigDescriptor {
    fn desc_tag() -> u8 {
        0x04
    }

    fn desc_size() -> u32 {
        13 + 1
            + size_of_length(DecoderSpecificDescriptor::desc_size())
            + DecoderSpecificDescriptor::desc_size()
    }
}

impl<R: Read + Seek> ReadDesc<&mut R> for DecoderConfigDescriptor {
    fn read_desc(reader: &mut R, size: u32) -> io::Result<Self> {
        let start = reader.stream_position()?;

        let object_type_indication = BigEndian::read_u8(reader)?;
        let byte_a = BigEndian::read_u8(reader)?;
        let stream_type = (byte_a & 0xFC) >> 2;
        let up_stream = byte_a & 0x02;
        let buffer_size_db = BigEndian::read_u24(reader)?;
        let max_bitrate = BigEndian::read_u32(reader)?;
        let avg_bitrate = BigEndian::read_u32(reader)?;

        let mut dec_specific: Option<DecoderSpecificDescriptor> = None;

        let mut current = reader.stream_position()?;
        let end = start + size as u64;

        while current < end {
            let (desc_tag, desc_size) = read_desc(reader)?;
            match desc_tag {
                0x05 => {
                    dec_specific.replace(DecoderSpecificDescriptor::read_desc(reader, desc_size)?);
                }
                _ => {
                    skip_bytes(reader, desc_size as _)?;
                }
            }
            current = reader.stream_position()?;
        }

        Ok(Self {
            object_type_indication,
            stream_type,
            up_stream,
            buffer_size_db,
            max_bitrate,
            avg_bitrate,
            dec_specific: dec_specific.unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SLConfifDescriptor {}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DecoderSpecificDescriptor {
    pub profile: u8,
    pub freq_index: u8,
    pub chan_conf: u8,
}

trait Descriptor: Sized {
    fn desc_tag() -> u8;
    fn desc_size() -> u32;
}

trait ReadDesc<T>: Sized {
    fn read_desc(_: T, size: u32) -> io::Result<Self>;
}

fn read_desc<R: Read>(reader: &mut R) -> io::Result<(u8, u32)> {
    let tag = BigEndian::read_u8(reader)?;
    let mut size: u32 = 0;

    for _ in 0..4 {
        let b = BigEndian::read_u8(reader)?;
        size = (size << 7) | (b & 0x7F) as u32;

        if b & 0x80 == 0 {
            break;
        }
    }

    Ok((tag, size))
}

fn size_of_length(size: u32) -> u32 {
    match size {
        0x0..=0x7F => 1,
        0x80..=0x3FFF => 2,
        0x4000..=0x1FFFFF => 3,
        _ => 4,
    }
}

fn get_audio_object_type(byte_a: u8, byte_b: u8) -> u8 {
    let mut profile = byte_a >> 3;
    if profile == 31 {
        profile = 32 + ((byte_a & 7) | (byte_b >> 5));
    }
    profile
}

fn get_chan_conf<R: Read + Seek>(
    reader: &mut R,
    byte_b: u8,
    freq_index: u8,
    extended_profile: bool,
) -> io::Result<u8> {
    let chan_conf: u8;
    if freq_index == 15 {
        let sample_rate = BigEndian::read_u24(reader)?;
        chan_conf = ((sample_rate >> 4) & 0x0F) as u8;
    } else if extended_profile {
        let byte_c = BigEndian::read_u8(reader)?;
        chan_conf = (byte_b & 1) | (byte_c & 0xE0);
    } else {
        chan_conf = (byte_b >> 3) & 0x0F;
    }

    Ok(chan_conf)
}

impl SLConfifDescriptor {
    fn new() -> Self {
        Self {}
    }
}

impl Descriptor for SLConfifDescriptor {
    fn desc_tag() -> u8 {
        0x06
    }

    fn desc_size() -> u32 {
        1
    }
}

impl<R: Read + Seek> ReadDesc<&mut R> for SLConfifDescriptor {
    fn read_desc(reader: &mut R, _: u32) -> io::Result<Self> {
        BigEndian::read_u8(reader)?; // pre-defined

        Ok(Self {})
    }
}

impl DecoderSpecificDescriptor {
    pub fn new(config: &AacConfig) -> Self {
        Self {
            profile: config.profile as _,
            freq_index: config.freq_index as _,
            chan_conf: config.chan_conf as _,
        }
    }
}

impl Descriptor for DecoderSpecificDescriptor {
    fn desc_tag() -> u8 {
        0x05
    }

    fn desc_size() -> u32 {
        2
    }
}

impl<R: Read + Seek> ReadDesc<&mut R> for DecoderSpecificDescriptor {
    fn read_desc(reader: &mut R, _: u32) -> io::Result<Self> {
        let byte_a = BigEndian::read_u8(reader)?;
        let byte_b = BigEndian::read_u8(reader)?;
        let profile = get_audio_object_type(byte_a, byte_b);

        let freq_index: u8;
        let chan_conf: u8;

        if profile > 31 {
            freq_index = (byte_b >> 1) & 0x0F;
            chan_conf = get_chan_conf(reader, byte_b, freq_index, true)?;
        } else {
            freq_index = ((byte_a & 0x07) << 1) + (byte_b >> 7);
            chan_conf = get_chan_conf(reader, byte_b, freq_index, false)?;
        }

        Ok(Self {
            profile,
            freq_index,
            chan_conf,
        })
    }
}
