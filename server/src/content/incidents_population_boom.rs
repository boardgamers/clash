use crate::content::custom_phase_actions::new_position_request;
use crate::game::Game;
use crate::incident::{Incident, IncidentBaseEffect, IncidentBuilder};
use crate::player_events::{IncidentInfo, IncidentTarget};
use crate::unit::UnitType;

pub(crate) fn population_booms() -> Vec<Incident> {
    vec![
        population_boom(27, IncidentBaseEffect::BarbariansSpawn),
        population_boom(28, IncidentBaseEffect::BarbariansMove),
    ]
}

fn population_boom(id: u8, effect: IncidentBaseEffect) -> Incident {
    let mut b = Incident::builder(id, "Population Boom", "-", effect);
    b = select_settler(b, 13, IncidentTarget::ActivePlayer);
    select_player_to_gain_settler(b).build()
}

pub(crate) fn select_player_to_gain_settler(mut b: IncidentBuilder) -> IncidentBuilder {
    b = b.add_incident_player_request(
        IncidentTarget::ActivePlayer,
        "Select a player to gain 1 settler",
        |p, _, _| p.available_units().settlers > 0 && !p.cities.is_empty(),
        12,
        |game, c| {
            game.add_info_log_item(&format!(
                "{} was selected to gain 1 settler.",
                game.player_name(c.choice)
            ));
            game.current_event_mut().selected_player = Some(c.choice);
        },
    );
    select_settler(b, 11, IncidentTarget::SelectedPlayer)
}

fn select_settler(
    b: IncidentBuilder,
    priority: i32,
    target: IncidentTarget,
) -> IncidentBuilder {
    b.add_incident_position_request(
        IncidentTarget::AllPlayers,
        priority,
        move |game, player_index, _| {
            let p = game.get_player(player_index);
            if p.available_units().settlers > 0 {
                Some(new_position_request(
                    p.cities.iter().map(|c| c.position).collect(),
                    1..=1,
                    "Select a city to gain 1 settler",
                ))
            } else {
                None
            }
        },
        |game, s| {
            let pos = s.choice[0];
            game.add_info_log_item(&format!("{} gained 1 settler in {}", s.player_name, pos));
            game.get_player_mut(s.player_index)
                .add_unit(pos, UnitType::Settler);
        },
    )
}
