use serde::{Deserialize, Serialize};

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
    pub fn can_add_building(&self, building: Building) -> bool {
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

    pub fn set_building(&mut self, building: Building, player_index: usize) {
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

    pub fn remove_building(&mut self, building: Building) {
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
    pub fn building_owner(&self, building: Building) -> Option<usize> {
        match building {
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

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct CityPiecesData {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    academy: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    market: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    obelisk: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    observatory: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    fortress: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    port: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    temple: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    wonders: Vec<String>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Clone, Debug, Copy)]
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
    /// Returns the json of this [`Building`].
    ///
    /// # Panics
    ///
    /// Panics if `serde_json` produces invalid json
    #[must_use]
    pub fn json(&self) -> String {
        serde_json::to_string(&self).expect("city piece data should be valid json")
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if invalid json is given
    #[must_use]
    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).expect("API call should receive valid city piece data json")
    }

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
