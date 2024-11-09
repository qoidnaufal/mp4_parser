use crate::{dinf::DinfBox, smhd::SmhdBox, stbl::StblBox, vmhd::VmhdBox};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MinfBox {
    pub vmhd: Option<VmhdBox>,
    pub smhd: Option<SmhdBox>,
    pub dinf: DinfBox,
    pub stbl: StblBox,
}
