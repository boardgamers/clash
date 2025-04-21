use std::{mem, time::Duration, vec};

use tokio::runtime::Runtime;

use crate::advance::Advance;
use crate::{
    ai::{self, ACTION_SCORE_WEIGHTING},
    barbarians,
    game::Game,
    map::Terrain,
    movement::{self, MoveUnits, MovementAction},
    payment::PaymentOptions,
    pirates,
    position::Position,
    unit::UnitType,
    utils::{self, Rng},
};

#[derive(Clone)]
pub struct ActiveMissions {
    pub missions: Vec<Mission>,
    pub player_index: usize,
    pub idle_units: Vec<u32>,
}

impl ActiveMissions {
    pub fn new(
        game: &Game,
        player_index: usize,
        rng: &mut Rng,
        monte_carlo_evaluation: Option<(Duration, f64)>,
    ) -> Self {
        let mut missions = Self {
            missions: Vec::new(),
            player_index,
            idle_units: Vec::new(),
        };
        missions
            .idle_units
            .extend(game.players[player_index].units.iter().map(|unit| unit.id));
        missions.allocate_units(game, rng, monte_carlo_evaluation);
        missions
    }

    pub fn update(
        &mut self,
        game: &Game,
        rng: &mut Rng,
        monte_carlo_evaluation: Option<(Duration, f64)>,
    ) {
        let cloned_missions = self.clone();
        for mission in &mut self.missions {
            mission.update(game, &cloned_missions);
        }
        for mission in &mut self.missions {
            if mission.is_complete(game) {
                let new_idle_units = mem::take(&mut mission.units);
                self.idle_units.extend(new_idle_units);
            }
        }
        self.missions.retain(|mission| !mission.is_complete(game));
        self.allocate_units(game, rng, monte_carlo_evaluation);
    }

    fn allocate_units(
        &mut self,
        game: &Game,
        rng: &mut Rng,
        monte_carlo_evaluation: Option<(Duration, f64)>,
    ) {
        let mut new_missions = Vec::new();
        for unit in &self.idle_units {
            let missions = self.missions_for_unit(game, *unit);
            if missions.is_empty() {
                continue;
            }
            if missions.len() == 1 {
                new_missions.push(
                    missions
                        .into_iter()
                        .next()
                        .expect("there should be 1 mission"),
                );
                continue;
            }
            let mission = if let Some((evaluation_time, difficulty)) = monte_carlo_evaluation {
                let runtime = Runtime::new().expect("failed to create runtime");
                runtime.block_on(self.decide_mission(
                    game,
                    difficulty,
                    missions,
                    rng,
                    evaluation_time,
                ))
            } else {
                let weightings = missions
                    .iter()
                    .map(|mission| mission.priority(game, self).powf(ACTION_SCORE_WEIGHTING))
                    .collect::<Vec<f64>>();
                let mission_index = utils::weighted_random_selection(&weightings, rng);
                missions
                    .into_iter()
                    .nth(mission_index)
                    .expect("index out of bounds")
            };
            new_missions.push(mission);
        }
        new_missions
            .chunk_by_mut(|a, b| {
                a.mission_type == b.mission_type
                    && a.target == b.target
                    && a.current_location == b.current_location
            })
            .map(|missions| {
                let mut units = Vec::new();
                for mission in missions.iter_mut() {
                    units.append(&mut mission.units);
                }
                Mission::new(
                    units,
                    missions[0].target,
                    missions[0].mission_type.clone(),
                    game,
                    self.player_index,
                )
            })
            .for_each(|mission| {
                self.missions.push(mission);
            });
    }

    fn missions_for_unit(&self, game: &Game, unit: u32) -> Vec<Mission> {
        let unit = game.players[self.player_index].get_unit(unit);
        let explore_target = decide_scouting_position(game, self.player_index, unit.position, self);
        if matches!(unit.unit_type, UnitType::Settler) {
            return self.get_settler_missions(game, unit, explore_target);
        }
        if matches!(unit.unit_type, UnitType::Ship) {
            return self.get_ship_missions(game, unit.id);
        }
        self.get_combat_unit_missions(game, explore_target, unit.id)
    }

