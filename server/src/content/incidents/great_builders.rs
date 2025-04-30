use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::ActionCard;
use crate::card::HandCard;
use crate::construct::available_buildings;
use crate::content::builtin::Builtin;
use crate::content::effects::ConstructEffect;
use crate::content::effects::PermanentEffect::Construct;
use crate::content::incidents::great_persons::{
    GREAT_PERSON_DESCRIPTION, great_person_action_card, great_person_description,
};
use crate::content::persistent_events::HandCardsRequest;
use crate::game::Game;
use crate::player::Player;
use crate::playing_actions::{ActionCost, PlayingActionType};
use crate::resource_pile::ResourcePile;
use crate::utils::remove_element_by;
use crate::wonder::{
    Wonder, WonderCardInfo, WonderDiscount, cities_for_wonder, on_play_wonder_card,
};

pub(crate) fn great_engineer() -> ActionCard {
    let groups = &["Construction"];
    great_person_action_card(
        26,
        "Great Engineer",
        &format!(
            "{} Then, you may build a building in a city without spending an action and without activating it.",
            great_person_description(groups)
        ),
        ActionCost::regular(),
        groups,
        can_construct_anything,
    )
        .add_bool_request(
            |e| &mut e.play_action_card,
            0,
            |_, _, _| Some("Build a building in a city without spending an action and without activating it?".to_string()),
            |game, s, _| {
                if s.choice {
                    game.permanent_effects.push(Construct(ConstructEffect::GreatEngineer));
                    game.actions_left += 1; // to offset the action spent for building
                    game.add_info_log_item("Great Engineer: You may build a building in a city without \
                    spending an action and without activating it.");
                } else {
                    game.add_info_log_item("Great Engineer: You decided not to use the ability.");
                }
            },
        )
        .build()
}

pub(crate) fn can_construct_anything(game: &Game, p: &Player) -> bool {
    PlayingActionType::Construct
        .is_available(game, p.index)
        .is_ok()
        && p.cities
            .iter()
            .any(|city| !available_buildings(game, p.index, city.position).is_empty())
}

pub(crate) fn construct_only() -> Builtin {
    Builtin::builder("construct only", "-")
        .add_transient_event_listener(
            |event| &mut event.is_playing_action_available,
            2,
            |available, game, i| {
                if game
                    .permanent_effects
                    .iter()
                    .any(|e| matches!(e, &Construct(_)))
                    && !matches!(i.action_type, PlayingActionType::Construct)
                {
                    *available = Err("You may only construct buildings.".to_string());
                }
            },
        )
        .add_transient_event_listener(
            |event| &mut event.construct_cost,
            1,
            |c, _, game| {
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
            |game, _, _, _| {
                remove_element_by(&mut game.permanent_effects, |e| matches!(e, &Construct(_)));
            },
        )
        .build()
}

const ARCHITECT_DISCOUNT: WonderDiscount = WonderDiscount::new(true, 3);

pub(crate) fn great_architect() -> ActionCard {
    great_person_action_card::<_, String>(
        55,
        "Great Architect",
        &format!(
            "{GREAT_PERSON_DESCRIPTION} When constructing a wonder, you may ignore \
                the requirement advances (but not Engineering). \
                In addition, the cost of constructing the wonder is reduced by 3 culture tokens.",
        ),
        ActionCost::free(),
        &[],
        |game, player| !playable_wonders(game, player).is_empty(),
    )
    .add_hand_card_request(
        |e| &mut e.play_action_card,
        0,
        |game, player, _| {
            Some(HandCardsRequest::new(
                playable_wonders(game, game.player(player))
                    .iter()
                    .map(|name| HandCard::Wonder(*name))
                    .collect(),
                1..=1,
                "Great Architect: Select a wonder to build",
            ))
        },
        |game, s, _| {
            let HandCard::Wonder(name) = &s.choice[0] else {
                panic!("Invalid choice");
            };
            on_play_wonder_card(
                game,
                s.player_index,
                WonderCardInfo::new(*name, ARCHITECT_DISCOUNT),
            );
        },
    )
    .build()
}

fn playable_wonders(game: &Game, player: &Player) -> Vec<Wonder> {
    player
        .wonder_cards
        .iter()
        .filter(|name| !cities_for_wonder(**name, game, player, &ARCHITECT_DISCOUNT).is_empty())
        .copied()
        .collect()
}
