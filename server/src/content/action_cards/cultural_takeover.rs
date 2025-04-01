use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{on_play_action_card, ActionCard, ActionCardInfo};
use crate::barbarians::get_barbarians_player;
use crate::content::builtin::Builtin;
use crate::content::custom_actions::CustomActionType;
use crate::content::custom_phase_actions::{Structure, UnitTypeRequest};
use crate::content::effects::PermanentEffect;
use crate::content::tactics_cards::TacticsCardFactory;
use crate::game::Game;
use crate::player_events::{InfluenceCultureInfo, PlayingActionInfo};
use crate::playing_actions::{ActionType, PlayingActionType};
use crate::unit::UnitType;
use crate::utils::remove_element;
use itertools::Itertools;

pub(crate) fn cultural_takeover(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Cultural Takeover",
        "You may influence Barbarian cities of size 1. \
        If successful, replace the Barbarian city with a city of your color. \
        Replace one of the Barbarian units with a Settler or Infantry of your color. \
        Remove the other Barbarian units.",
        ActionType::free(),
        |_game, _p| true,
    )
    .add_unit_type_request(
        |event| &mut event.on_play_action_card,
        1,
        |game, player, a| {
            if let Some(position) = a.selected_position {
                //set in use_cultural_takeover
                let b = game.get_player_mut(get_barbarians_player(game).index);
                let units = b.get_units(position).iter().map(|u| u.id).collect_vec();
                let len = units.len();
                for id in units {
                    b.remove_unit(id);
                }
                if len > 0 {
                    let p = game.get_player_mut(player);
                    let u = p.available_units();
                    let mut t = vec![];
                    if u.settlers > 0 {
                        t.push(UnitType::Settler);
                    }
                    if u.infantry > 0 {
                        t.push(UnitType::Infantry);
                    }
                    return Some(UnitTypeRequest::new(
                        t,
                        player,
                        &format!("Select unit to gain at {position}"),
                    ));
                }
            }
            None
        },
        |game, s, a| {
            game.add_info_log_item(&format!(
                "{} selected unit to gain: {:?}",
                s.player_name, s.choice,
            ));
            game.get_player_mut(s.player_index)
                .add_unit(a.selected_position.expect("unit position"), s.choice);
        },
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.on_play_action_card,
        0,
        |game, _player, _name, a| {
            if a.selected_position.is_none() {
                // skip this the second time where we only select a unit type to add
                game.permanent_effects
                    .push(PermanentEffect::CulturalTakeover);
                game.add_info_log_item(
                    "Cultural Takeover: You may influence Barbarian cities of size 1.",
                );
            }
        },
    )
    .tactics_card(tactics_card)
    .build()
}

pub(crate) fn use_cultural_takeover() -> Builtin {
    Builtin::builder("cultural_takeover", "-")
        .add_transient_event_listener(
            |event| &mut event.is_playing_action_available,
            3,
            |available, game, i| {
                if game
                    .permanent_effects
                    .contains(&PermanentEffect::CulturalTakeover)
                    && !is_influence(i)
                {
                    *available =
                        Err("Cultural Takeover: You may only influence culture.".to_string());
                }
            },
        )
        .add_transient_event_listener(
            |event| &mut event.on_influence_culture_attempt,
            5,
            |c, _, game| {
                if c.is_defender
                    && matches!(c.structure, Structure::CityCenter)
                    && !is_barbarian_takeover(game, c)
                {
                    // only add in is_defender to avoid double messages
                    c.add_blocker("City center can't be influenced");
                }
            },
        )
        .add_transient_event_listener(
            |event| &mut event.on_influence_culture_resolve,
            1,
            |game, outcome, ()| {
                if remove_element(
                    &mut game.permanent_effects,
                    &PermanentEffect::CulturalTakeover,
                )
                .is_some_and(|_| outcome.success)
                {
                    let mut info = ActionCardInfo::new(15);
                    info.selected_position = Some(outcome.position);
                    on_play_action_card(game, outcome.player, info);
                }
            },
        )
        .build()
}

fn is_barbarian_takeover(game: &Game, c: &InfluenceCultureInfo) -> bool {
    let city = game.get_any_city(c.position);
    city.player_index == get_barbarians_player(game).index
        && city.size() == 1
        && game
            .permanent_effects
            .contains(&PermanentEffect::CulturalTakeover)
}

fn is_influence(i: &PlayingActionInfo) -> bool {
    match &i.action_type {
        PlayingActionType::InfluenceCultureAttempt => true,
        PlayingActionType::Custom(i)
            if i.custom_action_type == CustomActionType::ArtsInfluenceCultureAttempt =>
        {
            true
        }
        _ => false,
    }
}
