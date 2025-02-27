use crate::content::custom_phase_actions::{PlayerRequest, PositionRequest};
use crate::game::Game;
use crate::incident::{Incident, IncidentBaseEffect, IncidentBuilder};
use crate::player_events::IncidentTarget;
use crate::unit::UnitType;
use itertools::Itertools;

pub(crate) fn population_boom() -> Vec<Incident> {
    let mut b = Incident::builder(
        28,
        "Population Boom",
        "-",
        IncidentBaseEffect::BarbariansMove,
    );
    b = select_settler(b, 2, |game, _| {
        game.current_custom_phase().is_active_player()
    });
    b = b.add_incident_player_request(
        IncidentTarget::ActivePlayer,
        1,
        |game, player_index, _incident| {
            let choices = game
                .players
                .iter()
                .filter(|p| p.available_units().settlers > 0 && p.index != player_index)
                .map(|p| p.index)
                .collect_vec();

            if choices.is_empty() {
                None
            } else {
                Some(PlayerRequest::new(
                    choices,
                    "Select a unit to gain 1 settler",
                ))
            }
        },
        |game, c| {
            game.add_info_log_item(&format!(
                "{} was selected to gain 1 settler from Population Boom",
                game.get_player(c.choice).get_name()
            ));
            game.current_custom_phase_mut().selected_player = Some(c.choice);
        },
    );
    b = select_settler(b, 0, |game, player| {
        let c = game.current_custom_phase();
        c.selected_player == Some(player)
    });
    vec![b.build()]
}

fn select_settler(
    b: IncidentBuilder,
    priority: i32,
    pred: impl Fn(&Game, usize) -> bool + 'static + Clone,
) -> IncidentBuilder {
    b.add_incident_position_request(
        IncidentTarget::AllPlayers,
        priority,
        move |game, player_index, _incident| {
            let p = game.get_player(player_index);
            if pred(game, player_index) && p.available_units().settlers > 0 {
                Some(PositionRequest::new(
                    p.cities.iter().map(|c| c.position).collect(),
                    Some("Select a city to gain 1 settler".to_string()),
                ))
            } else {
                None
            }
        },
        |game, s| {
            game.add_info_log_item(&format!(
                "{} gained 1 settler in {}",
                s.player_name, s.choice
            ));
            game.get_player_mut(s.player_index)
                .add_unit(s.choice, UnitType::Settler);
        },
    )
}
