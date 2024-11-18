#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrafBox {
    pub tfhd: TfhdBox,
    pub tftd: Option<TftdBox>,
    pub truns: Vec<TrunBox>,
}
