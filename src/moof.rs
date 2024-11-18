use crate::{mfhd::MfhdBox, traf::TrafBox};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MoofBox {
    pub start: u64,
    pub mfhd: MfhdBox,
    pub trafs: Vec<TrafBox>,
}
