use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::gain_action;
use crate::action_card::ActionCard;
use crate::card::HandCard;
use crate::construct::{ConstructDiscount, available_buildings};
use crate::content::ability::Ability;
use crate::content::advances::AdvanceGroup;
use crate::content::effects::ConstructEffect;
use crate::content::effects::PermanentEffect::Construct;
use crate::content::incidents::great_persons::GreatPersonType;
use crate::content::incidents::great_persons::{great_person_card, tech_great_person_description};
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
    great_person_card(
        26,
        GreatPersonType::ActionCard,
        "Great Engineer",
        &format!(
            "{} Then, you may construct a building in one of your cities. \
            Pay its normal resource cost, without spending an additional action or activating the city.",
            tech_great_person_description(&groups)
        ),
        |c| c.action().no_resources(),
        groups,
        |game, p| can_construct_any_building(game, p, &[ConstructDiscount::NoCityActivation]),
    )
    .add_bool_request(
        |e| &mut e.play_action_card,
        0,
        |game, p, _| {
            // The card provides the construction action even when played for free at zero actions.
            let mut preview = game.clone();
            preview.actions_left = preview.actions_left.max(1);
            preview.events.clear();
            if !can_construct_any_building(&preview, p.get(game), &[ConstructDiscount::NoCityActivation]) {
                p.log(game, "Great Engineer: No building available to construct.");
                return None;
            }
            Some(
                "Construct in one of your cities at the normal resource cost, without an additional action or city activation?"
                    .to_string(),
            )
        },
        |game, s, _| {
            if s.choice {
                game.permanent_effects
                    .push(Construct(ConstructEffect::GreatEngineer));
                s.log(
                    game,
                    "Great Engineer: You may build a building in a city without \
                    spending an action and without activating it.",
                );
                gain_action(game, &s.player()); // to offset the action spent for building
            } else {
                s.log(game, "Great Engineer: You decided not to use the ability.");
            }
        },
    )
    .build()
}

