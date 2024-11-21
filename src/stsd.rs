use std::{
    fmt::Write,
    io::{self, Read, Seek},
};

use crate::{
    av01::Av01Box,
    avc1::Avc1Box,
    box_start,
    hevc::{HevcBox, HevcDecoderConfigurationRecord},
    mp4a::Mp4aBox,
    read_box_header_ext, skip_bytes_to,
    tx3g::Tx3gBox,
    vp08::Vp08Box,
    vp09::Vp09Box,
    BigEndian, BoxHeader, BoxType, FourCC, Mp4Box, ReadBox, TrackKind, HEADER_EXT_SIZE,
    HEADER_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StsdBoxContent {
    Av01(Av01Box),
    Avc1(Avc1Box),
    Hvc1(HevcBox),
    Hev1(HevcBox),
    Vp08(Vp08Box),
    Vp09(Vp09Box),
    Mp4a(Mp4aBox),
    Tx3g(Tx3gBox),
    Unknown(FourCC),
}

impl Default for StsdBoxContent {
    fn default() -> Self {
        Self::Unknown(FourCC::default())
    }
}

impl StsdBoxContent {
    pub fn bit_depth(&self) -> Option<u8> {
        match self {
            Self::Av01(bx) => Some(bx.av1c.bit_depth),
            Self::Avc1(_) => None, // TODO: figure out bit_depth
            Self::Hvc1(_) => None, // TODO: figure out bit_depth
            Self::Hev1(_) => None, // TODO: figure out bit_depth
            Self::Vp08(bx) => Some(bx.vpcc.bit_depth),
            Self::Vp09(bx) => Some(bx.vpcc.bit_depth),
            Self::Mp4a(_) | Self::Tx3g(_) | Self::Unknown(_) => None, // Not aplicable
        }
    }

    pub fn codec_string(&self) -> Option<String> {
        match self {
            Self::Av01(av01) => {
                let profile = av01.av1c.profile;
                let level = av01.av1c.level;
                let tier = if av01.av1c.tier == 0 { "M" } else { "H" };
                let bit_depth = av01.av1c.bit_depth;

                Some(format!("av01.{profile}.{level:02}{tier}.{bit_depth:02}"))
            }
            Self::Avc1(avc1) => {
                let profile = avc1.avcc.avc_profile_indication;
                let constraint = avc1.avcc.profile_compatibility;
                let level = avc1.avcc.avc_level_indication;

                Some(format!("avc1.{profile:02X}{constraint:02X}{level:02X}"))
            }
            Self::Hvc1(hevc) => Some(format!("hvc1{}", hevc_codec_details(&hevc.hvcc))),
            Self::Hev1(hevc) => Some(format!("hev1{}", hevc_codec_details(&hevc.hvcc))),
            Self::Vp08(vp08) => {
                let profile = vp08.vpcc.profile;
                let level = vp08.vpcc.level;
                let bit_depth = vp08.vpcc.bit_depth;

                Some(format!("vp08.{profile:02}.{level:02}.{bit_depth:02}"))
            }
            Self::Vp09(vp09) => {
                let profile = vp09.vpcc.profile;
                let level = vp09.vpcc.level;
                let bit_depth = vp09.vpcc.bit_depth;

                Some(format!("vp09.{profile:02}.{level:02}.{bit_depth:02}"))
            }
            Self::Mp4a(_) | Self::Tx3g(_) | Self::Unknown(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StsdBox {
    pub version: u8,
    pub flags: u32,
    pub contents: StsdBoxContent,
}

impl StsdBox {
    pub fn kind(&self) -> Option<TrackKind> {
        match &self.contents {
            StsdBoxContent::Av01(_)
            | StsdBoxContent::Avc1(_)
            | StsdBoxContent::Hvc1(_)
            | StsdBoxContent::Hev1(_)
            | StsdBoxContent::Vp08(_)
            | StsdBoxContent::Vp09(_) => Some(TrackKind::Video),
            StsdBoxContent::Mp4a(_) => Some(TrackKind::Audio),
            StsdBoxContent::Tx3g(_) => Some(TrackKind::Subtitle),
            StsdBoxContent::Unknown(_) => None,
        }
    }

    fn get_type(&self) -> BoxType {
        BoxType::StsdBox
    }

    fn get_size(&self) -> u64 {
        HEADER_SIZE
            + HEADER_EXT_SIZE
            + 4
            + match &self.contents {
                StsdBoxContent::Av01(contents) => contents.box_size(),
                StsdBoxContent::Avc1(contents) => contents.box_size(),
                StsdBoxContent::Hvc1(contents) | StsdBoxContent::Hev1(contents) => {
                    contents.box_size()
                }
                StsdBoxContent::Vp08(contents) => contents.box_size(),
                StsdBoxContent::Vp09(contents) => contents.box_size(),
                StsdBoxContent::Mp4a(contents) => contents.box_size(),
                StsdBoxContent::Tx3g(contents) => contents.box_size(),
                StsdBoxContent::Unknown(_) => 0,
            }
    }
}

impl Mp4Box for StsdBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for StsdBox {
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = box_start(reader)?;
        let (version, flags) = read_box_header_ext(reader)?;

        let _ = BigEndian::read_u32(reader)?; // XXX entry_count

        let header = BoxHeader::read(reader)?;

        if header.size > size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "stsd box contains a box with a larger size than itself",
            ));
        }

        let contents = match header.name {
            BoxType::Av01Box => StsdBoxContent::Av01(Av01Box::read_box(reader, header.size)?),
            //
            // According to MPEG-4 part 15, sections 5.4.2.1.2 and 5.4.4
            // -- or the whole 5.4 section in general --
            // the Avc1Box and Avc3Box are identical,
            // but the Avc3Box is used in some cases
            //
            BoxType::Avc1Box => StsdBoxContent::Avc1(Avc1Box::read_box(reader, header.size)?),
            BoxType::Hvc1Box => StsdBoxContent::Hvc1(HevcBox::read_box(reader, header.size)?),
            BoxType::Hev1Box => StsdBoxContent::Hev1(HevcBox::read_box(reader, header.size)?),
            BoxType::Vp08Box => StsdBoxContent::Vp08(Vp08Box::read_box(reader, header.size)?),
            BoxType::Vp09Box => StsdBoxContent::Vp09(Vp09Box::read_box(reader, header.size)?),
            BoxType::Mp4aBox => StsdBoxContent::Mp4a(Mp4aBox::read_box(reader, header.size)?),
            BoxType::Tx3gBox => StsdBoxContent::Tx3g(Tx3gBox::read_box(reader, header.size)?),
            _ => StsdBoxContent::Unknown(header.name.into()),
        };

        skip_bytes_to(reader, start + size)?;

        Ok(Self {
            version,
            flags,
            contents,
        })
    }
}

fn hevc_codec_details(hvcc: &HevcDecoderConfigurationRecord) -> String {
    let mut codec = String::new();
    match hvcc.general_profile_space {
        1 => codec.push_str(".A"),
        2 => codec.push_str(".B"),
        3 => codec.push_str(".C"),
        _ => {}
    }
    write!(&mut codec, ".{}", hvcc.general_profile_idc).ok();

    let mut val = hvcc.general_profile_compatibility_flags;
    let mut reversed = 0;

    for i in 0..32 {
        reversed |= val & 1;
        if i == 31 {
            break;
        }
        reversed <<= 1;
        val >>= 1;
    }
    write!(&mut codec, ".{reversed:X}").ok();

    if hvcc.general_tier_flag {
        codec.push_str(".H")
    } else {
        codec.push_str(".L")
    }
    write!(&mut codec, "{}", hvcc.general_level_idc).ok();

    let mut constraint = [0u8; 6];
    constraint.copy_from_slice(&hvcc.general_constraint_indicator_flag.to_be_bytes()[2..]);

    let mut has_byte = false;
    let mut i: isize = 5;

    while i >= 0 {
        let v = constraint[i as usize];
        if v > 0 || has_byte {
            write!(&mut codec, ".{v:00X}").ok();
            has_byte = true;
        }
        i -= 1;
    }

    codec
}
