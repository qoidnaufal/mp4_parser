mod av01;
mod avc1;
mod co64;
mod ctts;
mod data;
mod dinf;
mod edts;
mod elst;
mod emsg;
mod ftyp;
mod hdlr;
mod hevc;
mod ilst;
mod mdhd;
mod mdia;
mod mehd;
mod meta;
mod mfhd;
mod minf;
mod moof;
mod moov;
mod mp4a;
mod mvex;
mod mvhd;
mod smhd;
mod stbl;
mod stco;
mod stsc;
mod stsd;
mod stss;
mod stsz;
mod stts;
mod tfdt;
mod tfhd;
mod tkhd;
mod traf;
mod trak;
mod trex;
mod trun;
mod tx3g;
mod udta;
mod vmhd;
mod vp08;
mod vp09;
mod vpcc;

use std::{
    borrow::Cow,
    collections::BTreeMap,
    io::{self, Read, Seek},
    str::FromStr,
};

use emsg::EmsgBox;
use ftyp::FtypBox;
use moof::MoofBox;
use moov::MoovBox;
use stbl::StblBox;
use tfhd::TfhdBox;
use trak::TrakBox;
use trun::TrunBox;

const HEADER_SIZE: u64 = 0b1000;
const HEADER_EXT_SIZE: u64 = 0b0100;

const DISPLAY_TYPE_VIDEO: &str = "Video";
const DISPLAY_TYPE_AUDIO: &str = "Audio";
const DISPLAY_TYPE_SUBTITLE: &str = "Subtitle";

const HANDLER_TYPE_VIDEO: &str = "vide";
const HANDLER_TYPE_VIDEO_FOURCC: [u8; 4] = *b"vide"; // [b'v', b'i', b'd', b'e'];

const HANDLER_TYPE_AUDIO: &str = "soun";
const HANDLER_TYPE_AUDIO_FOURCC: [u8; 4] = *b"soun";

const HANDLER_TYPE_SUBTITLE: &str = "sbtl";
const HANDLER_TYPE_SUBTITLE_FOURCC: [u8; 4] = *b"sbtl";

pub type TrackId = u32;

macro_rules! boxtype {
    ($( $name:ident => $value:expr ),*) => {
        #[derive(Clone, Copy, PartialEq, Eq)]
        pub enum BoxType {
            $( $name, )*
            UnknownBox(u32),
        }

        impl From<u32> for BoxType {
            fn from(t: u32) -> Self {
                match t {
                    $( $value => BoxType::$name, )*
                    _ => BoxType::UnknownBox(t),
                }
            }
        }

        impl From<BoxType> for u32 {
            fn from(b: BoxType) -> Self {
                match b {
                    $( BoxType::$name => $value, )*
                    BoxType::UnknownBox(t) => t,
                }
            }
        }
    };
}

boxtype! {
    FtypBox => 0x66747970,
    MvhdBox => 0x6d766864,
    MfhdBox => 0x6d666864,
    FreeBox => 0x66726565,
    MdatBox => 0x6d646174,
    MoovBox => 0x6d6f6f76,
    MvexBox => 0x6d766578,
    MehdBox => 0x6d656864,
    TrexBox => 0x74726578,
    EmsgBox => 0x656d7367,
    MoofBox => 0x6d6f6f66,
    TkhdBox => 0x746b6864,
    TfhdBox => 0x74666864,
    TfdtBox => 0x74666474,
    EdtsBox => 0x65647473,
    MdiaBox => 0x6d646961,
    ElstBox => 0x656c7374,
    MdhdBox => 0x6d646864,
    HdlrBox => 0x68646c72,
    MinfBox => 0x6d696e66,
    VmhdBox => 0x766d6864,
    StblBox => 0x7374626c,
    StsdBox => 0x73747364,
    SttsBox => 0x73747473,
    CttsBox => 0x63747473,
    StssBox => 0x73747373,
    StscBox => 0x73747363,
    StszBox => 0x7374737A,
    StcoBox => 0x7374636F,
    Co64Box => 0x636F3634,
    TrakBox => 0x7472616b,
    TrafBox => 0x74726166,
    TrunBox => 0x7472756E,
    UdtaBox => 0x75647461,
    MetaBox => 0x6d657461,
    DinfBox => 0x64696e66,
    DrefBox => 0x64726566,
    UrlBox  => 0x75726C20,
    SmhdBox => 0x736d6864,
    Avc1Box => 0x61766331,
    // Avc3Box => 0x61766333,
    AvcCBox => 0x61766343,
    Av01Box => 0x61763031,
    Av1CBox => 0x61763143,
    Hev1Box => 0x68657631,
    Hvc1Box => 0x68766331,
    HvcCBox => 0x68766343,
    Mp4aBox => 0x6d703461,
    EsdsBox => 0x65736473,
    Tx3gBox => 0x74783367,
    VpccBox => 0x76706343,
    Vp08Box => 0x76703038,
    Vp09Box => 0x76703039,
    DataBox => 0x64617461,
    IlstBox => 0x696c7374,
    NameBox => 0xa96e616d,
    DayBox => 0xa9646179,
    CovrBox => 0x636f7672,
    DescBox => 0x64657363,
    WideBox => 0x77696465,
    WaveBox => 0x77617665
}

impl std::fmt::Debug for BoxType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fourcc = FourCC::from(*self);
        write!(f, "{fourcc}")
    }
}

