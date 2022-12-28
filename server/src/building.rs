use crate::wonder::Wonder;

pub struct Building {
    pub influencer: usize,
    pub building_type: BuildingType,
}

pub enum BuildingType {
    Academy,
    Market,
    Obelisk,
    Apothecary,
    Fortress,
    Port,
    Temple,
    Wonder(Wonder),
}