    #[allow(clippy::too_many_lines)]
    fn get_combat_unit_missions(
        &self,
        game: &Game,
        explore_target: Option<Position>,
        unit: u32,
    ) -> Vec<Mission> {
        let defend_city_targets = game.players[self.player_index]
            .cities
            .iter()
            .map(|city| city.position)
            .filter(|city_position| {
                city_threat_score(game, self.player_index, *city_position) > 0.1
            })
            .collect::<Vec<Position>>();
        let capture_player_city_targets = game
            .players
            .iter()
            .filter(|player| player.is_human() && player.index != self.player_index)
            .flat_map(|player| {
                player
                    .cities
                    .iter()
                    .map(|city| (player.index, city.position))
            })
            .collect::<Vec<(usize, Position)>>();
        let capture_barbarian_camp_targets = barbarians::get_barbarians_player(game)
            .cities
            .iter()
            .map(|city| city.position)
            .collect::<Vec<Position>>();
        let fight_player_forces_targets = game
            .players
            .iter()
            .filter(|player| player.is_human() && player.index != self.player_index)
            .flat_map(|player| {
                player
                    .units
                    .iter()
                    .map(|unit| unit.position)
                    .map(|position| {
                        (
                            player.index,
                            position,
                            player
                                .get_units(position)
                                .iter()
                                .map(|unit| unit.id)
                                .collect::<Vec<u32>>(),
                        )
                    })
            })
            .collect::<Vec<(usize, Position, Vec<u32>)>>();
        let fight_barbarians_targets = barbarians::get_barbarians_player(game)
            .units
            .iter()
            .map(|unit| unit.position)
            .map(|position| {
                (
                    position,
                    barbarians::get_barbarians_player(game)
                        .get_units(position)
                        .iter()
                        .map(|unit| unit.id)
                        .collect::<Vec<u32>>(),
                )
            })
            .collect::<Vec<(Position, Vec<u32>)>>();
        let mut missions = Vec::new();
        missions.extend(explore_target.map(|target| {
            Mission::new(
                vec![unit],
                target,
                MissionType::Explore,
                game,
                self.player_index,
            )
        }));
        missions.extend(defend_city_targets.into_iter().map(|target| {
            Mission::new(
                vec![unit],
                target,
                MissionType::DefendCity,
                game,
                self.player_index,
            )
        }));
        missions.extend(
            capture_player_city_targets
                .into_iter()
                .map(|(player_index, target)| {
                    Mission::new(
                        vec![unit],
                        target,
                        MissionType::CapturePlayerCity { player_index },
                        game,
                        self.player_index,
                    )
                }),
        );
        missions.extend(capture_barbarian_camp_targets.into_iter().map(|target| {
            Mission::new(
                vec![unit],
                target,
                MissionType::CaptureBarbarianCamp,
                game,
                self.player_index,
            )
        }));
        missions.extend(fight_player_forces_targets.into_iter().map(
            |(player_index, target, units)| {
                Mission::new(
                    vec![unit],
                    target,
                    MissionType::FightPlayerForces {
                        player_index,
                        units,
                    },
                    game,
                    self.player_index,
                )
            },
        ));
        missions.extend(fight_barbarians_targets.into_iter().map(|(target, units)| {
            Mission::new(
                vec![unit],
                target,
                MissionType::FightBarbarians { units },
                game,
                self.player_index,
            )
        }));
        missions
    }

    fn get_settler_missions(
        &self,
        game: &Game,
        unit: &crate::unit::Unit,
        explore_target: Option<Position>,
    ) -> Vec<Mission> {
        let found_city_target =
            decide_settling_position(game, self.player_index, unit.position, self);
        let mut missions = vec![Mission::new(
            vec![unit.id],
            found_city_target,
            MissionType::FoundCity,
            game,
            self.player_index,
        )];
        missions.extend(explore_target.map(|target| {
            Mission::new(
                vec![unit.id],
                target,
                MissionType::Explore,
                game,
                self.player_index,
            )
        }));
        missions
    }