impl std::fmt::Display for BoxType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fourcc = FourCC::from(*self);
        write!(f, "{fourcc}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioObjectType {
    AacMain = 1,                                       // AAC Main Profile
    AacLowComplexity = 2,                              // AAC Low Complexity
    AacScalableSampleRate = 3,                         // AAC Scalable Sample Rate
    AacLongTermPrediction = 4,                         // AAC Long Term Predictor
    SpectralBandReplication = 5,                       // Spectral band Replication
    AACScalable = 6,                                   // AAC Scalable
    TwinVQ = 7,                                        // Twin VQ
    CodeExcitedLinearPrediction = 8,                   // CELP
    HarmonicVectorExcitationCoding = 9,                // HVXC
    TextToSpeechtInterface = 12,                       // TTSI
    MainSynthetic = 13,                                // Main Synthetic
    WavetableSynthesis = 14,                           // Wavetable Synthesis
    GeneralMIDI = 15,                                  // General MIDI
    AlgorithmicSynthesis = 16,                         // Algorithmic Synthesis
    ErrorResilientAacLowComplexity = 17,               // ER AAC LC
    ErrorResilientAacLongTermPrediction = 19,          // ER AAC LTP
    ErrorResilientAacScalable = 20,                    // ER AAC Scalable
    ErrorResilientAacTwinVQ = 21,                      // ER AAC TwinVQ
    ErrorResilientAacBitSlicedArithmeticCoding = 22,   // ER Bit Sliced Arithmetic Coding
    ErrorResilientAacLowDelay = 23,                    // ER AAC Low Delay
    ErrorResilientCodeExcitedLinearPrediction = 24,    // ER CELP
    ErrorResilientHarmonicVectorExcitationCoding = 25, // ER HVXC
    ErrorResilientHarmonicIndividualLinesNoise = 26,   // ER HILN
    ErrorResilientParametric = 27,                     // ER Parametric
    SinuSoidalCoding = 28,                             // SSC
    ParametricStereo = 29,                             // PS
    MpegSurround = 30,                                 // MPEG Surround
    MpegLayer1 = 32,                                   // MPEG Layer 1
    MpegLayer2 = 33,                                   // MPEG Layer 2
    MpegLayer3 = 34,                                   // MPEG Layer 3
    DirectStreamTransfer = 35,                         // DST Direct Stream Transfer
    AudioLosslessCoding = 36,                          // ALS Audio Lossless Coding
    ScalableLosslessCoding = 37,                       // SLC Scalable Lossless Coding
    ScalableLosslessCodingNoneCore = 38,               // SLC non-core
    ErrorResilientAacEnhancedLowDelay = 39,            // ER AAC ELD
    SymbolicMusicRepresentationSimple = 40,            // SMR Simple
    SymbolicMusicRepresentationMain = 41,              // SMR Main
    UnifiedSpeechAudioCoding = 42,                     // USAC
    SpatialAudioObjectCoding = 43,                     // SAOC
    LowDelayMpegSurround = 44,                         // LD MPEG Surround
    SpatialAudioObjectCodingDialogueEnhancement = 45,  // SAOC-DE
    AudioSync = 46,                                    // Audio Sync
}

impl TryFrom<u8> for AudioObjectType {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::AacMain),
            2 => Ok(Self::AacLowComplexity),
            3 => Ok(Self::AacScalableSampleRate),
            4 => Ok(Self::AacLongTermPrediction),
            5 => Ok(Self::SpectralBandReplication),
            6 => Ok(Self::AACScalable),
            7 => Ok(Self::TwinVQ),
            8 => Ok(Self::CodeExcitedLinearPrediction),
            9 => Ok(Self::HarmonicVectorExcitationCoding),
            12 => Ok(Self::TextToSpeechtInterface),
            13 => Ok(Self::MainSynthetic),
            14 => Ok(Self::WavetableSynthesis),
            15 => Ok(Self::GeneralMIDI),
            16 => Ok(Self::AlgorithmicSynthesis),
            17 => Ok(Self::ErrorResilientAacLowComplexity),
            19 => Ok(Self::ErrorResilientAacLongTermPrediction),
            20 => Ok(Self::ErrorResilientAacScalable),
            21 => Ok(Self::ErrorResilientAacTwinVQ),
            22 => Ok(Self::ErrorResilientAacBitSlicedArithmeticCoding),
            23 => Ok(Self::ErrorResilientAacLowDelay),
            24 => Ok(Self::ErrorResilientCodeExcitedLinearPrediction),
            25 => Ok(Self::ErrorResilientHarmonicVectorExcitationCoding),
            26 => Ok(Self::ErrorResilientHarmonicIndividualLinesNoise),
            27 => Ok(Self::ErrorResilientParametric),
            28 => Ok(Self::SinuSoidalCoding),
            29 => Ok(Self::ParametricStereo),
            30 => Ok(Self::MpegSurround),
            32 => Ok(Self::MpegLayer1),
            33 => Ok(Self::MpegLayer2),
            34 => Ok(Self::MpegLayer3),
            35 => Ok(Self::DirectStreamTransfer),
            36 => Ok(Self::AudioLosslessCoding),
            37 => Ok(Self::ScalableLosslessCoding),
            38 => Ok(Self::ScalableLosslessCodingNoneCore),
            39 => Ok(Self::ErrorResilientAacEnhancedLowDelay),
            40 => Ok(Self::SymbolicMusicRepresentationSimple),
            41 => Ok(Self::SymbolicMusicRepresentationMain),
            42 => Ok(Self::UnifiedSpeechAudioCoding),
            43 => Ok(Self::SpatialAudioObjectCoding),
            44 => Ok(Self::LowDelayMpegSurround),
            45 => Ok(Self::SpatialAudioObjectCodingDialogueEnhancement),
            46 => Ok(Self::AudioSync),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid audio object type",
            )),
        }
    }
}

