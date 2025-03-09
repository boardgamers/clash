use crate::ability_initializer::AbilityInitializerSetup;
use crate::city_pieces::Building;
use crate::content::custom_phase_actions::ResourceRewardRequest;
use crate::game::Game;
use crate::game_api::current_player;
use crate::incident::{Incident, IncidentBaseEffect};
use crate::payment::PaymentOptions;
use crate::player_events::IncidentTarget;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::unit::get_current_move;
use std::env::current_exe;
use std::fmt::format;

pub(crate) fn trades() -> Vec<Incident> {
    vec![
        scientific_trade(),
        flourishing_trade(),
        era_of_stability(),
        reformation(),
    ]
}

fn scientific_trade() -> Incident {
    Incident::builder(
        45,
        "Scientific Trade",
        "Every player gains an amount of ideas equal to the number of cities that have an Academy or Observatory. You gain at least 2 ideas.",
        IncidentBaseEffect::PiratesSpawnAndRaid,
    )
        .add_simple_incident_listener(
            IncidentTarget::AllPlayers,
            0,
            |game, p, name, i| {
                let player = game.get_player_mut(p);
                let mut ideas = player
                    .cities
                    .iter()
                    .filter(|c| {
                        let b = c.pieces.buildings(None);
                        b.contains(&Building::Academy) || b.contains(&Building::Observatory)
                    })
                    .count();
                if i.active_player == p {
                    ideas = ideas.max(2);
                }

                let pile = ResourcePile::ideas(ideas as u32);
                player.gain_resources(pile.clone());
                game.add_info_log_item(&format!("{name} gained {pile}"));
            }).build()
}

fn flourishing_trade() -> Incident {
    Incident::builder(
        46,
        "Flourishing Trade",
        "Every player gains an amount of gold equal to the number of cities that have a Market or Port (up to a maximum of 3). You gain at least 1 gold.",
        IncidentBaseEffect::PiratesSpawnAndRaid,
    )
        .add_simple_incident_listener(
            IncidentTarget::AllPlayers,
            0,
            |game, p, name, i| {
                let player = game.get_player_mut(p);
                let mut gold = player
                    .cities
                    .iter()
                    .filter(|c| {
                        let b = c.pieces.buildings(None);
                        b.contains(&Building::Market) || b.contains(&Building::Port)
                    })
                    .count();

                gold = gold.min(3);

                if i.active_player == p {
                    gold = gold.max(1);
                }

                let pile = ResourcePile::gold(gold as u32);
                player.gain_resources(pile.clone());
                game.add_info_log_item(&format!("{name} gained {pile}"));
            }).build()
}

fn era_of_stability() -> Incident {
    Incident::builder(
        47,
        "Era of Stability",
        "Every player gains an amount of mood or culture tokens equal to the number of cities that have a Temple or Obelisk (up to a maximum of 3). You gain at least 1 token.",
        IncidentBaseEffect::ExhaustedLand,
    )
        .add_incident_resource_request(
            IncidentTarget::AllPlayers,
            0,
            |game, p, i| {
                let player = game.get_player(p);
                let mut tokens = player
                    .cities
                    .iter()
                    .filter(|c| {
                        let b = c.pieces.buildings(None);
                        b.contains(&Building::Temple) || b.contains(&Building::Obelisk)
                    })
                    .count();

                tokens = tokens.min(3);

                if i.active_player == p {
                    tokens = tokens.max(1);
                }
                Some(ResourceRewardRequest::new(PaymentOptions::tokens(tokens as u32),
                                                "Select token to gain".to_string()))
            },
            |game, s| {
                vec![format!("{} gained {}", s.player_name, s.choice)]
            },
        ).build()
}

fn reformation() -> Incident {
    Incident::builder(
        48,
        "Reformation",
        "Select another player to replace one of your Temples with one of their Temples (this can't be prevented). If you don't own any Temples, select a player that has a Temple: they execute this event instead.",
        IncidentBaseEffect::BarbariansSpawn,
    )
        // select a player to execute the incident
        .add_incident_player_request(
            "Select a player to execute the event",
            |p, game, i| {
                if has_temple(game, i.active_player) {
                    // active player executes the event
                    false
                } else {
                    p.index != i.active_player && has_temple(game, p.index)
                }
            },
            3,
            |game, s| {
                game.current_event_mut().selected_player = Some(s.choice);
                game.add_info_log_item(
                    &format!("{} selected {} to execute the event", 
                             s.player_name, game.player_name(s.choice)));
            },
        )
        // select a player to gain a temple
        .add_incident_player_request(
            "Select a player to gain a Temple",
            |p, game, i| {
                if let Some(s) = game.current_event().selected_player {
                    if p.index != s && can_gain_temple(game, p.index) {
                        return true;
                    }
                }
                false
            },
            2,
            |game, p| {
                game.current_event_mut().selected_player = Some(p.choice);
            },
        )
        .build()
}

fn has_temple(game: &Game, player: usize) -> bool {
    game.get_player(player)
        .cities
        .iter()
        .any(|c| c.pieces.buildings(Some(player)).contains(&Building::Temple))
}

fn can_gain_temple(game: &Game, player: usize) -> bool {
    game.get_player(player)
        .is_building_available(Building::Temple, game)
}
