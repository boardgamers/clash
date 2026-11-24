use crate::action::Action;
use crate::card::HandCard;
use crate::combat_listeners::{CombatResult, on_end_combat};
use crate::combat_stats::{Battleground, CombatPlayerStats, CombatStats};
use crate::content::persistent_events::{
    EventResponse, PersistentEventRequest, PersistentEventType,
};
use crate::game::{Game, GameContext, GameOptions, GameVariant};
use crate::game_setup::{GameSetupBuilder, setup_game};
use crate::log::{ActionLogAction, current_turn_log_mut};
use crate::objective_card::present_objective_cards;
use crate::playing_actions::PlayingAction;
use crate::position::Position;
use crate::unit::Units;

const CARDS: [u8; 2] = [23, 25];

fn game(card: u8) -> Game {
    let mut game = setup_game(
        &GameSetupBuilder::new(2)
            .seed("aggressor-printed-rule".into())
            .assigned_civilizations(vec!["Rome".into(), "Greece".into()])
            .build(),
    );
    for player in &mut game.players {
        player.objective_cards.clear();
        player.action_cards.clear();
    }
    game.players[0].objective_cards = vec![card];
    game.current_player_index = 0;
    game.starting_player_index = 0;
    reload(game)
}

fn infantry(n: u8) -> Units {
    Units {
        infantry: n,
        ..Units::empty()
    }
}

fn stats(
    attacking: bool,
    opponent: usize,
    result: CombatResult,
    losses: Units,
    ground: Battleground,
) -> CombatStats {
    let own = CombatPlayerStats::new(0, infantry(3), Position::from_offset("C2"));
    let mut other = CombatPlayerStats::new(opponent, infantry(3), Position::from_offset("C1"));
    other.losses = losses;
    let (attacker, defender) = if attacking {
        (own, other)
    } else {
        (other, own)
    };
    CombatStats::new(ground, false, attacker, defender, Some(result), None)
}

fn finish(mut game: Game, stats: CombatStats) -> Game {
    current_turn_log_mut(&mut game)
        .actions
        .push(ActionLogAction::new(
            Action::Playing(PlayingAction::EndTurn),
            0,
            None,
            0,
        ));
    game.log_index += 1;
    on_end_combat(&mut game, stats);
    while let Some(handler) = game.current_event_handler() {
        let PersistentEventRequest::ResourceReward(request) = &handler.request else {
            break;
        };
        let response = Action::Response(EventResponse::ResourceReward(
            request.reward.payment_options.default.clone(),
        ));
        let player = game.active_player();
        game = crate::game_api::execute(game, response, player);
    }
    game
}

fn offered(game: &Game) -> bool {
    game.events.iter().any(|event| matches!(&event.event_type,
        PersistentEventType::SelectObjectives(info) if info.shown_objective.as_deref() == Some("Aggressor")))
}

fn reload(game: Game) -> Game {
    let data = serde_json::from_str(&serde_json::to_string(&game.cloned_data()).unwrap()).unwrap();
    Game::from_data(data, game.cache.clone(), GameContext::Play)
}

fn claim(game: Game, card: u8) -> Game {
    assert!(offered(&game));
    let PersistentEventRequest::SelectHandCards(request) =
        &game.current_event_handler().unwrap().request
    else {
        panic!("expected objective choice");
    };
    assert!(request.choices.contains(&HandCard::ObjectiveCard(card)));
    let game = crate::game_api::execute(
        game,
        Action::Response(EventResponse::SelectHandCards(vec![
            HandCard::ObjectiveCard(card),
        ])),
        0,
    );
    assert_eq!(game.players[0].completed_objectives.len(), 1);
    assert_eq!(game.players[0].completed_objectives[0].name, "Aggressor");
    assert_eq!(game.players[0].completed_objectives[0].card, card);
    assert!(!game.players[0].objective_cards.contains(&card));
    game
}