impl std::fmt::Display for AudioObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_str = match self {
            AudioObjectType::AacMain => "AAC Main",
            AudioObjectType::AacLowComplexity => "LC",
            AudioObjectType::AacScalableSampleRate => "SSR",
            AudioObjectType::AacLongTermPrediction => "LTP",
            AudioObjectType::SpectralBandReplication => "SBR",
            AudioObjectType::AACScalable => "Scalable",
            AudioObjectType::TwinVQ => "TwinVQ",
            AudioObjectType::CodeExcitedLinearPrediction => "CELP",
            AudioObjectType::HarmonicVectorExcitationCoding => "HVXC",
            AudioObjectType::TextToSpeechtInterface => "TTSI",
            AudioObjectType::MainSynthetic => "Main Synthetic",
            AudioObjectType::WavetableSynthesis => "Wavetable Synthesis",
            AudioObjectType::GeneralMIDI => "General MIDI",
            AudioObjectType::AlgorithmicSynthesis => "Algorithmic Synthesis",
            AudioObjectType::ErrorResilientAacLowComplexity => "ER AAC LC",
            AudioObjectType::ErrorResilientAacLongTermPrediction => "ER AAC LTP",
            AudioObjectType::ErrorResilientAacScalable => "ER AAC scalable",
            AudioObjectType::ErrorResilientAacTwinVQ => "ER AAC TwinVQ",
            AudioObjectType::ErrorResilientAacBitSlicedArithmeticCoding => "ER AAC BSAC",
            AudioObjectType::ErrorResilientAacLowDelay => "ER AAC LD",
            AudioObjectType::ErrorResilientCodeExcitedLinearPrediction => "ER CELP",
            AudioObjectType::ErrorResilientHarmonicVectorExcitationCoding => "ER HVXC",
            AudioObjectType::ErrorResilientHarmonicIndividualLinesNoise => "ER HILN",
            AudioObjectType::ErrorResilientParametric => "ER Parametric",
            AudioObjectType::SinuSoidalCoding => "SSC",
            AudioObjectType::ParametricStereo => "Parametric Stereo",
            AudioObjectType::MpegSurround => "MPEG surround",
            AudioObjectType::MpegLayer1 => "MPEG Layer 1",
            AudioObjectType::MpegLayer2 => "MPEG Layer 2",
            AudioObjectType::MpegLayer3 => "MPEG Layer 3",
            AudioObjectType::DirectStreamTransfer => "DST",
            AudioObjectType::AudioLosslessCoding => "ALS",
            AudioObjectType::ScalableLosslessCoding => "SLS",
            AudioObjectType::ScalableLosslessCodingNoneCore => "SLS Non-core",
            AudioObjectType::ErrorResilientAacEnhancedLowDelay => "ER AAC ELD",
            AudioObjectType::SymbolicMusicRepresentationSimple => "SMR Simple",
            AudioObjectType::SymbolicMusicRepresentationMain => "SMR Main",
            AudioObjectType::UnifiedSpeechAudioCoding => "USAC",
            AudioObjectType::SpatialAudioObjectCoding => "SAOC",
            AudioObjectType::LowDelayMpegSurround => "LD MPEG Surround",
            AudioObjectType::SpatialAudioObjectCodingDialogueEnhancement => "SAOC-DE",
            AudioObjectType::AudioSync => "Audio Sync",
        };

        write!(f, "{type_str}")
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SampleFreqIndex {
    Freq96000 = 0x0,
    Freq88200 = 0x1,
    Freq64000 = 0x2,
    Freq48000 = 0x3,
    Freq44100 = 0x4,
    Freq32000 = 0x5,
    Freq24000 = 0x6,
    Freq22050 = 0x7,
    Freq16000 = 0x8,
    Freq12000 = 0x9,
    Freq11025 = 0xa,
    Freq8000 = 0xb,
    Freq7350 = 0xc,
}

impl TryFrom<u8> for SampleFreqIndex {
    type Error = io::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x0 => Ok(Self::Freq96000),
            0x1 => Ok(Self::Freq88200),
            0x2 => Ok(Self::Freq64000),
            0x3 => Ok(Self::Freq48000),
            0x4 => Ok(Self::Freq44100),
            0x5 => Ok(Self::Freq32000),
            0x6 => Ok(Self::Freq24000),
            0x7 => Ok(Self::Freq22050),
            0x8 => Ok(Self::Freq16000),
            0x9 => Ok(Self::Freq12000),
            0xa => Ok(Self::Freq11025),
            0xb => Ok(Self::Freq8000),
            0xc => Ok(Self::Freq7350),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid sampling frequency index",
            )),
        }
    }
}

impl SampleFreqIndex {
    pub fn freq(&self) -> u32 {
        match *self {
            Self::Freq96000 => 96000,
            Self::Freq88200 => 88200,
            Self::Freq64000 => 64000,
            Self::Freq48000 => 48000,
            Self::Freq44100 => 44100,
            Self::Freq32000 => 32000,
            Self::Freq24000 => 24000,
            Self::Freq22050 => 22050,
            Self::Freq16000 => 16000,
            Self::Freq12000 => 12000,
            Self::Freq11025 => 11025,
            Self::Freq8000 => 8000,
            Self::Freq7350 => 7350,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ChannelConfig {
    Mono = 0x1,
    Stereo = 0x2,
    Three = 0x3,
    Four = 0x4,
    Five = 0x5,
    FiveOne = 0x6,
    SevenOne = 0x7,
}

impl TryFrom<u8> for ChannelConfig {
    type Error = io::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x1 => Ok(Self::Mono),
            0x2 => Ok(Self::Stereo),
            0x3 => Ok(Self::Three),
            0x4 => Ok(Self::Four),
            0x5 => Ok(Self::Five),
            0x6 => Ok(Self::FiveOne),
            0x7 => Ok(Self::SevenOne),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid channel configuration",
            )),
        }
    }
}

impl std::fmt::Display for ChannelConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Mono => "mono",
            Self::Stereo => "stereo",
            Self::Three => "three",
            Self::Four => "four",
            Self::Five => "five",
            Self::FiveOne => "five.one",
            Self::SevenOne => "seven.one",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataType {
    Binary = 0x000000,
    Text = 0x000001,
    Image = 0x00000D,
    TempoCpil = 0x000015,
}

impl Default for DataType {
    fn default() -> Self {
        Self::Binary
    }
}

