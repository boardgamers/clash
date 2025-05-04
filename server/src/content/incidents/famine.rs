use crate::ability_initializer::{AbilityInitializerSetup, SelectedChoice};
use crate::advance::Advance;
use crate::city::non_angry_cites;
use crate::city::{City, MoodState};
use crate::content::builtin::Builtin;
use crate::content::effects::PermanentEffect;
use crate::content::persistent_events::UnitsRequest;
use crate::game::Game;
use crate::incident::{DecreaseMood, Incident, IncidentBaseEffect, MoodModifier};
use crate::player::Player;
use crate::player_events::IncidentTarget;
use crate::playing_actions::PlayingActionType;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::unit::kill_units;
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
    .with_protection_advance(Advance::Sanitation)
    .add_decrease_mood(
        IncidentTarget::AllPlayers,
        MoodModifier::Decrease,
        move |p, _game, _| {
            if !pestilence_applies(p) {
                return DecreaseMood::none();
            }

            DecreaseMood::new(
                non_angry_cites(p),
                if additional_sanitation_damage(p) {
                    2
                } else {
                    1
                },
            )
        },
    )
    .add_simple_incident_listener(IncidentTarget::ActivePlayer, 0, |game, _, _, _| {
        game.permanent_effects.push(PermanentEffect::Pestilence);
    })
    .add_simple_persistent_event_listener(
        |event| &mut event.advance,
        11,
        |game, _player_index, _player_name, i| {
            if i.advance == Advance::Sanitation
                && game
                    .players
                    .iter()
                    .all(|p| !p.is_human() || p.has_advance(Advance::Sanitation))
            {
                game.permanent_effects
                    .retain(|e| e != &PermanentEffect::Pestilence);
                game.add_info_log_item("Pestilence removed");
            }
        },
    )
    .build()
}

fn pestilence_applies(player: &Player) -> bool {
    player.cities.len() >= 2
}

pub(crate) fn additional_sanitation_damage(p: &Player) -> bool {
    p.can_use_advance(Advance::Roads)
        || p.can_use_advance(Advance::Navigation)
        || p.can_use_advance(Advance::TradeRoutes)
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
            let player = game.player(i.player);
            if game
                .permanent_effects
                .contains(&PermanentEffect::Pestilence)
                && matches!(i.action_type, PlayingActionType::Construct)
                && !player.can_use_advance(Advance::Sanitation)
            {
                *available = Err(
                    "Cannot construct buildings or wonders until you research Sanitation."
                        .to_string(),
                );
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
    .with_protection_advance(Advance::Sanitation)
    .add_incident_units_request(
        IncidentTarget::AllPlayers,
        0,
        |game, player_index, _incident| {
            let p = game.player(player_index);
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
        |game, s, _| {
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

    let p = game.player(s.player_index);
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
    kill_units(game, &s.choice, s.player_index, None);
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
    amount: impl Fn(&Player, &Game) -> u8 + Clone + 'static + Sync + Send,
    player_pred: impl Fn(&Player) -> bool + Clone + 'static + Sync + Send,
    city_pred: impl Fn(&City, &Game) -> bool + Clone + 'static + Sync + Send,
) -> Incident {
    let player_pred2 = player_pred.clone();
    let city_pred2 = city_pred.clone();
    Incident::builder(id, name, description, incident_base_effect)
        .with_protection_advance(Advance::Irrigation)
        .add_simple_incident_listener(target, 11, move |game, player_index, player_name, i| {
            // we lose the food regardless of the outcome
            let p = game.player(player_index);
            if !player_pred.clone()(p) {
                return;
            }

            let needed = amount(p, game);
            let lost = needed.min(p.resources.food);

            if lost == needed {
                // only avoid anger if full amount is paid
                i.player.payment = ResourcePile::food(lost);
            }

            game.player_mut(player_index)
                .lose_resources(ResourcePile::food(lost));

            game.add_info_log_item(&format!("{player_name} lost {lost} food to Famine",));
        })
        .add_decrease_mood(target, MoodModifier::MakeAngry, move |p, game, i| {
            if player_pred2(p) && i.player.payment.is_empty() {
                DecreaseMood::new(famine_targets(p, game, city_pred2.clone()), 1)
            } else {
                DecreaseMood::none()
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
