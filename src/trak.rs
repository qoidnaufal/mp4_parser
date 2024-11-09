use crate::{edts::EdtsBox, mdia::MdiaBox, meta::MetaBox, tkhd::TkhdBox};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrakBox {
    pub tkhd: TkhdBox,
    pub edts: Option<EdtsBox>,
    pub meta: Option<MetaBox>,
    pub mdia: MdiaBox,
}
