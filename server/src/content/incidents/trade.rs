use crate::city::City;
use crate::city_pieces::{Building, gain_building};
use crate::content::persistent_events::{PositionRequest, ResourceRewardRequest};
use crate::game::Game;
use crate::incident::{Incident, IncidentBaseEffect, PassedIncident};
use crate::player::Player;
use crate::player_events::IncidentTarget;
use crate::resource_pile::ResourcePile;

pub(crate) fn trade_incidents() -> Vec<Incident> {
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
        "Every player gains an amount of ideas equal to the number of cities \
        that have an Academy or Observatory. You gain at least 2 ideas.",
        IncidentBaseEffect::PiratesSpawnAndRaid,
    )
    .add_simple_incident_listener(IncidentTarget::AllPlayers, 0, |game, p, i| {
        let mut ideas = p
            .get(game)
            .cities
            .iter()
            .filter(|c| {
                let b = c.pieces.buildings(None);
                b.contains(&Building::Academy) || b.contains(&Building::Observatory)
            })
            .count();
        if i.active_player == p.index {
            ideas = ideas.max(2);
        }
        p.gain_resources(game, ResourcePile::ideas(ideas as u8));
    })
    .build()
}

fn flourishing_trade() -> Incident {
    Incident::builder(
        46,
        "Flourishing Trade",
        "Every player gains an amount of gold equal to the number of cities \
        that have a Market or Port (up to a maximum of 3). You gain at least 1 gold.",
        IncidentBaseEffect::PiratesSpawnAndRaid,
    )
    .add_simple_incident_listener(IncidentTarget::AllPlayers, 0, |game, p, i| {
        let mut gold = p
            .get_mut(game)
            .cities
            .iter()
            .filter(|c| {
                let b = c.pieces.buildings(None);
                b.contains(&Building::Market) || b.contains(&Building::Port)
            })
            .count();

        gold = gold.min(3);

        if i.active_player == p.index {
            gold = gold.max(1);
        }
        p.gain_resources(game, ResourcePile::gold(gold as u8));
    })
    .build()
}

fn era_of_stability() -> Incident {
    Incident::builder(
        47,
        "Era of Stability",
        "Every player gains an amount of mood or culture tokens \
        equal to the number of cities that have a Temple or Obelisk (up to a maximum of 3). \
        You gain at least 1 token.",
        IncidentBaseEffect::ExhaustedLand,
    )
    .add_incident_resource_request(IncidentTarget::AllPlayers, 0, |game, p, i| {
        let player = p.get(game);
        let mut tokens = player
            .cities
            .iter()
            .filter(|c| {
                let b = c.pieces.buildings(None);
                b.contains(&Building::Temple) || b.contains(&Building::Obelisk)
            })
            .count();

        tokens = tokens.min(3);

        if i.active_player == p.index {
            tokens = tokens.max(1);
        }
        Some(ResourceRewardRequest::new(
            p.reward_options().tokens(tokens as u8),
            "Select token to gain".to_string(),
        ))
    })
    .build()
}

fn reformation() -> Incident {
    Incident::builder(
        48,
        "Reformation",
        "Select another player to replace one of your Temples with one of their Temples \
        (this can't be prevented). If you don't own any Temples, select a player \
        that has a Temple: they execute this event instead.",
        IncidentBaseEffect::BarbariansSpawn,
    )
    .add_simple_incident_listener(IncidentTarget::ActivePlayer, 4, |game, p, i| {
        if has_temple(game, p.index) {
            i.selected_player = Some(p.index);
        } else if game
            .players
            .iter()
            .filter(|p| has_temple(game, p.index) && p.is_human())
            .count()
            > 1
        {
            p.log(game, "Has no temples: Select a player to execute the event");
        }
    })
    // select a player to execute the incident
    .add_incident_player_request(
        IncidentTarget::ActivePlayer,
        "Select a player to execute the event",
        |p, game, i| has_temple(game, p.index) && i.selected_player.is_none(),
        3,
        |game, s, i| {
            // pass the event to the player itself
            i.passed = Some(PassedIncident::NewPlayer(s.choice));
            s.log(
                game,
                &format!(
                    "Selected {} to execute the event",
                    game.player_name(s.choice)
                ),
            );
        },
    )
    // select a player to gain a temple
    .add_incident_player_request(
        IncidentTarget::ActivePlayer,
        "Select a player to gain a Temple",
        |p, game, _| can_gain_temple(game, p),
        2,
        |_game, s, i| {
            i.selected_player = Some(s.choice);
        },
    )
    .add_incident_position_request(
        IncidentTarget::SelectedPlayer,
        1,
        |game, _p, i| {
            let donor = i.active_player;
            let choices = game
                .player(donor)
                .cities
                .iter()
                .filter(|c| city_has_temple(c, donor))
                .map(|c| c.position)
                .collect();
            let needed = 1..=1;
            Some(PositionRequest::new(
                choices,
                needed,
                "Select a city to gain a Temple",
            ))
        },
        |game, s, _| {
            gain_building(game, &s.player(), Building::Temple, s.choice[0]);
        },
    )
    .build()
}

fn has_temple(game: &Game, player: usize) -> bool {
    game.player(player)
        .cities
        .iter()
        .any(|c| city_has_temple(c, player))
}

fn city_has_temple(c: &City, player: usize) -> bool {
    c.pieces.buildings(Some(player)).contains(&Building::Temple)
}

fn can_gain_temple(game: &Game, player: &Player) -> bool {
    player.is_building_available(Building::Temple, game)
}
