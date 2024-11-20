use crate::{tfdt::TfdtBox, tfhd::TfhdBox, trun::TrunBox};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrafBox {
    pub tfhd: TfhdBox,
    pub tftd: Option<TfdtBox>,
    pub truns: Vec<TrunBox>,
}