    fn get_ship_missions(&self, game: &Game, unit: u32) -> Vec<Mission> {
        let fight_pirates_targets = pirates::get_pirates_player(game)
            .units
            .iter()
            .map(|unit| unit.position)
            .map(|position| {
                (
                    position,
                    pirates::get_pirates_player(game)
                        .get_units(position)
                        .iter()
                        .map(|unit| unit.id)
                        .collect::<Vec<u32>>(),
                )
            })
            .collect::<Vec<(Position, Vec<u32>)>>();
        let transport_targets = Vec::<Position>::new();
        //todo
        let cartography_targets = Vec::<Position>::new();
        //todo
        let mut missions = Vec::new();
        missions.extend(fight_pirates_targets.into_iter().map(|(target, units)| {
            Mission::new(
                vec![unit],
                target,
                MissionType::FightPirates { units },
                game,
                self.player_index,
            )
        }));
        missions.extend(transport_targets.into_iter().map(|target| {
            Mission::new(
                vec![unit],
                target,
                MissionType::Transport,
                game,
                self.player_index,
            )
        }));
        missions.extend(cartography_targets.into_iter().map(|target| {
            Mission::new(
                vec![unit],
                target,
                MissionType::Cartography,
                game,
                self.player_index,
            )
        }));
        missions
    }

    async fn decide_mission(
        &self,
        game: &Game,
        difficulty: f64,
        missions: Vec<Mission>,
        rng: &mut Rng,
        evaluation_time: Duration,
    ) -> Mission {
        let time_per_mission = evaluation_time / missions.len() as u32;
        let mut scores = Vec::new();
        let players_active_missions = self.get_players_active_missions(game, rng);
        let difficulty_factor = ai::difficulty_factor(difficulty);
        for mission in &missions {
            let mut game = game.clone();
            game.supports_undo = false;
            let mut players_active_missions = players_active_missions.clone();
            players_active_missions[self.player_index]
                .missions
                .push(mission.clone());
            let score = ai::get_average_score(
                game,
                self.player_index,
                rng,
                &time_per_mission,
                &players_active_missions,
            )
            .await
            .powf(difficulty_factor);
            scores.push(score);
        }
        let chosen_mission = if difficulty >= 1.0 - f64::EPSILON {
            scores
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).expect("floating point error"))
                .expect("there are no possible actions")
                .0
        } else {
            utils::weighted_random_selection(&scores, rng)
        };
        missions
            .into_iter()
            .nth(chosen_mission)
            .expect("index out of bounds")
    }

    fn missions_of_type(&self, mission_type: &MissionType) -> usize {
        self.missions
            .iter()
            .filter(|mission| mission.mission_type == *mission_type)
            .count()
    }

    fn targets_of_type(&self, mission_type: &MissionType) -> Vec<Position> {
        self.missions
            .iter()
            .filter(|mission| mission.mission_type == *mission_type)
            .map(|mission| mission.target)
            .collect()
    }

    pub fn get_players_active_missions(&self, game: &Game, rng: &mut Rng) -> Vec<ActiveMissions> {
        let mut players_active_missions = Vec::new();
        for player in &game.players {
            if player.index == self.player_index {
                players_active_missions.push(self.clone());
                continue;
            }
            players_active_missions.push(ActiveMissions::new(game, player.index, rng, None));
        }
        players_active_missions
    }
}

#[derive(Clone)]
pub struct Mission {
    units: Vec<u32>,
    player_index: usize,
    mission_type: MissionType,
    target: Position,
    current_location: Position,
    pub next_action: Option<MovementAction>,
}

impl Mission {
    fn new(
        units: Vec<u32>,
        target: Position,
        mission_type: MissionType,
        game: &Game,
        player_index: usize,
    ) -> Self {
        let current_location = game.players[player_index].get_unit(units[0]).position;
        let mut mission = Self {
            units,
            player_index,
            target,
            mission_type,
            current_location,
            next_action: None,
        };
        mission.next_action = mission.next_movement(game);
        mission
    }

    /// Returns the priority of the mission.
    ///
    /// # Panics
    ///
    /// Panics if the game or the mission is invalid.
    #[must_use]
    #[allow(clippy::only_used_in_recursion)]
    pub fn priority(&self, _game: &Game, _active_missions: &ActiveMissions) -> f64 {
        1.0 //todo
    }