pub(crate) fn can_construct_any_building(
    game: &Game,
    p: &Player,
    discounts: &[ConstructDiscount],
) -> bool {
    PlayingActionType::Construct
        .is_available(game, p.index)
        .is_ok()
        && p.cities
            .iter()
            .any(|city| !available_buildings(game, p.index, city.position, discounts).is_empty())
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
    great_person_card::<_>(
        55,
        GreatPersonType::Public,
        "Great Architect",
        "When constructing a wonder, you may ignore \
        the requirement advances (but not Engineering). \
        In addition, the cost of constructing the wonder is reduced by 3 culture tokens.",
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
                    s.origin.clone(),
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
    // The public card is a modifier, not payment for Construct Wonder. Only
    // recorded legacy plays retain the former action waiver. City-specific
    // exemptions (wonder experts) still apply in can_construct_wonder.
    if legacy_architect_play(game) {
        info.ignore_action_cost = true;
    }
    info
}

// A saved card-selection prompt belongs to its original play, even when its
// response is the first action executed by the updated engine. Later wonder
// prompts already persist their CostInfo, including the legacy waiver.
fn legacy_architect_play(game: &Game) -> bool {
    use crate::action::Action;
    use crate::content::persistent_events::PersistentEventType;
    use crate::playing_actions::PlayingAction;

    let pending = game.events.iter().any(|event| {
        matches!(&event.event_type, PersistentEventType::ActionCard(card) if card.id == 155)
    });
    let index = if pending {
        crate::log::linear_action_entries(game)
            .iter()
            .rposition(|entry| {
                matches!(
                    entry.action,
                    Action::Playing(PlayingAction::ActionCard(155))
                )
            })
            .map_or(0, |index| index + 1)
    } else if game.context == crate::game::GameContext::Replay {
        crate::game_api::log_length(game)
    } else {
        // A live eligibility preview always describes a new branch, even if
        // an undone legacy future still exists beyond the current cursor.
        return false;
    };
    game.great_architect_action_cost_from
        .is_some_and(|from| index < from)
        || (pending && game.great_architect_action_cost_from.is_none())
}

#[cfg(test)]
mod architect_tests {
    use super::*;
    use crate::action::{Action, try_execute_action};
    use crate::cache::Cache;
    use crate::content::persistent_events::EventResponse;
    use crate::game::GameContext;
    use crate::game_data::GameData;
    use crate::playing_actions::PlayingAction;
    use crate::position::Position;

    fn game() -> Game {
        let data: GameData = serde_json::from_str(include_str!(
            "../../../tests/test_games/incidents/great_persons/great_architect.json"
        ))
        .expect("valid Architect test fixture");
        let cache = Cache::new(&data.options);
        let mut game = Game::from_data(data, cache, GameContext::Play);
        game.players[0].action_cards = vec![155];
        game.actions_left = 3;
        game
    }

    fn play(game: Game) -> Game {
        crate::game_api::execute(game, Action::Playing(PlayingAction::ActionCard(155)), 0)
    }

    fn select(game: Game) -> Game {
        crate::game_api::execute(
            game,
            Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                "A1",
            )])),
            0,
        )
    }

    #[test]
    fn architect_charges_construction_once_and_keeps_discounts() {
        let game = game();
        let base = wonder_cost(&game, game.player(0), Wonder::Pyramids);
        let cost = architect_wonder_cost(&game, game.player(0), Wonder::Pyramids);
        assert_eq!(
            cost.cost.default.culture_tokens + 3,
            base.cost.default.culture_tokens
        );
        assert!(cost.ignore_required_advances);
        assert!(!cost.ignore_action_cost);
        let game = play(game);
        assert_eq!(game.actions_left, 3); // choosing the public modifier is free
        let game = select(game);
        assert_eq!(game.actions_left, 2);
        let game = crate::game_api::execute(
            game,
            Action::Response(EventResponse::Payment(vec![ResourcePile::new(
                2, 3, 6, 0, 1, 0, 2,
            )])),
            0,
        );
        assert_eq!(game.actions_left, 2);
        assert!(game.players[0].wonders_built.contains(&Wonder::Pyramids));
        assert!(!game.players[0].action_cards.contains(&155));
    }

    #[test]
    fn architect_unavailable_without_action_engineering_or_city() {
        for condition in 0..3 {
            let mut game = game();
            match condition {
                0 => game.actions_left = 0,
                1 => {
                    game.players[0]
                        .advances
                        .remove(crate::advance::Advance::Engineering);
                }
                _ => {
                    for city in &mut game.players[0].cities {
                        city.mood_state = crate::city::MoodState::Angry;
                    }
                }
            }
            assert!(
                PlayingActionType::ActionCard(155)
                    .is_available(&game, 0)
                    .is_err()
            );
            assert!(
                try_execute_action(
                    game.clone(),
                    Action::Playing(PlayingAction::ActionCard(155)),
                    0
                )
                .is_err()
            );
            assert!(game.players[0].action_cards.contains(&155));
        }
    }

    #[test]
    fn japan_cannot_use_shogunate_to_make_public_architect_free() {
        let mut game = game();
        game.players[0].civilization = game.cache.get_civilization("Japan");
        game.players[0]
            .special_advances
            .insert(crate::special_advance::SpecialAdvance::Shogunate);
        let mut game = game.clone();
        game.actions_left = 0;
        assert!(
            PlayingActionType::ActionCard(155)
                .is_available(&game, 0)
                .is_err()
        );
        game.actions_left = 3;
        let game = select(play(game));
        assert_eq!(game.actions_left, 2);
    }

    #[test]
    fn expert_city_retains_its_own_free_wonder_exemption() {
        for (civilization, leader) in [
            ("Babylonia", crate::leader::Leader::Nebuchadnezzar),
            ("China", crate::leader::Leader::Qin),
        ] {
            let mut game = game();
            game.players[0].civilization = game.cache.get_civilization(civilization);
            game.players[0].units[0].unit_type = crate::unit::UnitType::Leader(leader);
            game.players[0].units[0].position = Position::from_offset("A1");
            game.players[0].recruited_leaders = vec![leader];
            let mut game = game.clone();
            game.actions_left = 0;
            let cities = cities_for_wonder(
                Wonder::Pyramids,
                &game,
                game.player(0),
                architect_wonder_cost(&game, game.player(0), Wonder::Pyramids),
            );
            assert_eq!(cities, vec![Position::from_offset("A1")]);
            assert!(
                PlayingActionType::ActionCard(155)
                    .is_available(&game, 0)
                    .is_ok()
            );
            let game = play(game); // single city is selected automatically
            assert_eq!(game.actions_left, 0);
        }
    }

    #[test]
    fn saved_multiple_wonder_choices_keep_original_cost_policy() {
        for legacy in [false, true] {
            let mut game = game();
            game.players[0].wonder_cards.push(Wonder::GreatWall);
            game.players[0].resources = ResourcePile::new(7, 7, 7, 7, 7, 7, 7);
            if legacy {
                game.great_architect_action_cost_from = Some(usize::MAX);
            }
            let game = play(game);
            assert_eq!(game.actions_left, 3);
            let mut value =
                serde_json::to_value(game.cloned_data()).expect("valid Architect test fixture");
            if legacy {
                value
                    .as_object_mut()
                    .expect("valid Architect test fixture")
                    .remove("great_architect_action_cost_from");
            }
            let game = Game::from_data(
                serde_json::from_value(value).expect("valid Architect test fixture"),
                game.cache.clone(),
                GameContext::Play,
            );
            let game = crate::game_api::execute(
                game,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::Wonder(
                    Wonder::Pyramids,
                )])),
                0,
            );
            assert_eq!(game.actions_left, 3);
            let game = select(game);
            assert_eq!(game.actions_left, if legacy { 3 } else { 2 });
        }
    }

    #[test]
    fn legacy_pending_wonder_keeps_waiver_but_new_branch_uses_action() {
        let mut game = game();
        game.great_architect_action_cost_from = Some(usize::MAX);
        let game = play(game);
        let mut value =
            serde_json::to_value(game.cloned_data()).expect("valid Architect test fixture");
        value
            .as_object_mut()
            .expect("valid Architect test fixture")
            .remove("great_architect_action_cost_from");
        let data = serde_json::from_value(value).expect("valid Architect test fixture");
        let game = Game::from_data(data, game.cache.clone(), GameContext::Play);
        let game = select(game);
        assert_eq!(game.actions_left, 3);

        let mut game = self::game();
        game.great_architect_action_cost_from = Some(crate::game_api::log_length(&game) + 1);
        game.actions_left = 0;
        assert!(
            PlayingActionType::ActionCard(155)
                .is_available(&game, 0)
                .is_err()
        );
    }
}
