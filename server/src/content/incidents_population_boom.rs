use crate::content::custom_phase_actions::PositionRequest;
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
    b = select_settler(b, 2, |_game, player, i| {
        i.is_active(IncidentTarget::ActivePlayer, player)
    });
    b = b.add_incident_player_request(
        "Select a player to gain 1 settler",
        |p| p.available_units().settlers > 0,
        1,
        |game, c| {
            game.add_info_log_item(&format!(
                "{} was selected to gain 1 settler from Population Boom",
                game.get_player(c.choice).get_name()
            ));
            game.current_event_mut().selected_player = Some(c.choice);
        },
    );
    b = select_settler(b, 0, |game, player, _| {
        game.current_event().selected_player == Some(player)
    });
    b.build()
}

fn select_settler(
    b: IncidentBuilder,
    priority: i32,
    pred: impl Fn(&Game, usize, &IncidentInfo) -> bool + 'static + Clone,
) -> IncidentBuilder {
    b.add_incident_position_request(
        IncidentTarget::AllPlayers,
        priority,
        move |game, player_index, incident| {
            let p = game.get_player(player_index);
            if pred(game, player_index, incident) && p.available_units().settlers > 0 {
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