impl TryFrom<u32> for DataType {
    type Error = std::io::Error;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0x000000 => Ok(Self::Binary),
            0x000001 => Ok(Self::Text),
            0x00000D => Ok(Self::Image),
            0x000015 => Ok(Self::TempoCpil),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid data type",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MetadataKey {
    Title,
    Year,
    Poster,
    Summary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackKind {
    Video,
    Audio,
    Subtitle,
}

impl std::fmt::Display for TrackKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Video => DISPLAY_TYPE_VIDEO,
            Self::Audio => DISPLAY_TYPE_AUDIO,
            Self::Subtitle => DISPLAY_TYPE_SUBTITLE,
        };
        write!(f, "{s}")
    }
}

impl TryFrom<&str> for TrackKind {
    type Error = io::Error;
    fn try_from(handler: &str) -> Result<Self, Self::Error> {
        match handler {
            HANDLER_TYPE_VIDEO => Ok(Self::Video),
            HANDLER_TYPE_AUDIO => Ok(Self::Audio),
            HANDLER_TYPE_SUBTITLE => Ok(Self::Subtitle),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported handler type",
            )),
        }
    }
}

impl TryFrom<&FourCC> for TrackKind {
    type Error = io::Error;
    fn try_from(fourcc: &FourCC) -> Result<Self, Self::Error> {
        match fourcc.value {
            HANDLER_TYPE_VIDEO_FOURCC => Ok(Self::Video),
            HANDLER_TYPE_AUDIO_FOURCC => Ok(Self::Audio),
            HANDLER_TYPE_SUBTITLE_FOURCC => Ok(Self::Subtitle),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported handler type",
            )),
        }
    }
}

impl From<TrackKind> for FourCC {
    fn from(track_kind: TrackKind) -> Self {
        match track_kind {
            TrackKind::Video => HANDLER_TYPE_VIDEO_FOURCC.into(),
            TrackKind::Audio => HANDLER_TYPE_AUDIO_FOURCC.into(),
            TrackKind::Subtitle => HANDLER_TYPE_SUBTITLE_FOURCC.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RgbColor {
    pub red: u16,
    pub green: u16,
    pub blue: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RgbaColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AacConfig {
    pub bitrate: u32,
    pub profile: AudioObjectType,
    pub freq_index: SampleFreqIndex,
    pub chan_conf: ChannelConfig,
}

impl Default for AacConfig {
    fn default() -> Self {
        Self {
            bitrate: 0,
            profile: AudioObjectType::AacLowComplexity,
            freq_index: SampleFreqIndex::Freq48000,
            chan_conf: ChannelConfig::Stereo,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Matrix {
    pub a: i32,
    pub b: i32,
    pub u: i32,
    pub c: i32,
    pub d: i32,
    pub v: i32,
    pub x: i32,
    pub y: i32,
    pub w: i32,
}

impl std::fmt::Display for Matrix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:#x} {:#x} {:#x} {:#x} {:#x} {:#x} {:#x} {:#x} {:#x}",
            self.a, self.b, self.u, self.c, self.d, self.v, self.x, self.y, self.w
        )
    }
}

impl Default for Matrix {
    fn default() -> Self {
        Self {
            a: 0x00010000,
            b: 0,
            u: 0,
            c: 0,
            d: 0x00010000,
            v: 0,
            x: 0,
            y: 0,
            w: 0x40000000,
        }
    }
}

impl Matrix {
    fn read_i32<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        let mut arr = [0i32; 9];

        for i in &mut arr {
            let num = BigEndian::read_i32(reader)?;
            *i = num;
        }

        Ok(Self {
            a: arr[0],
            b: arr[1],
            u: arr[2],
            c: arr[3],
            d: arr[4],
            v: arr[5],
            x: arr[6],
            y: arr[7],
            w: arr[8],
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ratio<T> {
    numer: T,
    denom: T,
}

use std::ops::{Add, Div, Mul, Rem, Sub};

impl<T> Ratio<T>
where
    T: Add<Output = T>
        + Sub<Output = T>
        + Mul<Output = T>
        + Div<Output = T>
        + Rem<Output = T>
        + Copy
        + PartialEq
        + Eq
        + PartialOrd
        + Ord,
{
    #[inline]
    const fn new_raw(numer: T, denom: T) -> Self {
        Self { numer, denom }
    }

    #[inline]
    fn to_integer(self) -> T {
        *self.numer() / *self.denom()
    }

    #[inline]
    fn numer(&self) -> &T {
        &self.numer
    }

    #[inline]
    fn denom(&self) -> &T {
        &self.denom
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedPointU8(Ratio<u16>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedPointI8(Ratio<i16>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedPointU16(Ratio<u32>);

impl FixedPointU8 {
    pub fn new(val: u8) -> Self {
        Self(Ratio::new_raw(val as u16 * 0x100, 0x100))
    }

    pub fn new_raw(val: u16) -> Self {
        Self(Ratio::new_raw(val, 0x100))
    }

    pub fn value(&self) -> u8 {
        self.0.to_integer() as u8
    }

    pub fn raw_value(&self) -> u16 {
        *self.0.numer()
    }
}

impl FixedPointI8 {
    pub fn new(val: i8) -> Self {
        Self(Ratio::new_raw(val as i16 * 0x100, 0x100))
    }

    pub fn new_raw(val: i16) -> Self {
        Self(Ratio::new_raw(val, 0x100))
    }

    pub fn value(&self) -> i8 {
        self.0.to_integer() as i8
    }

    pub fn raw_value(&self) -> i16 {
        *self.0.numer()
    }
}

impl FixedPointU16 {
    pub fn new(val: u16) -> Self {
        Self(Ratio::new_raw(val as u32 * 0x10000, 0x10000))
    }

    pub fn new_raw(val: u32) -> Self {
        Self(Ratio::new_raw(val, 0x10000))
    }

    pub fn value(&self) -> u16 {
        self.0.to_integer() as u16
    }

    pub fn raw_value(&self) -> u32 {
        *self.0.numer()
    }
}

#[derive(Debug, Clone, Copy)]
struct BoxHeader {
    name: BoxType,
    size: u64,
}

impl BoxHeader {
    // fn new(name: BoxType, size: u64) -> Self {
    //     Self { name, size }
    // }

    fn read<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 8];
        reader.read_exact(&mut buf)?;

        let mut s = [0u8; 4];
        let mut t = [0u8; 4];
        s.copy_from_slice(&buf[..4]);
        t.copy_from_slice(&buf[4..]);

        let size = u32::from_be_bytes(s);
        let type_ = u32::from_be_bytes(t);

        if size == 1 {
            let largesize = BigEndian::read_u64(reader)?;

            Ok(Self {
                name: BoxType::from(type_),
                size: match largesize {
                    0 => 0,
                    1..=15 => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "64-bit box size too small",
                        ))
                    }
                    16..=u64::MAX => largesize - 8,
                },
            })
        } else {
            Ok(Self {
                name: BoxType::from(type_),
                size: size as u64,
            })
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct FourCC {
    value: [u8; 4],
}

impl FromStr for FourCC {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let [a, b, c, d] = s.as_bytes() {
            Ok(Self {
                value: [*a, *b, *c, *d],
            })
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "expected exactly four bytes in string",
            ))
        }
    }
}

impl From<u32> for FourCC {
    fn from(number: u32) -> Self {
        Self {
            value: number.to_be_bytes(),
        }
    }
}

impl From<FourCC> for u32 {
    fn from(fourcc: FourCC) -> Self {
        (&fourcc).into()
    }
}

impl From<&FourCC> for u32 {
    fn from(fourcc: &FourCC) -> Self {
        Self::from_be_bytes(fourcc.value)
    }
}

impl From<[u8; 4]> for FourCC {
    fn from(value: [u8; 4]) -> Self {
        Self { value }
    }
}

impl From<BoxType> for FourCC {
    fn from(value: BoxType) -> Self {
        let box_num: u32 = value.into();
        Self::from(box_num)
    }
}

impl std::fmt::Display for FourCC {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code: u32 = self.into();
        let display = String::from_utf8_lossy(&self.value[..]);
        write!(f, "{display} / {code:#010X}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RawBox<T> {
    pub contents: T,
    pub raw: Vec<u8>,
}

impl<T> std::ops::Deref for RawBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.contents
    }
}

impl<T> std::ops::DerefMut for RawBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.contents
    }
}

impl<R, T> ReadBox<&mut R> for RawBox<T>
where
    R: Read + Seek,
    T: for<'a> ReadBox<&'a mut R>,
{
    fn read_box(reader: &mut R, size: u64) -> io::Result<Self> {
        let start = reader.stream_position()?;
        let contents = T::read_box(reader, size)?;
        let end = reader.stream_position()?;
        let mut raw = vec![0u8; (end - start) as _];
        reader.seek(io::SeekFrom::Start(start))?;
        reader.read_exact(&mut raw[..])?;

        Ok(Self { contents, raw })
    }
}

pub struct BigEndian;

impl BigEndian {
    pub fn read_i8<R: Read + Seek>(reader: &mut R) -> io::Result<i8> {
        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf)?;
        Ok(i8::from_be_bytes(buf))
    }

    pub fn read_u8<R: Read>(reader: &mut R) -> io::Result<u8> {
        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf)?;
        Ok(u8::from_be_bytes(buf))
    }

    pub fn read_i16<R: Read + Seek>(reader: &mut R) -> io::Result<i16> {
        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf)?;
        Ok(i16::from_be_bytes(buf))
    }