    fn update(&mut self, game: &Game, active_missions: &ActiveMissions) {
        self.units.retain(|unit| {
            game.players[self.player_index]
                .try_get_unit(*unit)
                .is_some()
        });
        if self.units.is_empty() {
            return;
        }
        self.current_location = game.players[self.player_index]
            .get_unit(self.units[0])
            .position;
        self.units.retain(|unit| {
            game.players[self.player_index].get_unit(*unit).position == self.current_location
        });
        self.next_action = self.next_movement(game);
        match &mut self.mission_type {
            MissionType::FoundCity => {
                if game.players.iter().any(|player| {
                    player
                        .cities
                        .iter()
                        .any(|city| city.position.distance(self.target) < 3)
                }) {
                    self.target = decide_settling_position(
                        game,
                        self.player_index,
                        self.current_location,
                        active_missions,
                    );
                }
            }
            MissionType::FightPlayerForces {
                player_index: enemy_player,
                units,
            } => {
                units.retain(|unit| game.players[*enemy_player].try_get_unit(*unit).is_some());
                if !units.is_empty() {
                    self.target = game.players[*enemy_player].get_unit(units[0]).position;
                }
                if units
                    .iter()
                    .any(|unit| game.players[*enemy_player].get_unit(*unit).position != self.target)
                {
                    self.units.clear();
                }
            }
            MissionType::FightBarbarians { units } => {
                units.retain(|unit| {
                    barbarians::get_barbarians_player(game)
                        .try_get_unit(*unit)
                        .is_some()
                });
                if !units.is_empty() {
                    self.target = barbarians::get_barbarians_player(game)
                        .get_unit(units[0])
                        .position;
                }
            }
            MissionType::FightPirates { units } => {
                units.retain(|unit| {
                    pirates::get_pirates_player(game)
                        .try_get_unit(*unit)
                        .is_some()
                });
                if !units.is_empty() {
                    self.target = pirates::get_pirates_player(game)
                        .get_unit(units[0])
                        .position;
                }
            }
            _ => (),
        }
    }

    fn is_complete(&self, game: &Game) -> bool {
        self.current_location == self.target
            || self.units.is_empty()
            || match &self.mission_type {
                MissionType::Explore => !game.map.is_unexplored(self.target),
                MissionType::DefendCity => game.players[self.player_index]
                    .try_get_city(self.target)
                    .is_none(),
                MissionType::CapturePlayerCity {
                    player_index: enemy_player,
                } => game.players[*enemy_player]
                    .try_get_city(self.target)
                    .is_none(), //todo: abort mission if defenses get too strong
                MissionType::CaptureBarbarianCamp => barbarians::get_barbarians_player(game)
                    .try_get_city(self.target)
                    .is_none(),
                MissionType::FightPlayerForces {
                    player_index: _,
                    units,
                }
                | MissionType::FightBarbarians { units }
                | MissionType::FightPirates { units } => units.is_empty(),
                _ => false,
            }
    }

    fn next_movement(&self, game: &Game) -> Option<MovementAction> {
        //todo: settlers and scouts should avoid stronger enemies in their path
        let next_position = self
            .current_location
            .next_position_in_path(&self.target)
            .expect("missions is at it's target location");
        let carrier = self.carrier(game);
        let cost = self.movement_cost(game);
        if !game.players[self.player_index].can_afford(&cost) {
            return None;
        }
        //todo: handle roads
        Some(MovementAction::Move(MoveUnits::new(
            self.units.clone(),
            next_position,
            carrier,
            cost.default_payment(),
        )))
    }

    fn carrier(&self, game: &Game) -> Option<u32> {
        let carrier = game.players[self.player_index]
            .get_unit(self.units[0])
            .carrier_id;
        carrier
    }

    fn movement_cost(&self, game: &Game) -> PaymentOptions {
        movement::move_units_destinations(
            game.player(self.player_index),
            game,
            &self.units,
            self.current_location,
            self.carrier(game),
        )
        .expect("units in mission can't move")
        .into_iter()
        .find(|destination| destination.destination == self.target)
        .expect("mission can't move to target")
        .cost
    }
}

