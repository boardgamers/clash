use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::ActionCard;
use crate::card::HandCard;
use crate::construct::available_buildings;
use crate::content::ability::Ability;
use crate::content::advances::AdvanceGroup;
use crate::content::effects::ConstructEffect;
use crate::content::effects::PermanentEffect::Construct;
use crate::content::incidents::great_persons::{
    GREAT_PERSON_DESCRIPTION, great_person_action_card, great_person_description,
};
use crate::content::persistent_events::HandCardsRequest;
use crate::game::Game;
use crate::player::Player;
use crate::player_events::CostInfo;
use crate::playing_actions::PlayingActionType;
use crate::resource_pile::ResourcePile;
use crate::utils::remove_element_by;
use crate::wonder::{Wonder, WonderCardInfo, cities_for_wonder, on_play_wonder_card, wonder_cost};

pub(crate) fn great_engineer() -> ActionCard {
    let groups = vec![AdvanceGroup::Construction];
    great_person_action_card(
        26,
        "Great Engineer",
        &format!(
            "{} Then, you may build a building in a city \
            without spending an action and without activating it.",
            great_person_description(&groups)
        ),
        |c| c.action().no_resources(),
        groups,
        can_construct_any_building,
    )
    .add_bool_request(
        |e| &mut e.play_action_card,
        0,
        |_, _, _| {
            Some(
                "Build a building in a city without spending an action and without activating it?"
                    .to_string(),
            )
        },
        |game, s, _| {
            if s.choice {
                game.permanent_effects
                    .push(Construct(ConstructEffect::GreatEngineer));
                game.actions_left += 1; // to offset the action spent for building
                s.log(
                    game,
                    "Great Engineer: You may build a building in a city without \
                    spending an action and without activating it.",
                );
            } else {
                s.log(game, "Great Engineer: You decided not to use the ability.");
            }
        },
    )
    .build()
}

pub(crate) fn can_construct_any_building(game: &Game, p: &Player) -> bool {
    PlayingActionType::Construct
        .is_available(game, p.index)
        .is_ok()
        && p.cities
            .iter()
            .any(|city| !available_buildings(game, p.index, city.position).is_empty())
}

pub(crate) fn construct_only() -> Ability {
    Ability::builder("Construct Only", "-")
        .add_transient_event_listener(
            |event| &mut event.is_playing_action_available,
            2,
            |available, game, t, _p| {
                if game
                    .permanent_effects
                    .iter()
                    .any(|e| matches!(e, &Construct(_)))
                    && t != &PlayingActionType::Construct
                {
                    *available = Err("You may only construct buildings.".to_string());
                }
            },
        )
        .add_transient_event_listener(
            |event| &mut event.building_cost,
            1,
            |c, _, game, _| {
                if game
                    .permanent_effects
                    .contains(&Construct(ConstructEffect::GreatEngineer))
                {
                    c.activate_city = false;
                }
                if game
                    .permanent_effects
                    .contains(&Construct(ConstructEffect::CityDevelopment))
                {
                    c.cost.default = ResourcePile::empty();
                }
            },
        )
        .add_simple_persistent_event_listener(
            |event| &mut event.construct,
            2,
            |game, _, _| {
                remove_element_by(&mut game.permanent_effects, |e| matches!(e, &Construct(_)));
            },
        )
        .build()
}

pub(crate) fn great_architect() -> ActionCard {
    great_person_action_card::<_>(
        55,
        "Great Architect",
        &format!(
            "{GREAT_PERSON_DESCRIPTION} When constructing a wonder, you may ignore \
                the requirement advances (but not Engineering). \
                In addition, the cost of constructing the wonder is reduced by 3 culture tokens.",
        ),
        |c| c.free_action().no_resources(),
        vec![],
        |game, player| !playable_wonders(game, player).is_empty(),
    )
    .add_hand_card_request(
        |e| &mut e.play_action_card,
        0,
        |game, player, _| {
            Some(HandCardsRequest::new(
                playable_wonders(game, player.get(game))
                    .iter()
                    .map(|name| HandCard::Wonder(*name))
                    .collect(),
                1..=1,
                "Great Architect: Select a wonder to build",
            ))
        },
        |game, s, _| {
            let HandCard::Wonder(w) = &s.choice[0] else {
                panic!("Invalid choice");
            };
            on_play_wonder_card(
                game,
                s.player_index,
                WonderCardInfo::new(
                    *w,
                    architect_wonder_cost(game, game.player(s.player_index), *w),
                ),
            );
        },
    )
    .build()
}

fn playable_wonders(game: &Game, player: &Player) -> Vec<Wonder> {
    player
        .wonder_cards
        .iter()
        .filter(|w| {
            !cities_for_wonder(**w, game, player, architect_wonder_cost(game, player, **w))
                .is_empty()
        })
        .copied()
        .collect()
}

fn architect_wonder_cost(game: &Game, player: &Player, w: Wonder) -> CostInfo {
    let mut info = wonder_cost(game, player, w);
    info.cost.default.culture_tokens -= 3;
    info.ignore_required_advances = true;
    info.ignore_action_cost = true; // we already paid for the action with the architect card
    info
}
