use crate::ability_initializer::{AbilityInitializerSetup, SelectedChoice};
use crate::city::MoodState;
use crate::content::builtin::Builtin;
use crate::content::custom_phase_actions::{new_position_request, UnitsRequest};
use crate::game::Game;
use crate::incident::{Incident, IncidentBaseEffect, IncidentBuilder, PermanentIncidentEffect};
use crate::player::Player;
use crate::player_events::{IncidentInfo, IncidentTarget};
use crate::playing_actions::PlayingActionType;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use itertools::Itertools;
use std::vec;

pub(crate) fn pestilence() -> Vec<Incident> {
    let mut builder = Incident::builder(
        1,
        "Pestilence",
        "Every player with 2 or more cities: Choose 1 (or 2 if you have Roads, Navigation, or Trade Routes) cities and decrease the mood by 1 in each of them. You must choose cities where this is possible. You cannot construct buildings or wonders until you research Sanitation.",
        IncidentBaseEffect::None)
        .set_protection_advance("Sanitation");
    builder = builder.add_myths_payment(IncidentTarget::AllPlayers, |_game, p| {
        if pestilence_applies(p) {
            if additional_sanitation_damage(p) {
                2
            } else {
                1
            }
        } else {
            0
        }
    });
    builder = pestilence_city(builder, 1);
    builder = builder.add_incident_listener(IncidentTarget::ActivePlayer, 0, |game, _p, _i| {
        if game
            .permanent_incident_effects
            .contains(&PermanentIncidentEffect::Pestilence)
        {
            return;
        }
        game.permanent_incident_effects
            .push(PermanentIncidentEffect::Pestilence);
    });
    vec![builder.build()]
}

fn pestilence_applies(player: &Player) -> bool {
    player.cities.len() >= 2
}

fn additional_sanitation_damage(p: &Player) -> bool {
    p.has_advance("Roads") || p.has_advance("Navigation") || p.has_advance("Trade Routes")
}

pub(crate) fn pestilence_permanent_effect() -> Builtin {
    Builtin::builder(
        "Pestilence",
        "You cannot construct buildings or wonders until you research Sanitation.",
    )
    .add_player_event_listener(
        |event| &mut event.is_playing_action_available,
        |available, game, i| {
            let player = game.get_player(i.player);
            if game
                .permanent_incident_effects
                .contains(&PermanentIncidentEffect::Pestilence)
                && matches!(i.action_type, PlayingActionType::Construct)
                && !player.has_advance("Sanitation")
            {
                *available = false;
            }
        },
        1,
    )
    .build()
}

fn pestilence_city(b: IncidentBuilder, priority: i32) -> IncidentBuilder {
    decrease_mood_incident_city(
        b,
        IncidentTarget::AllPlayers,
        priority,
        move |game, player_index| {
            let p = game.get_player(player_index);
            if !pestilence_applies(p) {
                return (vec![], 0);
            }

            let needed = if additional_sanitation_damage(p) {
                2
            } else {
                1
            } - game.current_event_player().payment.resource_amount() as u8;

            (
                p.cities
                    .iter()
                    .filter(|c| !matches!(c.mood_state, MoodState::Angry))
                    .map(|c| c.position)
                    .collect_vec(),
                needed,
            )
        },
    )
}

pub(crate) fn decrease_mood_incident_city(
    b: IncidentBuilder,
    target: IncidentTarget,
    priority: i32,
    cities: impl Fn(&Game, usize) -> (Vec<Position>, u8) + 'static + Clone,
) -> IncidentBuilder {
    b.add_incident_position_request(
        target,
        priority,
        move |game, player_index, _incident| {
            let (cities, needed) = cities(game, player_index);
            Some(new_position_request(
                cities,
                needed..=needed,
                Some("Select a city to decrease the mood".to_string()),
            ))
        },
        |game, s| {
            decrease_mod_and_log(game, s);
        },
    )
}

