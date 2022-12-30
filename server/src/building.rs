use crate::wonder::Wonder;

pub enum Building {
    Academy,
    Market,
    Obelisk,
    Apothecary,
    Fortress,
    Port,
    Temple,
    Wonder(Wonder),
}
