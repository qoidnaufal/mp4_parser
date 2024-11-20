use crate::{tfdt::TfdtBox, tfhd::TfhdBox};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrafBox {
    pub tfhd: TfhdBox,
    pub tftd: Option<TfdtBox>,
    pub truns: Vec<TrunBox>,
}