#[test]
fn aggressor_offers_and_claims_attacker_wins_losses_and_draws() {
    for card in CARDS {
        for result in [
            CombatResult::AttackerWins,
            CombatResult::DefenderWins,
            CombatResult::Draw,
        ] {
            for ground in [
                Battleground::Land,
                Battleground::City,
                Battleground::CityWithFortress,
            ] {
                let game = finish(
                    game(card),
                    stats(true, 1, result.clone(), infantry(2), ground),
                );
                let game = claim(reload(game), card);
                assert_eq!(reload(game).players[0].completed_objectives.len(), 1);
            }
        }
    }
}

#[test]
fn aggressor_rejects_defenders_barbarians_insufficient_army_losses_and_sea() {
    for card in CARDS {
        for result in [
            CombatResult::AttackerWins,
            CombatResult::DefenderWins,
            CombatResult::Draw,
        ] {
            for battle in [
                stats(false, 1, result.clone(), infantry(2), Battleground::Land),
                stats(true, 2, result.clone(), infantry(2), Battleground::Land),
                stats(true, 1, result.clone(), infantry(1), Battleground::Land),
                stats(
                    true,
                    1,
                    result.clone(),
                    Units {
                        settlers: 2,
                        ..Units::empty()
                    },
                    Battleground::Land,
                ),
                stats(
                    true,
                    1,
                    result.clone(),
                    Units {
                        ships: 2,
                        ..Units::empty()
                    },
                    Battleground::Sea,
                ),
                stats(true, 1, result.clone(), infantry(2), Battleground::Sea),
            ] {
                let game = finish(game(card), battle);
                assert!(!offered(&game), "card {card}, result {result:?}");
                assert!(game.players[0].completed_objectives.is_empty());
            }
        }
    }
}

#[test]
fn aggressor_builder_filter_keeps_only_noncombat_halves() {
    let options = GameOptions {
        variant: GameVariant::Builder,
        ..GameOptions::default()
    };
    let cards = crate::content::objective_cards::get_all_uncached(&options);
    for card in CARDS {
        let game = game(card);
        let objective = game
            .cache
            .get_objective_card(card)
            .objectives
            .iter()
            .find(|o| o.name == "Aggressor")
            .unwrap();
        assert!(objective.requires_player_combat);
        assert!(objective.status_phase_check.is_none());
        assert_eq!(
            objective.description,
            "Immediately after eliminating at least 2 army units in a battle you started against another player."
        );
        let builder_card = cards.iter().find(|c| c.id == card).unwrap();
        assert_eq!(builder_card.objectives.len(), 1);
        assert_ne!(builder_card.objectives[0].name, "Aggressor");
    }
}

#[test]
fn aggressor_preserves_saved_legacy_pending_choices_and_completed_awards() {
    for card in CARDS {
        // Historical eligibility was recorded when the prompt was created. Loading
        // an already offered choice does not rerun the changed combat predicate.
        let mut game = game(card);
        present_objective_cards(&mut game, 0, vec!["Aggressor".into()]);
        let game = claim(reload(game), card);
        let game = reload(game);
        assert_eq!(game.players[0].completed_objectives[0].card, card);
        assert_eq!(game.players[0].completed_objectives[0].name, "Aggressor");
    }
}