    pub fn read_u16<R: Read>(reader: &mut R) -> io::Result<u16> {
        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    pub fn read_u24<R: Read>(reader: &mut R) -> io::Result<u32> {
        let mut buf = [0u8; 3];
        reader.read_exact(&mut buf)?;
        let num: u32 = buf
            .iter()
            .enumerate()
            .map(|(idx, n)| {
                let p = buf.len() - (idx + 1);
                (*n as u32) << (p * 8)
            })
            .sum();
        Ok(num)
    }

    pub fn read_i32<R: Read>(reader: &mut R) -> io::Result<i32> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        Ok(i32::from_be_bytes(buf))
    }

    pub fn read_u32<R: Read>(reader: &mut R) -> io::Result<u32> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }

    fn read_u48<R: Read + Seek>(reader: &mut R) -> io::Result<u64> {
        let mut buf = [0u8; 6];
        reader.read_exact(&mut buf)?;
        let num: u64 = buf
            .iter()
            .enumerate()
            .map(|(idx, n)| {
                let p = buf.len() - (idx + 1);
                (*n as u64) << (p * 8)
            })
            .sum();
        Ok(num)
    }

    pub fn read_u64<R: Read>(reader: &mut R) -> io::Result<u64> {
        let mut buf = [0u8; 8];
        reader.read_exact(&mut buf)?;
        Ok(u64::from_be_bytes(buf))
    }
}

pub trait Metadata<'a> {
    fn title(&self) -> Option<Cow<'_, str>>;
    fn year(&self) -> Option<u32>;
    fn poster(&self) -> Option<&[u8]>;
    fn summary(&self) -> Option<Cow<'_, str>>;
}

impl<'a, T: Metadata<'a>> Metadata<'a> for &'a T {
    fn title(&self) -> Option<Cow<'_, str>> {
        (**self).title()
    }

    fn year(&self) -> Option<u32> {
        (**self).year()
    }

    fn poster(&self) -> Option<&[u8]> {
        (**self).poster()
    }

    fn summary(&self) -> Option<Cow<'_, str>> {
        (**self).summary()
    }
}

impl<'a, T: Metadata<'a>> Metadata<'a> for Option<T> {
    fn title(&self) -> Option<Cow<'_, str>> {
        self.as_ref().and_then(|t| t.title())
    }

    fn year(&self) -> Option<u32> {
        self.as_ref().and_then(|t| t.year())
    }

    fn poster(&self) -> Option<&[u8]> {
        self.as_ref().and_then(|t| t.poster())
    }

    fn summary(&self) -> Option<Cow<'_, str>> {
        self.as_ref().and_then(|t| t.summary())
    }
}

