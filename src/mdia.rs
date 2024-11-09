use crate::{hdlr::HdlrBox, mdhd::MdhdBox, minf::MinfBox};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MdiaBox {
    pub mdhd: MdhdBox,
    pub hdlr: HdlrBox,
    pub minf: MinfBox,
}
