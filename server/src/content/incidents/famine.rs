use crate::ability_initializer::{AbilityInitializerSetup, SelectedChoice};
use crate::city::{City, MoodState};
use crate::content::builtin::Builtin;
use crate::content::custom_phase_actions::UnitsRequest;
use crate::content::incidents::civil_war::non_angry_cites;
use crate::game::Game;
use crate::incident::{Incident, IncidentBaseEffect, MoodModifier, PermanentIncidentEffect};
use crate::player::Player;
use crate::player_events::IncidentTarget;
use crate::playing_actions::PlayingActionType;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use itertools::Itertools;
use std::vec;

pub(crate) fn pestilence_incidents() -> Vec<Incident> {
    let mut r = vec![pestilence(), epidemics()];
    r.extend(famines());
    r
}

fn pestilence() -> Incident {
    Incident::builder(
        1,
        "Pestilence",
        "Every player with 2 or more cities: \
            Choose 1 (or 2 if you have Roads, Navigation, or Trade Routes) cities \
            and decrease the mood by 1 in each of them. \
            You must choose cities where this is possible. \
            You cannot construct buildings or wonders until you research Sanitation.",
        IncidentBaseEffect::None,
    )
    .set_protection_advance("Sanitation")
    .add_decrease_mood(
        IncidentTarget::AllPlayers,
        MoodModifier::Decrease,
        move |p, game| {
            if !pestilence_applies(p) {
                return (vec![], 0);
            }

            let needed = if additional_sanitation_damage(p) {
                2
            } else {
                1
            } - game.current_event_player().payment.amount() as u8;

            (non_angry_cites(p), needed)
        },
    )
    .add_simple_incident_listener(IncidentTarget::ActivePlayer, 0, |game, _, _, _| {
        game.permanent_incident_effects
            .push(PermanentIncidentEffect::Pestilence);
    })
    .build()
}

fn pestilence_applies(player: &Player) -> bool {
    player.cities.len() >= 2
}

pub(crate) fn additional_sanitation_damage(p: &Player) -> bool {
    p.has_advance("Roads") || p.has_advance("Navigation") || p.has_advance("Trade Routes")
}

pub(crate) fn pestilence_permanent_effect() -> Builtin {
    Builtin::builder(
        "Pestilence",
        "You cannot construct buildings or wonders until you research Sanitation.",
    )
    .add_transient_event_listener(
        |event| &mut event.is_playing_action_available,
        1,
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
    )
    .build()
}

fn epidemics() -> Incident {
    Incident::builder(
        2,
        "Epidemics",
        "Every player with 2 or more units: \
            Choose 1 (or 2 if you have Roads, Navigation, or Trade Routes) units and kill them.",
        IncidentBaseEffect::None,
    )
    .set_protection_advance("Sanitation")
    .add_incident_units_request(
        IncidentTarget::AllPlayers,
        0,
        |game, player_index, _incident| {
            let p = game.get_player(player_index);
            let units = p.units.iter().map(|u| u.id).collect_vec();
            let needed = if additional_sanitation_damage(p) {
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
                    "Select units to kill",
                ))
            }
        },
        |game, s| {
            kill_incident_units(game, s);
        },
    )
    .build()
}

pub(crate) fn kill_incident_units(game: &mut Game, s: &SelectedChoice<Vec<u32>>) {
    if s.choice.is_empty() {
        game.add_info_log_item(&format!("{} declined to kill units", s.player_name));
        return;
    }

    let p = game.get_player(s.player_index);
    game.add_info_log_item(&format!(
        "{} killed units: {}",
        p.get_name(),
        s.choice
            .iter()
            .map(|u| {
                let unit = p.get_unit(*u);
                format!("{:?} at {}", unit.unit_type, unit.position)
            })
            .join(", ")
    ));
    for u in &s.choice {
        game.kill_unit(*u, s.player_index, None);
    }
}

fn famines() -> Vec<Incident> {
    vec![
        common_famine(3, false),
        common_famine(4, false),
        common_famine(5, false),
        common_famine(6, true),
        common_famine(7, true),
        common_famine(8, true),
    ]
}

fn common_famine(id: u8, severe: bool) -> Incident {
    let (name, amount) = if severe {
        ("Severe famine", 2)
    } else {
        ("Famine", 1)
    };

    let description = &format!(
        "You must pay up to {amount} food (gold is not possible). If you cannot pay the full amount, make 1 city Angry."
    );
    famine(
        id,
        name,
        description,
        IncidentTarget::ActivePlayer,
        IncidentBaseEffect::BarbariansMove,
        move |_, _| amount,
        |_| true,
        |_, _| true,
    )
}

pub(crate) fn famine(
    id: u8,
    name: &str,
    description: &str,
    target: IncidentTarget,
    incident_base_effect: IncidentBaseEffect,
    amount: impl Fn(&Player, &Game) -> u8 + Clone + 'static,
    player_pred: impl Fn(&Player) -> bool + Clone + 'static,
    city_pred: impl Fn(&City, &Game) -> bool + Clone + 'static,
) -> Incident {
    let player_pred2 = player_pred.clone();
    let city_pred2 = city_pred.clone();
    Incident::builder(id, name, description, incident_base_effect)
        .set_protection_advance("Irrigation")
        .add_simple_incident_listener(target, 11, move |game, player_index, player_name, _| {
            // we lose the food regardless of the outcome
            let p = game.get_player(player_index);
            if !player_pred.clone()(p) {
                return;
            }

            let needed = amount(p, game);
            let lost = needed.min(p.resources.food as u8) as u32;

            if lost == needed as u32 {
                // only avoid anger if full amount is paid
                game.current_event_mut().player.payment = ResourcePile::food(lost);
            }

            game.get_player_mut(player_index)
                .lose_resources(ResourcePile::food(lost));

            game.add_info_log_item(&format!("{player_name} lost {lost} food to Famine",));
        })
        .add_decrease_mood(target, MoodModifier::MakeAngry, move |p, game| {
            if player_pred2(p) && game.current_event_player().payment.is_empty() {
                (famine_targets(p, game, city_pred2.clone()), 1)
            } else {
                (vec![], 0)
            }
        })
        .build()
}

fn famine_targets(
    p: &Player,
    game: &Game,
    city_pred: impl Fn(&City, &Game) -> bool,
) -> Vec<Position> {
    p.cities
        .iter()
        .filter(|c| !matches!(c.mood_state, MoodState::Angry) && city_pred(c, game))
        .map(|c| c.position)
        .collect_vec()
}
