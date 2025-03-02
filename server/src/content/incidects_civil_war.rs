use crate::city::MoodState;
use crate::content::custom_phase_actions::PositionRequest;
use crate::content::incidents_famine::{decrease_mod_and_log, decrease_mood_incident_city};
use crate::content::incidents_population_boom::select_player_to_gain_settler;
use crate::incident::{Incident, IncidentBaseEffect};
use crate::player::Player;
use crate::player_events::IncidentTarget;
use crate::position::Position;
use crate::unit::UnitType;
use itertools::Itertools;

pub(crate) fn migrations() -> Vec<Incident> {
    vec![migration(34), migration(35), civil_war(36), civil_war(37)]
}

fn migration(id: u8) -> Incident {
    let mut b = Incident::builder(
        id,
        "Migration",
        "Select a player to gain 1 settler in one of their cities. Decrease the mood in one of your cities.",
        IncidentBaseEffect::GoldDeposits,
    );
    b = select_player_to_gain_settler(b);
    b = b.add_myths_payment(IncidentTarget::ActivePlayer, |_game, p| {
        u32::from(!non_angry_cites(p).is_empty())
    });
    decrease_mood_incident_city(b, IncidentTarget::ActivePlayer, 0, |game, player_index| {
        non_angry_cites(game.get_player(player_index))
    })
    .build()
}

fn non_angry_cites(p: &Player) -> Vec<Position> {
    p.cities
        .iter()
        .filter(|c| !matches!(c.mood_state, MoodState::Angry))
        .map(|c| c.position)
        .collect_vec()
}

fn civil_war(id: u8) -> Incident {
    let mut b = Incident::builder(
        id,
        "Civil War",
        "Select a non-Happy city with an Infantry: kill the Infantry and decrease the mood. If no such city exists, select a city to decrease the mood.",
        IncidentBaseEffect::None,
    );
    b = b.add_myths_payment(IncidentTarget::ActivePlayer, |_game, p| {
        u32::from(!non_angry_cites(p).is_empty())
    });
    b = decrease_mood_incident_city(b, IncidentTarget::ActivePlayer, 1, |game, player_index| {
        if !game.current_event_player().payment.is_empty() {
            return vec![];
        }
        if non_happy_cites_with_infantry(game.get_player(player_index)).is_empty() {
            return non_angry_cites(game.get_player(player_index));
        }
        vec![]
    });
    b = b.add_incident_position_request(
        IncidentTarget::ActivePlayer,
        0,
        |game, player_index, _incident| {
            let p = game.get_player(player_index);
            let suffix = if !non_angry_cites(p).is_empty()
                && game.current_event_player().payment.is_empty()
            {
                " and decrease the mood"
            } else {
                ""
            };
            Some(PositionRequest::new(
                non_happy_cites_with_infantry(p),
                Some(format!(
                    "Select a non-Happy city with an Infantry to kill the Infantry {suffix}"
                )),
            ))
        },
        |game, s| {
            let mood = game
                .get_any_city(s.choice)
                .expect("city should exist")
                .mood_state
                .clone();
            if game.current_event_player().payment.is_empty() && !matches!(mood, MoodState::Angry) {
                decrease_mod_and_log(game, s);
            }
            let unit = game
                .get_player(s.player_index)
                .get_units(s.choice)
                .iter()
                .filter(|u| matches!(u.unit_type, UnitType::Infantry))
                .sorted_by_key(|u| u.movement_restrictions.len()).next_back()
                .expect("infantry should exist")
                .id;
            game.add_info_log_item(&format!(
                "{} killed an Infantry in {}",
                s.player_name, s.choice
            ));
            game.kill_unit(unit, s.player_index, None);
        },
    );
    b.build()
}

fn non_happy_cites_with_infantry(p: &Player) -> Vec<Position> {
    p.cities
        .iter()
        .filter(|c| {
            !matches!(c.mood_state, MoodState::Happy)
                && p.get_units(c.position)
                    .iter()
                    .any(|u| matches!(u.unit_type, UnitType::Infantry))
        })
        .map(|c| c.position)
        .collect_vec()
}
