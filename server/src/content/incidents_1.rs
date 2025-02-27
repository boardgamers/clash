use crate::ability_initializer::AbilityInitializerSetup;
use crate::city::MoodState;
use crate::content::builtin::Builtin;
use crate::content::custom_phase_actions::{PositionRequest, ResourceRewardRequest};
use crate::game::Game;
use crate::incident::{Incident, IncidentBaseEffect, IncidentBuilder, PermanentIncidentEffect};
use crate::payment::PaymentOptions;
use crate::player_events::IncidentTarget;
use crate::playing_actions::PlayingActionType;
use crate::resource::ResourceType;
use itertools::Itertools;

pub(crate) fn pestilence() -> Vec<Incident> {
    let mut builder = Incident::builder(
        1,
        "Pestilence",
        "Every player with 2 or more cities: Choose 1 (or 2 if you have Roads, Navigation, or Trade Routes) cities and decrease the mood by 1 in each of them. You must choose cities where this is possible. You cannot construct buildings or wonders until you research Sanitation.",
        IncidentBaseEffect::None)
        .set_protection_advance("Sanitation");
    builder = pestilence_city(builder, 2, |_, _| true);
    builder = pestilence_city(builder, 1, |game, player| {
        let p = game.get_player(player);
        p.has_advance("Roads") || p.has_advance("Navigation") || p.has_advance("Trade Routes")
    });
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

fn pestilence_city(
    b: IncidentBuilder,
    priority: i32,
    pred: impl Fn(&Game, usize) -> bool + 'static + Clone,
) -> IncidentBuilder {
    b.add_incident_position_request(
        IncidentTarget::AllPlayers,
        priority,
        move |game, player_index, _incident| {
            let cities = game
                .get_player(player_index)
                .cities
                .iter()
                .filter(|c| !matches!(c.mood_state, MoodState::Angry))
                .map(|c| c.position)
                .collect_vec();
            if pred(game, player_index) && !cities.is_empty() {
                Some(PositionRequest::new(
                    cities,
                    Some("Select a city to decrease the mood".to_string()),
                ))
            } else {
                None
            }
        },
        |game, s| {
            game.add_info_log_item(&format!(
                "{} decreased the mood in city {}",
                s.player_name, s.choice
            ));
            game.get_player_mut(s.player_index)
                .get_city_mut(s.choice)
                .expect("city should exist")
                .decrease_mood_state();
        },
    )
}

pub(crate) fn good_year() -> Vec<Incident> {
    vec![
        add_good_year(
            IncidentTarget::AllPlayers,
            Incident::builder(
                9,
                "A good year",
                "Every player gains 1 food",
                IncidentBaseEffect::BarbariansSpawn,
            ),
        ),
        add_good_year(
            IncidentTarget::ActivePlayer,
            Incident::builder(
                10,
                "A good year",
                "You gain 1 food",
                IncidentBaseEffect::BarbariansSpawn,
            ),
        ),
    ]
}

fn add_good_year(target: IncidentTarget, builder: IncidentBuilder) -> Incident {
    builder
        .add_incident_resource_request(
            target,
            0,
            |_game, _player_index, _incident| {
                Some(ResourceRewardRequest::new(
                    PaymentOptions::sum(1, &[ResourceType::Food]),
                    "Gain 1 food".to_string(),
                ))
            },
            |_game, s| {
                vec![format!(
                    "{} gained {} from A good year",
                    s.player_name, s.choice
                )]
            },
        )
        .build()
}
