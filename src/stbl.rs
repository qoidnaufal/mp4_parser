use crate::{ctts::CttsBox, stsd::StsdBox, stss::StssBox, stts::SttsBox, BoxType};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StblBox {
    pub stsd: StsdBox,
    pub stts: SttsBox,
    pub ctts: Option<CttsBox>,
    pub stss: Option<StssBox>,
    pub stsc: StscBox,
    pub stsz: StszBox,
    pub stco: Option<StcoBox>,
    pub co64: Option<Co64Box>,
}

impl StblBox {
    fn get_type(&self) -> BoxType {
        BoxType::StblBox
    }
}
