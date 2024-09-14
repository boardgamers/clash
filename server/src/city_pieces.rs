use std::ops::{AddAssign, SubAssign};

use crate::{content::wonders, wonder::Wonder};
use Building::*;

#[derive(Default)]
pub struct CityPieces {
    pub academy: Option<usize>,
    pub market: Option<usize>,
    pub obelisk: Option<usize>,
    pub observatory: Option<usize>,
    pub fortress: Option<usize>,
    pub port: Option<usize>,
    pub temple: Option<usize>,
    pub wonders: Vec<Wonder>,
}

impl CityPieces {
    ///
    ///
    /// # Panics
    ///
    /// Panics if any wonder does not exist
    #[must_use]
    pub fn from_data(data: &CityPiecesData) -> Self {
        Self {
            academy: data.academy,
            market: data.market,
            obelisk: data.obelisk,
            observatory: data.observatory,
            fortress: data.fortress,
            port: data.port,
            temple: data.temple,
            wonders: data
                .wonders
                .iter()
                .map(|wonder| {
                    wonders::get_wonder_by_name(wonder)
                        .expect("city piece data should contain a valid wonder")
                })
                .collect(),
        }
    }

    #[must_use]
    pub fn data(self) -> CityPiecesData {
        CityPiecesData {
            academy: self.academy,
            market: self.market,
            obelisk: self.obelisk,
            observatory: self.observatory,
            fortress: self.fortress,
            port: self.port,
            temple: self.temple,
            wonders: self.wonders.into_iter().map(|wonder| wonder.name).collect(),
        }
    }

    #[must_use]
    pub fn cloned_data(&self) -> CityPiecesData {
        CityPiecesData {
            academy: self.academy,
            market: self.market,
            obelisk: self.obelisk,
            observatory: self.observatory,
            fortress: self.fortress,
            port: self.port,
            temple: self.temple,
            wonders: self
                .wonders
                .iter()
                .map(|wonder| wonder.name.clone())
                .collect(),
        }
    }

    #[must_use]
    pub fn can_add_building(&self, building: &Building) -> bool {
        match building {
            Academy => self.academy.is_none(),
            Market => self.market.is_none(),
            Obelisk => self.obelisk.is_none(),
            Observatory => self.observatory.is_none(),
            Fortress => self.fortress.is_none(),
            Port => self.port.is_none(),
            Temple => self.temple.is_none(),
        }
    }

    pub fn set_building(&mut self, building: &Building, player_index: usize) {
        match building {
            Academy => self.academy = Some(player_index),
            Market => self.market = Some(player_index),
            Obelisk => self.obelisk = Some(player_index),
            Observatory => self.observatory = Some(player_index),
            Fortress => self.fortress = Some(player_index),
            Port => self.port = Some(player_index),
            Temple => self.temple = Some(player_index),
        }
    }

    pub fn remove_building(&mut self, building: &Building) {
        match building {
            Academy => self.academy = None,
            Market => self.market = None,
            Obelisk => self.obelisk = None,
            Observatory => self.observatory = None,
            Fortress => self.fortress = None,
            Port => self.port = None,
            Temple => self.temple = None,
        }
    }

    #[must_use]
    pub fn amount(&self) -> usize {
        self.buildings(None).len() + self.wonders.len()
    }

    #[must_use]
    pub fn building_owner(&self, building: &Building) -> Option<usize> {
        match *building {
            Academy => self.academy,
            Market => self.market,
            Obelisk => self.obelisk,
            Observatory => self.observatory,
            Fortress => self.fortress,
            Port => self.port,
            Temple => self.temple,
        }
    }

    #[must_use]
    pub fn building_owners(&self) -> Vec<(Building, Option<usize>)> {
        vec![
            (Academy, self.academy),
            (Market, self.market),
            (Obelisk, self.obelisk),
            (Observatory, self.observatory),
            (Fortress, self.fortress),
            (Port, self.port),
            (Temple, self.temple),
        ]
    }

    #[must_use]
    pub fn buildings(&self, owned_by: Option<usize>) -> Vec<Building> {
        self.building_owners()
            .into_iter()
            .filter_map(|(building, owner)| {
                owner
                    .filter(|owner| match owned_by {
                        Some(want_owner) => owner == &want_owner,
                        None => true,
                    })
                    .map(|_| building)
            })
            .collect()
    }
}

#[derive(Clone, PartialEq)]
pub struct CityPiecesData {
    academy: Option<usize>,
    market: Option<usize>,
    obelisk: Option<usize>,
    observatory: Option<usize>,
    fortress: Option<usize>,
    port: Option<usize>,
    temple: Option<usize>,
    wonders: Vec<String>,
}

#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub enum Building {
    Academy,
    Market,
    Obelisk,
    Observatory,
    Fortress,
    Port,
    Temple,
}

impl Building {
    #[must_use]
    pub fn required_advance(&self) -> String {
        String::from(match self {
            Self::Academy => "Writing",
            Self::Market => "Bartering",
            Self::Obelisk => "Arts",
            Self::Observatory => "Math",
            Self::Fortress => "Tactics",
            Self::Port => "Fishing",
            Self::Temple => "Myths",
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AvailableCityPieces {
    pub academies: u8,
    pub markets: u8,
    pub obelisks: u8,
    pub observatories: u8,
    pub fortresses: u8,
    pub ports: u8,
    pub temples: u8,
}

impl AvailableCityPieces {
    #[must_use]
    pub fn new(
        academies: u8,
        markets: u8,
        obelisks: u8,
        observatories: u8,
        fortresses: u8,
        ports: u8,
        temples: u8,
    ) -> Self {
        Self {
            academies,
            markets,
            obelisks,
            observatories,
            fortresses,
            ports,
            temples,
        }
    }

    #[must_use]
    pub fn can_build(&self, building: &Building) -> bool {
        match *building {
            Academy => self.academies > 0,
            Market => self.markets > 0,
            Obelisk => self.obelisks > 0,
            Observatory => self.observatories > 0,
            Fortress => self.fortresses > 0,
            Port => self.ports > 0,
            Temple => self.temples > 0,
        }
    }
}

impl AddAssign<&Building> for AvailableCityPieces {
    fn add_assign(&mut self, rhs: &Building) {
        match *rhs {
            Academy => self.academies += 1,
            Market => self.markets += 1,
            Obelisk => self.obelisks += 1,
            Observatory => self.observatories += 1,
            Fortress => self.fortresses += 1,
            Port => self.ports += 1,
            Temple => self.temples += 1,
        };
    }
}

impl SubAssign<&Building> for AvailableCityPieces {
    fn sub_assign(&mut self, rhs: &Building) {
        match *rhs {
            Academy => self.academies -= 1,
            Market => self.markets -= 1,
            Obelisk => self.obelisks -= 1,
            Observatory => self.observatories -= 1,
            Fortress => self.fortresses -= 1,
            Port => self.ports -= 1,
            Temple => self.temples -= 1,
        };
    }
}