#[derive(Clone, PartialEq)]
enum MissionType {
    Explore,
    DefendCity,
    FoundCity,
    CapturePlayerCity {
        player_index: usize,
    },
    CaptureBarbarianCamp,
    FightPlayerForces {
        player_index: usize,
        units: Vec<u32>,
    },
    FightBarbarians {
        units: Vec<u32>,
    },
    FightPirates {
        units: Vec<u32>,
    },
    Transport,
    Cartography,
}

fn decide_settling_position(
    game: &Game,
    player_index: usize,
    settler_position: Position,
    missions: &ActiveMissions,
) -> Position {
    let mut blocked_positions = game
        .players
        .iter()
        .flat_map(|player| &player.cities)
        .map(|city| city.position)
        .collect::<Vec<Position>>();
    blocked_positions.extend(missions.targets_of_type(&MissionType::FoundCity));

    game.map
        .tiles
        .clone()
        .into_keys()
        .filter(|position| !blocked_positions.contains(position))
        .map(|position| {
            (
                position,
                settling_score(game, player_index, position, missions, &blocked_positions)
                    / settler_position.distance(position) as f64,
            )
        })
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .expect("empty map")
        .0
}

fn settling_score(
    game: &Game,
    player_index: usize,
    city_position: Position,
    missions: &ActiveMissions,
    blocked_positions: &[Position],
) -> f64 {
    let mut score = 0.0;
    let neighbors = city_position
        .neighbors()
        .into_iter()
        .filter(|position| !blocked_positions.contains(position))
        .filter_map(|position| game.map.get(position))
        .collect::<Vec<&Terrain>>();
    if neighbors.contains(&&Terrain::Mountain) {
        score += 1.0;
    }
    if neighbors.contains(&&Terrain::Forest) {
        score += 1.0;
    }
    if neighbors.contains(&&Terrain::Fertile)
        || (neighbors.contains(&&Terrain::Barren)
            && game.players[player_index].has_advance(Advance::Irrigation))
        || neighbors.contains(&&Terrain::Water)
            && game.players[player_index].has_advance(Advance::Fishing)
    {
        score += 1.0;
    }
    if neighbors.contains(&&Terrain::Water) {
        score += 0.25;
        if game.players[player_index].cities.len()
            + missions.missions_of_type(&MissionType::FoundCity)
            == 1
        {
            score += 0.75;
        }
        if game.players[player_index].has_advance(Advance::Fishing) {
            score += 0.25;
        }
    }
    score -= city_threat_score(game, player_index, city_position) * 3.0;
    score
}

fn city_threat_score(game: &Game, player_index: usize, city_position: Position) -> f64 {
    game.players
        .iter()
        .filter(|player| player.index != player_index)
        .flat_map(|player| {
            let mut player_threats = player
                .units
                .iter()
                .map(|unit| unit.position)
                .collect::<Vec<Position>>();
            player_threats.extend(player.cities.iter().map(|city| city.position));
            player_threats
        })
        .fold(0.0, |acc, position| {
            acc + 1.0 / position.distance(city_position).pow(2) as f64
        })
}

fn decide_scouting_position(
    game: &Game,
    player_index: usize,
    scout_position: Position,
    missions: &ActiveMissions,
) -> Option<Position> {
    let scout_targets = missions.targets_of_type(&MissionType::Explore);
    game.map
        .tiles
        .clone()
        .into_keys()
        .filter(|position| game.map.is_unexplored(*position) && !scout_targets.contains(position))
        .map(|position| {
            (
                position,
                scout_score(game, player_index, position, &scout_targets)
                    / scout_position.distance(position) as f64,
            )
        })
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(position, _)| position)
}

fn scout_score(
    game: &Game,
    player_index: usize,
    position: Position,
    scout_targets: &[Position],
) -> f64 {
    let capital_city_position = game.players[player_index].cities[0].position;
    let mut score = 1.0;
    for target in scout_targets {
        let distance = position.distance(*target);
        if distance < 4 {
            score /= 4.0 - distance as f64;
        }
    }
    score /= position.distance(capital_city_position) as f64;
    score
}
