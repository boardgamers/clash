use crate::city::MoodState;
use crate::content::custom_phase_actions::{new_position_request, UnitsRequest};
use crate::content::incidents_famine::{decrease_mod_and_log, decrease_mood_incident_city};
use crate::content::incidents_population_boom::select_player_to_gain_settler;
use crate::game::{Game, GameState};
use crate::incident::{Incident, IncidentBaseEffect, IncidentBuilder, PermanentIncidentEffect};
use crate::player::Player;
use crate::player_events::IncidentTarget;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::status_phase::{add_change_government, can_change_government_for_free};
use crate::unit::UnitType;
use itertools::Itertools;

pub(crate) fn migrations() -> Vec<Incident> {
    vec![
        migration(34),
        migration(35),
        civil_war(36),
        civil_war(37),
        revolution(),
    ]
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
        (non_angry_cites(game.get_player(player_index)), 1)
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
            return (vec![], 0);
        }
        if non_happy_cites_with_infantry(game.get_player(player_index)).is_empty() {
            return (non_angry_cites(game.get_player(player_index)), 1);
        }
        (vec![], 0)
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
            Some(new_position_request(
                non_happy_cites_with_infantry(p),
                1..=1,
                &format!("Select a non-Happy city with an Infantry to kill the Infantry {suffix}"),
            ))
        },
        |game, s| {
            let position = s.choice[0];
            let mood = game.get_any_city(position).mood_state.clone();
            if game.current_event_player().payment.is_empty() && !matches!(mood, MoodState::Angry) {
                decrease_mod_and_log(game, s);
            }
            let unit = game
                .get_player(s.player_index)
                .get_units(position)
                .iter()
                .filter(|u| matches!(u.unit_type, UnitType::Infantry))
                .sorted_by_key(|u| u.movement_restrictions.len())
                .next_back()
                .expect("infantry should exist")
                .id;
            game.add_info_log_item(&format!(
                "{} killed an Infantry in {}",
                s.player_name, position
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

fn revolution() -> Incident {
    let mut b = Incident::builder(
        38,
        "Revolution",
        "You may kill one of your Army units each to avoid the following steps: Step 1: Loose one action (from your next turn if in Status phase). Step 2: Change your government for free if possible.",
        IncidentBaseEffect::GoldDeposits,
    );
    b = kill_unit_for_revolution(
        b,
        3,
        "Kill a unit to avoid losing an action",
        |game, _player| can_loose_action(game),
    );
    b = b.add_incident_listener(IncidentTarget::ActivePlayer, 2, |game, player| {
        if can_loose_action(game) && game.current_event_player().sacrifice == 0 {
            loose_action(game, player);
        }
    });
    b = kill_unit_for_revolution(
        b,
        1,
        "Kill a unit to avoid changing government",
        |_game, player| can_change_government_for_free(player),
    );
    b = add_change_government(
        b,
        |event| &mut event.on_incident,
        false,
        ResourcePile::empty(),
    );
    b.build()
}

fn kill_unit_for_revolution(
    b: IncidentBuilder,
    priority: i32,
    description: &str,
    pred: impl Fn(&Game, &Player) -> bool + 'static + Clone,
) -> IncidentBuilder {
    let description = description.to_string();
    b.add_incident_units_request(
        IncidentTarget::ActivePlayer,
        priority,
        move |game, player_index, _incident| {
            game.current_event_mut().player.sacrifice = 0;
            let units = game
                .get_player(player_index)
                .units
                .iter()
                .filter(|u| u.unit_type.is_army_unit())
                .map(|u| u.id)
                .collect_vec();
            Some(UnitsRequest::new(
                player_index,
                if pred(game, game.get_player(player_index)) {
                    units
                } else {
                    vec![]
                },
                0..=1,
                &description,
            ))
        },
        |game, s| {
            if s.choice.is_empty() {
                game.add_info_log_item(&format!("{} did not kill an Army unit", s.player_name));
                return;
            }
            game.add_info_log_item(&format!("{} killed an Army unit", s.player_name));
            game.kill_unit(s.choice[0], s.player_index, None);
            game.current_event_mut().player.sacrifice = 1;
        },
    )
}

fn can_loose_action(game: &Game) -> bool {
    match game.state() {
        GameState::StatusPhase(_) => true,
        _ => game.actions_left > 0,
    }
}

fn loose_action(game: &mut Game, player: usize) {
    let name = game.get_player(player).get_name();
    if let GameState::StatusPhase(_) = game.state() {
        game.add_info_log_item(&format!("{name} lost an action for the next turn"));
        game.permanent_incident_effects
            .push(PermanentIncidentEffect::LooseAction(player));
    } else {
        game.add_info_log_item(&format!("{name} lost an action"));
        game.actions_left -= 1;
    };
}