pub trait Mp4Box: Sized {
    fn box_type(&self) -> BoxType;
    fn box_size(&self) -> u64;
}

pub trait ReadBox<T>: Sized {
    fn read_box(_: T, size: u64) -> io::Result<Self>;
}

fn read_box_header_ext<R: Read>(reader: &mut R) -> io::Result<(u8, u32)> {
    let version = BigEndian::read_u8(reader)?;
    let flag = BigEndian::read_u24(reader)?;

    Ok((version, flag))
}

fn box_start<R: Seek>(seeker: &mut R) -> io::Result<u64> {
    Ok(seeker.stream_position()? - HEADER_SIZE)
}

fn skip_bytes<S: Seek>(seeker: &mut S, size: u64) -> io::Result<()> {
    seeker.seek(io::SeekFrom::Current(size as i64))?;
    Ok(())
}

fn skip_bytes_to<S: Seek>(seeker: &mut S, pos: u64) -> io::Result<()> {
    seeker.seek(io::SeekFrom::Start(pos))?;
    Ok(())
}

fn skip_box<S: Seek>(seeker: &mut S, size: u64) -> io::Result<()> {
    let start = box_start(seeker)?;
    skip_bytes_to(seeker, start + size)?;
    Ok(())
}

#[derive(Default, Clone, Copy)]
pub struct Sample {
    pub id: u32,
    pub is_sync: bool,
    pub size: u64,
    pub offset: u64,
    pub timescale: u64,
    pub decode_timestamp: i64,
    pub composition_timestamp: i64,
    pub duration: u64,
}

impl std::fmt::Debug for Sample {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sample")
            .field("is_sync", &self.is_sync)
            .field("size", &self.size)
            .field("offset", &self.offset)
            .field("decode_timestamp", &self.decode_timestamp)
            .field("composition_timestamp", &self.composition_timestamp)
            .field("duration", &self.duration)
            .finish()
    }
}

impl Sample {
    pub fn byte_range(&self) -> std::ops::Range<usize> {
        self.offset as usize..(self.offset + self.size) as usize
    }
}

pub struct Track {
    first_traf_merged: bool,
    pub width: u16,
    pub height: u16,
    pub track_id: u32,
    pub time_scale: u64,
    pub duration: u64,
    pub kind: Option<TrackKind>,
    pub samples: Vec<Sample>,
}

impl std::fmt::Debug for Track {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Track")
            .field("first_traf_merged", &self.first_traf_merged)
            .field("kind", &self.kind)
            .field("timescale", &self.time_scale)
            .field("duration", &self.duration)
            .finish()
    }
}

impl Track {
    pub fn trak<'a>(&self, mp4: &'a Mp4) -> &'a TrakBox {
        let Some(trak) = mp4
            .moov
            .traks
            .iter()
            .find(|trak| trak.tkhd.track_id == self.track_id)
        else {
            unreachable!("track with id \"{}\" not found", self.track_id);
        };

        trak
    }

    pub fn raw_codec_config(&self, mp4: &Mp4) -> Option<Vec<u8>> {
        let sample_description = &self.trak(mp4).mdia.minf.stbl.stsd;

        match &sample_description.contents {
            stsd::StsdBoxContent::Av01(content) => Some(content.av1c.raw.clone()),
            stsd::StsdBoxContent::Avc1(content) => Some(content.avcc.raw.clone()),
            stsd::StsdBoxContent::Hev1(content) | stsd::StsdBoxContent::Hvc1(content) => {
                Some(content.hvcc.raw.clone())
            }
            stsd::StsdBoxContent::Vp08(content) => Some(content.vpcc.raw.clone()),
            stsd::StsdBoxContent::Vp09(content) => Some(content.vpcc.raw.clone()),
            stsd::StsdBoxContent::Mp4a(_)
            | stsd::StsdBoxContent::Tx3g(_)
            | stsd::StsdBoxContent::Unknown(_) => None,
        }
    }

    pub fn codec_string(&self, mp4: &Mp4) -> Option<String> {
        self.trak(mp4).mdia.minf.stbl.stsd.contents.codec_string()
    }
}

#[derive(Debug)]
pub struct Mp4 {
    pub ftyp: FtypBox,
    pub moov: MoovBox,
    pub moofs: Vec<MoofBox>,
    pub emsgs: Vec<EmsgBox>,
    tracks: BTreeMap<TrackId, Track>,
}