#[test]
fn aggressor_actual_movement_battles_offer_and_claim_both_cards() {
    use crate::advance::Advance;
    use crate::map::{Map, Terrain};
    use crate::movement::{MoveUnits, MovementAction};
    use crate::resource_pile::ResourcePile;
    use crate::unit::{Unit, UnitType};
    for card in CARDS {
        for (attackers, defenders, expected) in [
            (3, 2, CombatResult::AttackerWins),
            (2, 3, CombatResult::DefenderWins),
            (2, 2, CombatResult::Draw),
        ] {
            let mut game = game(card);
            let start = Position::from_offset("C2");
            let end = Position::from_offset("C1");
            game.map = Map::new([(start, Terrain::Fertile), (end, Terrain::Fertile)].into());
            for player in &mut game.players {
                player.cities.clear();
                player.units.clear();
                player.advances = [Advance::Farming, Advance::Mining, Advance::Tactics].into();
            }
            for (player, count, position) in [(0, attackers, start), (1, defenders, end)] {
                game.players[player].units = (0..count)
                    .map(|id| Unit::new(player, position, UnitType::Infantry, id))
                    .collect();
                game.players[player].next_unit_id = count;
            }
            game.dice_roll_outcomes = vec![11; 10];
            let game = crate::game_api::execute(
                reload(game),
                Action::Movement(MovementAction::Move(MoveUnits::new(
                    (0..attackers).collect(),
                    end,
                    None,
                    ResourcePile::empty(),
                ))),
                0,
            );
            let turn = crate::log::current_turn_log_without_redo(&game);
            let stats = turn
                .actions
                .iter()
                .find_map(|a| a.combat_stats.as_ref())
                .unwrap();
            assert_eq!(stats.result, Some(expected));
            assert_eq!(stats.defender.losses.infantry, 2);
            claim(reload(game), card);
        }
    }
}

#[test]
fn aggressor_rules_boundary_preserves_legacy_and_switches_at_current_action() {
    for card in CARDS {
        for boundary in [None, Some(usize::MAX), Some(0)] {
            for (attacker, opponent, result, legacy, corrected) in [
                (false, 1, CombatResult::DefenderWins, true, false),
                (true, 2, CombatResult::AttackerWins, true, false),
                (true, 1, CombatResult::DefenderWins, false, true),
                (true, 1, CombatResult::Draw, false, true),
            ] {
                let mut game = game(card);
                game.aggressor_eligibility_from = boundary;
                let game = finish(
                    reload(game),
                    stats(attacker, opponent, result, infantry(2), Battleground::Land),
                );
                assert_eq!(
                    offered(&game),
                    if boundary == Some(0) {
                        corrected
                    } else {
                        legacy
                    }
                );
                if offered(&game) {
                    claim(reload(game), card);
                }
            }
        }
        // finish appends exactly one action. Equality must already use new rules.
        let mut game = game(card);
        game.aggressor_eligibility_from = Some(crate::game_api::log_length(&game) + 1);
        let game = finish(
            game,
            stats(true, 1, CombatResult::Draw, infantry(2), Battleground::Land),
        );
        claim(game, card);
    }
}

#[test]
fn aggressor_builder_metadata_does_not_change_legacy_deck_ids_or_rng() {
    // Both affected cards retain their other half. Filtering the added metadata
    // must not remove cards, reorder the deck, or consume additional shuffle RNG.
    let options = GameOptions {
        variant: GameVariant::Builder,
        ..GameOptions::default()
    };
    let cards = crate::content::objective_cards::get_all_uncached(&options);
    assert_eq!(
        cards.iter().map(|c| c.id).collect::<Vec<_>>(),
        (1..=38).collect::<Vec<_>>()
    );
    for (id, half) in [(23, "Seafarers"), (25, "Government")] {
        let card = cards.iter().find(|c| c.id == id).unwrap();
        assert_eq!(card.objectives.len(), 1);
        assert_eq!(card.objectives[0].name, half);
    }
}