pub(crate) fn decrease_mod_and_log(
    game: &mut Game,
    s: &SelectedChoice<Vec<Position>, IncidentInfo>,
) {
    let pos = s.choice[0];
    game.add_info_log_item(&format!(
        "{} decreased the mood in city {}",
        s.player_name, pos
    ));
    game.get_player_mut(s.player_index)
        .get_city_mut(pos)
        .decrease_mood_state();
}

pub(crate) fn epidemics() -> Vec<Incident> {
    vec![Incident::builder(
        2,
        "Epidemics",
        "Every player with 2 or more units: Choose 1 (or 2 if you have Roads, Navigation, or Trade Routes) units and kill them.",
        IncidentBaseEffect::None)
        .set_protection_advance("Sanitation")
        .add_incident_units_request(IncidentTarget::AllPlayers, 0, |game, player_index, _incident| {
            let p = game
                .get_player(player_index);
            let units = p
                .units
                .iter()
                .map(|u| u.id)
                .collect_vec();
            let needed = if additional_sanitation_damage(p)
            {
                2
            } else {
                1
            };
            if units.len() <= 2 {
                None
            } else {
                Some(UnitsRequest::new(
                    player_index,
                    units,
                    needed..=needed,
                    Some("Select units to kill".to_string()),
                ))
            }
        },
        |game, s| {
            let p = game.get_player(s.player_index);
            game.add_info_log_item(&format!(
                "{} killed units: {}",
                p.get_name(),
                s.choice.iter().map(|u| {
                    let unit = p
                        .get_unit(*u);
                    format!("{:?} at {}", unit.unit_type, unit.position)
                }).join(", ")
            ));
            for u in &s.choice {
                game.kill_unit(*u, s.player_index, None);
            }
        },
        ).build()]
}

pub(crate) fn famines() -> Vec<Incident> {
    vec![
        famine(3, false),
        famine(4, false),
        famine(5, false),
        famine(6, true),
        famine(7, true),
        famine(8, true),
    ]
}

pub(crate) fn famine(id: u8, severe: bool) -> Incident {
    let (name, amount) = if severe {
        ("Severe famine", 2)
    } else {
        ("Famine", 1)
    };

    Incident::builder(
        id,
        name,
        &format!(
            "You must pay up to {amount} food (gold is not possible). If you cannot pay the full amount, make 1 city Angry."
        ),
        IncidentBaseEffect::BarbariansMove,
    )
    .set_protection_advance("Irrigation")
    .add_myths_payment(IncidentTarget::ActivePlayer, move |_game, p| {
        u32::from(famine_applies(p, amount))
    })
    .add_incident_position_request(
        IncidentTarget::ActivePlayer,
        0,
        move |game, player_index, _incident| {
            let p = game.get_player(player_index);
            let applies = famine_applies(p, amount);
            let lost = amount.min(p.resources.food as i32) as u32;

            // we lose the food regardless of the outcome
            game.get_player_mut(player_index)
                .lose_resources(ResourcePile::food(lost));

            game.add_info_log_item(&format!(
                "{} lost {} food to Famine",
                game.get_player(player_index).get_name(),
                lost
            ));

            if applies && game.current_event_player().payment.is_empty() {
                Some(new_position_request(
                    famine_targets(game.get_player(player_index)),
                    1..=1,
                    Some("Select a city to make Angry".to_string()),
                ))
            } else {
                None
            }
        },
        |game, s| {
            let p = game.get_player_mut(s.player_index);
            let pos = s.choice[0];
            p.get_city_mut(pos)
                .mood_state = MoodState::Angry;
            let name = p.get_name();
            game.add_info_log_item(&format!("{name} made city {pos} Angry"));
        },
    )
    .build()
}

fn famine_applies(p: &Player, amount: i32) -> bool {
    p.resources.food < amount as u32 && !famine_targets(p).is_empty()
}

fn famine_targets(p: &Player) -> Vec<Position> {
    p.cities
        .iter()
        .filter(|c| !matches!(c.mood_state, MoodState::Angry))
        .map(|c| c.position)
        .collect_vec()
}