impl Mp4 {
    pub fn read<R: Read + Seek>(mut reader: R, size: u64) -> io::Result<Self> {
        let start = reader.stream_position()?;

        let mut ftyp = None;
        let mut moov = None;
        let mut moofs = Vec::new();
        let mut moof_offsets = Vec::new();
        let mut emsgs = Vec::new();

        let mut current = start;
        while current < size {
            let header = BoxHeader::read(&mut reader)?;

            if header.size > size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "file contains a box with a larger scale than it",
                ));
            }

            if header.size == 0 {
                break;
            }

            match header.name {
                BoxType::FtypBox => {
                    ftyp.replace(FtypBox::read_box(&mut reader, header.size)?);
                }
                BoxType::FreeBox => {
                    skip_box(&mut reader, header.size)?;
                }
                BoxType::MdatBox => {
                    skip_box(&mut reader, header.size)?;
                }
                BoxType::MoovBox => {
                    moov.replace(MoovBox::read_box(&mut reader, header.size)?);
                }
                BoxType::MoofBox => {
                    let moof_offset = reader.stream_position()?;
                    let moof = MoofBox::read_box(&mut reader, header.size)?;
                    moofs.push(moof);
                    moof_offsets.push(moof_offset);
                }
                BoxType::EmsgBox => {
                    let emsg = EmsgBox::read_box(&mut reader, header.size)?;
                    emsgs.push(emsg)
                }
                _ => {
                    skip_box(&mut reader, header.size)?;
                }
            }

            current = reader.stream_position()?;
        }

        let Some(ftyp) = ftyp else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "ftyp box is not found",
            ));
        };

        let Some(moov) = moov else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "moov box is not found",
            ));
        };

        let mut this = Self {
            ftyp,
            moov,
            moofs,
            emsgs,
            tracks: Default::default(),
        };

        let mut tracks = this.build_tracks();
        this.update_sample_list(&mut tracks)?;
        this.tracks = tracks;
        this.update_tracks();

        Ok(this)
    }

    pub fn tracks(&self) -> &BTreeMap<TrackId, Track> {
        &self.tracks
    }

    fn build_tracks(&mut self) -> BTreeMap<TrackId, Track> {
        let mut tracks = BTreeMap::new();

        for trak in &self.moov.traks {
            let mut sample_n = 0usize;
            let mut chunk_index = 1u64;
            let mut chunk_run_index = 0usize;
            let mut last_sample_in_chunk = 0u64;
            let mut offset_in_chunk = 0u64;
            let mut last_chunk_in_run = 0u64;
            let mut last_sample_in_stts_run = -1i64;
            let mut stts_run_index = -1i64;
            let mut last_stss_index = 0;
            let mut last_sample_in_ctts_run = -1i64;
            let mut ctts_run_index = -1i64;
            let mut dts_shift = 0;

            // The smallest presentation timestap observed on this stream
            //
            // This is typically 0, but in the pressence of sample reordering (cused by AVC/HVC b-frames), it may be non-zero.
            // In fact, many formats don't require this to be zero, but video players typically
            // normalize the shown time to start at zero
            let mut min_composition_timestamp = i64::MAX;

            let mut samples = Vec::<Sample>::new();

            fn get_sample_chunk_offset(stbl: &StblBox, chunk_index: u64) -> u64 {
                if let Some(stco) = &stbl.stco {
                    stco.entries[chunk_index as usize - 1] as u64
                } else if let Some(co64) = &stbl.co64 {
                    co64.entries[chunk_index as usize - 1]
                } else {
                    panic!()
                }
            }

            let stbl = &trak.mdia.minf.stbl;
            let stsc = &stbl.stsc;
            let stsz = &stbl.stsz;
            let stts = &stbl.stts;

            while sample_n < stsz.sample_sizes.len() {
                // compute offset
                if sample_n == 0 {
                    chunk_index = 1;
                    chunk_run_index = 0;
                    last_sample_in_chunk = stsc.entries[chunk_run_index].samples_per_chunk as u64;
                    offset_in_chunk = 0;

                    if chunk_run_index + 1 < stsc.entries.len() {
                        last_chunk_in_run =
                            stsc.entries[chunk_run_index + 1].first_chunk as u64 - 1;
                    } else {
                        last_chunk_in_run = u64::MAX;
                    }
                } else if sample_n < last_sample_in_chunk as usize {
                    /* ... */
                } else {
                    chunk_index += 1;
                    offset_in_chunk = 0;
                    if chunk_index > last_chunk_in_run {
                        chunk_run_index += 1;
                        if chunk_run_index + 1 < stsc.entries.len() {
                            last_chunk_in_run =
                                stsc.entries[chunk_run_index + 1].first_chunk as u64 - 1;
                        } else {
                            last_chunk_in_run = u64::MAX;
                        }
                    }

                    last_sample_in_chunk += stsc.entries[chunk_run_index].samples_per_chunk as u64;
                }

                // compute timestamp, duration, is_sync

                if sample_n as i64 > last_sample_in_stts_run {
                    stts_run_index += 1;
                    if last_sample_in_stts_run < 0 {
                        last_sample_in_stts_run = 0;
                    }
                    last_sample_in_stts_run +=
                        stts.entries[stts_run_index as usize].sample_count as i64;
                }

                let timescale = trak.mdia.mdhd.timescale as u64;
                let size = stsz.sample_sizes[sample_n] as u64;
                let offset = get_sample_chunk_offset(stbl, chunk_index) + offset_in_chunk;
                offset_in_chunk += size;

                let decode_timestamp = if sample_n > 0 {
                    samples[sample_n - 1].duration =
                        stts.entries[stts_run_index as usize].sample_delta as u64;
                    samples[sample_n - 1].decode_timestamp + samples[sample_n - 1].duration as i64
                } else {
                    0
                };

                let composition_timestamp = if let Some(ctts) = &stbl.ctts {
                    if sample_n as i64 >= last_sample_in_ctts_run {
                        ctts_run_index += 1;
                        if last_sample_in_ctts_run < 0 {
                            last_sample_in_ctts_run = 0;
                        }
                        last_sample_in_ctts_run +=
                            ctts.entries[ctts_run_index as usize].sample_count as i64;
                    }

                    // dts shift is determined by the smallest negative sample offset
                    let offset = ctts.entries[ctts_run_index as usize].sample_offset as i64;
                    if offset < 0 {
                        dts_shift = dts_shift.max(-offset);
                    }

                    decode_timestamp + offset
                } else {
                    decode_timestamp
                };

                min_composition_timestamp = min_composition_timestamp.min(composition_timestamp);

                let is_sync = if let Some(stss) = &stbl.stss {
                    if last_stss_index < stss.entries.len()
                        && sample_n == stss.entries[last_stss_index] as usize - 1
                    {
                        last_stss_index += 1;
                        true
                    } else {
                        false
                    }
                } else {
                    true
                };

                samples.push(Sample {
                    id: samples.len() as u32,
                    is_sync,
                    size,
                    offset,
                    timescale,
                    decode_timestamp,
                    composition_timestamp,
                    duration: 0, // filled once next sample timestamp is known
                });

                sample_n += 1;
            }

            if let Some(last_sample) = samples.last_mut() {
                last_sample.duration =
                    trak.mdia.mdhd.duration - last_sample.decode_timestamp as u64;
            }

            if dts_shift > 0 {
                for sample in &mut samples {
                    sample.decode_timestamp -= dts_shift;
                }
            }

            // Shift both DTS & CTS by the smallest CTS
            if min_composition_timestamp != 0 {
                for sample in &mut samples {
                    sample.decode_timestamp -= min_composition_timestamp;
                    sample.composition_timestamp -= min_composition_timestamp;
                }
            }

            tracks.insert(
                trak.tkhd.track_id,
                Track {
                    first_traf_merged: false,
                    width: trak.tkhd.width.value(),
                    height: trak.tkhd.height.value(),
                    track_id: trak.tkhd.track_id,
                    time_scale: trak.mdia.mdhd.timescale as u64,
                    duration: trak.mdia.mdhd.duration,
                    kind: trak.mdia.minf.stbl.stsd.kind(),
                    samples,
                },
            );
        }

        tracks
    }

    // In case the input file is fragmented, it will contain one or more `moof` boxes,
    // which must be processed to obtain the full list of samples for each track.
    fn update_sample_list(&mut self, tracks: &mut BTreeMap<TrackId, Track>) -> io::Result<()> {
        let mut last_run_position = 0;

        for moof in &self.moofs {
            for traf in &moof.trafs {
                let track_id = traf.tfhd.track_id;
                let track = tracks.get_mut(&track_id).ok_or(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("trak box with id {} not found", track_id).as_str(),
                ))?;
                let trak = self
                    .moov
                    .traks
                    .iter()
                    .find(|trak| trak.tkhd.track_id == track_id)
                    .ok_or(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("trak box with id {} not found", track_id).as_str(),
                    ))?;
                let trex = if let Some(mvex) = &self.moov.mvex {
                    mvex.trexs
                        .iter()
                        .find(|trex| trex.track_id == track_id)
                        .ok_or(io::Error::new(
                            io::ErrorKind::NotFound,
                            format!("trak box with id {} not found", track_id).as_str(),
                        ))?
                        .clone()
                } else {
                    Default::default()
                };

                let default_sample_duration = traf
                    .tfhd
                    .default_sample_duration
                    .unwrap_or(trex.default_sample_duration);
                let default_sample_size = traf
                    .tfhd
                    .default_sample_size
                    .unwrap_or(trex.default_sample_size);
                let default_sample_flags = traf
                    .tfhd
                    .default_sample_flags
                    .unwrap_or(trex.default_sample_flags);

                for (traf_idx, trun) in traf.truns.iter().enumerate() {
                    for sample_n in 0..trun.sample_count as usize {
                        let mut sample_flags = default_sample_flags;
                        if trun.flags & TrunBox::FLAG_SAMPLE_FLAGS != 0 {
                            sample_flags = trun
                                .sample_flags
                                .get(sample_n)
                                .copied()
                                .unwrap_or(sample_flags);
                        } else if sample_n == 0
                            && (trun.flags & TrunBox::FLAG_FIRST_SAMPLE_FLAGS != 0)
                        {
                            sample_flags = trun.first_sample_flags.unwrap_or(sample_flags);
                        }

                        let mut decode_timestamp = 0;
                        if track.first_traf_merged || sample_n > 0 {
                            let prev = &track.samples[track.samples.len() - 1];
                            decode_timestamp = prev.decode_timestamp + prev.duration as i64;
                        } else {
                            if let Some(tfdt) = &traf.tfdt {
                                decode_timestamp = tfdt.base_media_decode_time as i64;
                            }
                            track.first_traf_merged = true;
                        }

                        let composition_timestamp = if trun.flags & TrunBox::FLAG_SAMPLE_CTS != 0 {
                            decode_timestamp
                                + trun.sample_cts.get(sample_n).copied().unwrap_or(0) as i64
                        } else {
                            decode_timestamp
                        };

                        let duration = trun
                            .sample_duration
                            .get(sample_n)
                            .copied()
                            .unwrap_or(default_sample_duration)
                            as u64;

                        let base_data_offset_present =
                            traf.tfhd.flags & TfhdBox::FLAG_BASE_DATA_OFFSET != 0;
                        let default_base_is_moof =
                            traf.tfhd.flags & TfhdBox::FLAG_DEFAULT_BASE_IS_MOOF != 0;
                        let data_offset_present = trun.flags & TrunBox::FLAG_DATA_OFFSET != 0;
                        let base_data_offset = if !base_data_offset_present {
                            if !default_base_is_moof {
                                if sample_n == 0 {
                                    // the first sample in the track fragment
                                    moof.start // the position of the first byte of the enclosing Movie Fragment Box
                                } else {
                                    last_run_position // the offset of the previous sample
                                }
                            } else {
                                moof.start
                            }
                        } else {
                            traf.tfhd.base_data_offset.unwrap_or(moof.start)
                        };

                        let sample_size =
                            trun.sample_sizes
                                .get(sample_n)
                                .copied()
                                .unwrap_or(default_sample_size) as u64;

                        let sample_offset = if traf_idx == 0 && sample_n == 0 {
                            if data_offset_present {
                                base_data_offset
                                    .saturating_add_signed(trun.data_offset.unwrap_or(0) as i64)
                            } else {
                                base_data_offset
                            }
                        } else {
                            last_run_position
                        };

                        last_run_position = sample_offset + sample_size;

                        track.samples.push(Sample {
                            id: track.samples.len() as u32,
                            is_sync: (sample_flags >> 16) & 0x1 != 0,
                            size: sample_size,
                            offset: sample_offset,
                            timescale: trak.mdia.mdhd.timescale as u64,
                            decode_timestamp,
                            composition_timestamp,
                            duration,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    fn update_tracks(&mut self) {
        for track in self.tracks.values_mut() {
            if track.duration == 0 {
                track.duration = track
                    .samples
                    .last()
                    .map(|v| v.duration.saturating_add_signed(v.composition_timestamp))
                    .unwrap_or_default();
            }
        }
    }
}