#[test]
fn aggressor_legacy_saved_battle_claim_replays_and_new_branch_changes_eligibility() {
    use crate::advance::Advance;
    use crate::map::{Map, Terrain};
    use crate::movement::{MoveUnits, MovementAction};
    use crate::resource_pile::ResourcePile;
    use crate::unit::{Unit, UnitType};
    for card in CARDS {
        for defender_owner in [false, true] {
            let mut saved = game(card);
            let start = Position::from_offset("C2");
            let end = Position::from_offset("C1");
            saved.map = Map::new([(start, Terrain::Fertile), (end, Terrain::Fertile)].into());
            let (attacker, defender) = if defender_owner { (1, 0) } else { (0, 2) };
            saved.current_player_index = attacker;
            for player in &mut saved.players {
                player.cities.clear();
                player.units.clear();
                player.advances = [Advance::Farming, Advance::Mining, Advance::Tactics].into();
            }
            for (player, count, position) in [
                (attacker, if defender_owner { 2 } else { 3 }, start),
                (defender, if defender_owner { 3 } else { 2 }, end),
            ] {
                saved.players[player].units = (0..count)
                    .map(|id| Unit::new(player, position, UnitType::Infantry, id))
                    .collect();
                saved.players[player].next_unit_id = count;
            }
            saved.dice_roll_outcomes = vec![11; 10];
            saved.aggressor_eligibility_from = Some(usize::MAX);
            let movement = Action::Movement(MovementAction::Move(MoveUnits::new(
                (0..if defender_owner { 2 } else { 3 }).collect(),
                end,
                None,
                ResourcePile::empty(),
            )));
            // Synthetic saved-position replay, deliberately not a from-init fixture.
            let prefix =
                crate::game_api::execute(reload(saved.clone()), movement.clone(), attacker);
            let completed = claim(reload(prefix), card);
            // Defender completion hands control back to the attacker and seals undo.
            let redone = if defender_owner {
                completed
            } else {
                let undone = crate::game_api::execute(completed, Action::Undo, 0);
                assert!(offered(&undone));
                assert_eq!(undone.aggressor_eligibility_from, Some(usize::MAX));
                crate::game_api::execute(undone, Action::Redo, 0)
            };
            assert_eq!(redone.players[0].completed_objectives[0].name, "Aggressor");
            let replayed = claim(
                crate::game_api::execute(reload(saved.clone()), movement.clone(), attacker),
                card,
            );
            assert_eq!(
                replayed.players[0].completed_objectives,
                redone.players[0].completed_objectives
            );
            // Loading an unversioned old live save adopts the correction on its
            // first new action, unlike replaying already recorded old actions.
            saved.aggressor_eligibility_from = None;
            let branched = crate::game_api::execute(reload(saved), movement, attacker);
            assert!(!offered(&branched));
            assert!(branched.aggressor_eligibility_from.is_some());
        }
    }
}

#[test]
fn aggressor_legacy_builder_keeps_barbarian_awards_and_pending_claims() {
    for card in CARDS {
        let mut old = game(card);
        old.options.variant = GameVariant::Builder;
        old.cache = crate::cache::Cache::new(&old.options);
        old.aggressor_eligibility_from = None;
        let old = reload(old);
        assert!(
            old.cache
                .get_objective_card(card)
                .has_objective("Aggressor")
        );
        let pending = finish(
            old,
            stats(
                true,
                2,
                CombatResult::AttackerWins,
                infantry(2),
                Battleground::Land,
            ),
        );
        let completed = claim(reload(pending), card);
        assert_eq!(
            reload(completed).players[0].completed_objectives[0].name,
            "Aggressor"
        );
    }
}

#[test]
fn aggressor_legacy_builder_setup_preserves_exact_deal_and_rng() {
    let options = GameOptions {
        variant: GameVariant::Builder,
        ..GameOptions::default()
    };
    let setup = GameSetupBuilder::new(2)
        .options(options.clone())
        .seed("legacy-builder-aggressor".into())
        .build();
    let current = setup_game(&setup);
    let mut cache = crate::cache::Cache::new(&options);
    cache.retain_legacy_builder_aggressor();
    let legacy = crate::game_setup::setup_game_with_cache(&setup, cache);
    assert_eq!(
        serde_json::to_value(current.cloned_data()).unwrap(),
        serde_json::to_value(legacy.cloned_data()).unwrap()
    );
    assert_eq!(legacy.cache.get_objective_card(23).objectives.len(), 2);
    assert_eq!(current.cache.get_objective_card(23).objectives.len(), 1);
}
