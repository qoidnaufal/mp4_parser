use crate::{meta::MetaBox, mvex::MvexBox, mvhd::MvhdBox, trak::TrakBox};

pub struct MoovBox {
    pub mvhd: MvhdBox,
    pub meta: Option<MetaBox>,
    pub mvex: Option<MvexBox>,
    pub traks: Option<TrakBox>,
    pub udta: Option<UdtaBox>,
}
