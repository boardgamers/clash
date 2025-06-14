use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{ActionCard, ActionCardInfo, on_play_action_card};
use crate::barbarians::get_barbarians_player;
use crate::content::ability::Ability;
use crate::content::custom_actions::is_base_or_modifier;
use crate::content::effects::PermanentEffect;
use crate::content::persistent_events::{SelectedStructure, Structure, UnitTypeRequest};
use crate::content::tactics_cards::TacticsCardFactory;
use crate::cultural_influence::{
    InfluenceCultureInfo, available_influence_actions, influence_culture_boost_cost,
};
use crate::game::Game;
use crate::player::{Player, gain_unit, remove_unit};
use crate::playing_actions::{ActionCost, PlayingActionType};
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
        ActionCost::free(),
        |game, p, _| any_barbarian_city_can_be_influenced(game, p),
    )
    .add_unit_type_request(
        |event| &mut event.play_action_card,
        1,
        |game, player, a| {
            if let Some(position) = a.selected_position {
                //set in use_cultural_takeover

                let barbarians = get_barbarians_player(game);
                let b = barbarians.index;
                let units = barbarians
                    .get_units(position)
                    .iter()
                    .map(|u| u.id)
                    .collect_vec();
                let len = units.len();
                for id in units {
                    remove_unit(b, id, game);
                }
                if len > 0 {
                    let p = player.get_mut(game);
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
                        player.index,
                        &format!("Select unit to gain at {position}"),
                    ));
                }
            }
            None
        },
        |game, s, a| {
            s.log(
                game,
                &format!("Selected unit to gain: {}", s.choice.non_leader_name(),),
            );
            gain_unit(
                s.player_index,
                a.selected_position.expect("unit position"),
                s.choice,
                game,
            );
        },
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.play_action_card,
        0,
        |game, p, a| {
            if a.selected_position.is_none() {
                // skip this the second time where we only select a unit type to add
                game.permanent_effects
                    .push(PermanentEffect::CulturalTakeover);
                p.log(game, "You may influence Barbarian cities of size 1.");
            }
        },
    )
    .tactics_card(tactics_card)
    .build()
}

fn any_barbarian_city_can_be_influenced(game: &Game, p: &Player) -> bool {
    let influence = available_influence_actions(game, p.index);
    if influence.is_empty() {
        return false;
    }
    get_barbarians_player(game).cities.iter().any(|c| {
        c.size() == 1
            && influence.iter().any(|i| {
                influence_culture_boost_cost(
                    game,
                    p.index,
                    &SelectedStructure::new(c.position, Structure::CityCenter),
                    i,
                    true,
                    true,
                )
                .is_ok()
            })
    })
}

pub(crate) fn use_cultural_takeover() -> Ability {
    Ability::builder("cultural_takeover", "-")
        .add_transient_event_listener(
            |event| &mut event.is_playing_action_available,
            3,
            |available, game, t, p| {
                if game
                    .permanent_effects
                    .contains(&PermanentEffect::CulturalTakeover)
                    && !is_base_or_modifier(
                        t,
                        p.get(game),
                        &PlayingActionType::InfluenceCultureAttempt,
                    )
                {
                    *available =
                        Err("Cultural Takeover: You may only influence culture.".to_string());
                }
            },
        )
        .add_transient_event_listener(
            |event| &mut event.on_influence_culture_attempt,
            5,
            |c, _, game, _| {
                if let Ok(i) = c {
                    if matches!(i.structure, Structure::CityCenter)
                        && !(is_barbarian_takeover(game, i) || i.barbarian_takeover_check)
                    {
                        *c = Err("City center can't be influenced".to_string());
                    }
                }
            },
        )
        .add_transient_event_listener(
            |event| &mut event.on_influence_culture_resolve,
            1,
            |game, outcome, (), _| {
                if remove_element(
                    &mut game.permanent_effects,
                    &PermanentEffect::CulturalTakeover,
                )
                .is_some_and(|_| outcome.success)
                {
                    let mut info = ActionCardInfo::new(15, None, None);
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
