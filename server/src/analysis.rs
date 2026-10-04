//! Independent, observation-based branches. Never reuse the source random stream.
use crate::action::Action;
use crate::card::{HandCard, HandCardLocation};
use crate::content::effects::PermanentEffect;
use crate::events::EventOrigin;
use crate::game::{Game, GameContext, GameState};
use crate::log::{
    ActionLogAction, ActionLogAge, ActionLogItem, ActionLogRound, ActionLogTurn, TurnType,
};
use crate::map::{BLOCKS, Block, Terrain, get_map_setup};
use crate::utils::{Rng, Shuffle};

/// Preserve public continuations. Private choices need separate reconstruction.
#[must_use]
pub fn can_create(game: &Game) -> bool {
    matches!(game.state, GameState::Playing | GameState::ChooseCivilization | GameState::Movement(_))
        && game.events.iter().all(public_continuation)
        && !game.permanent_effects.iter().any(|e| matches!(e, PermanentEffect::GreatSeer(_)))
        // Spy's prose records cannot reliably be mapped back to card identities.
        && game.players.iter().all(|p| p.secrets.is_empty())
        // These discard arrays are lifetime archives, not current-cycle piles.
        // Once a pile has refilled, their exclusions are no longer sufficient.
        && !game.log.iter().flat_map(|a| &a.rounds).flat_map(|r| &r.turns)
            .flat_map(|t| &t.actions).flat_map(|a| &a.items).any(|item| {
                matches!(&item.entry, crate::log::ActionLogEntry::Text(text) if text == "Reshuffling Action Card pile" || text == "Reshuffling Events pile")
            })
}

fn public_continuation(event: &crate::content::persistent_events::PersistentEventState) -> bool {
    use crate::content::persistent_events::{
        EventResponse, PersistentEventRequest as R, PersistentEventType as E,
    };
    use crate::playing_actions::PlayingAction;
    let public_payload = match &event.event_type {
        E::TurnStart
        | E::Advance(_)
        | E::Construct(_)
        | E::Recruit(_)
        | E::FoundCity(_)
        | E::Collect(_)
        | E::CityActivationMoodDecreased(_)
        | E::ShipConstructionConversion(_)
        | E::StopBarbarianMovement(_)
        | E::ReinforceBarbarians(_)
        | E::UnitsKilled(_)
        | E::InfluenceCultureBoost(_)
        | E::StatusPhase(_) => true,
        E::PayAction(payment) => !matches!(
            payment.action,
            PlayingAction::ActionCard(_) | PlayingAction::WonderCard(_)
        ),
        _ => false,
    };
    public_payload
        && event.player.handler.as_ref().is_none_or(|handler| {
            !matches!(
                handler.request,
                R::SelectHandCards(_) | R::ExploreResolution
            ) && !matches!(
                handler.response,
                Some(EventResponse::SelectHandCards(_) | EventResponse::ExploreResolution(_))
            )
        })
}

/// Build from public facts and the requesting seat's hand, never from the
/// composition or order of hidden source piles. The caller authenticates `seat`.
///
/// # Errors
/// Rejects unsupported pending choices, invalid seats and empty seeds.
pub fn create(mut game: Game, seat: Option<usize>, seed: &str) -> Result<Game, String> {
    if seed.is_empty() || seat.is_some_and(|s| s >= game.human_players_count()) {
        return Err("Analysis requires a seed and a valid requesting seat".into());
    }
    if !can_create(&game) {
        return Err("Analysis is unavailable during this private choice, Great Seer, remembered Spy information or after action/incident pile reshuffling".into());
    }
    // Hash all supplied UTF-8 bytes; no source seed, clock or source RNG involved.
    let value = seed
        .bytes()
        .fold(0x6c62_272e_07bb_0142_62b8_2175_6295_c58d_u128, |h, b| {
            (h ^ u128::from(b)).wrapping_mul(0x0000_0000_0100_0000_0000_0000_0000_013b)
        });
    game.rng = Rng::from_seed(value);
    seed.clone_into(&mut game.seed);
    game.context = GameContext::Play;
    game.messages.clear();
    game.board_history = Default::default();
    for player in &mut game.players {
        if Some(player.index) != seat {
            // A queued objective identifies a private card. Simulated opponents
            // receive different hands, so source-game claims cannot carry over.
            player.objective_opportunities.clear();
        }
    }
    game.dice_roll_outcomes.clear();
    game.dice_roll_log.clear();
    game.dropped_players.clear();
    game.custom_ui_elements.clear();

    let known = known_hands(&game, seat);

    let mut actions: Vec<_> = game.cache.get_action_cards().iter().map(|c| c.id).collect();
    let mut objectives: Vec<_> = game
        .cache
        .get_objective_cards()
        .iter()
        .map(|c| c.id)
        .collect();
    let mut wonders: Vec<_> = game.cache.get_wonders().iter().map(|c| c.wonder).collect();
    let mut incidents: Vec<_> = game.cache.get_incidents().iter().map(|c| c.id).collect();
    actions.retain(|c| !game.action_cards_discarded.contains(c));
    incidents.retain(|c| !game.incidents_discarded.contains(c));
    for p in &game.players {
        objectives.retain(|c| !p.completed_objectives.iter().any(|o| o.card == *c));
        wonders.retain(|c| !p.wonders_built.contains(c));
        if seat == Some(p.index) {
            actions.retain(|c| !p.action_cards.contains(c));
            objectives.retain(|c| !p.objective_cards.contains(c));
            wonders.retain(|c| !p.wonder_cards.contains(c));
        }
    }
    for cards in &known {
        for card in cards {
            match card {
                HandCard::ActionCard(id) => actions.retain(|c| c != id),
                HandCard::ObjectiveCard(id) => objectives.retain(|c| c != id),
                HandCard::Wonder(id) => wonders.retain(|c| c != id),
            }
        }
    }
    for e in &game.permanent_effects {
        if let PermanentEffect::PublicWonderCard(w) = e {
            wonders.retain(|c| c != w);
        }
    }
    actions.shuffle(&mut game.rng);
    objectives.shuffle(&mut game.rng);
    wonders.shuffle(&mut game.rng);
    incidents.shuffle(&mut game.rng);
    for p in &mut game.players {
        if seat != Some(p.index) {
            let mut known_actions = vec![];
            let mut known_objectives = vec![];
            let mut known_wonders = vec![];
            for card in &known[p.index] {
                match card {
                    HandCard::ActionCard(id) => known_actions.push(*id),
                    HandCard::ObjectiveCard(id) => known_objectives.push(*id),
                    HandCard::Wonder(id) => known_wonders.push(*id),
                }
            }
            p.action_cards = fill(&mut actions, p.action_cards.len(), known_actions)?;
            p.objective_cards = fill(&mut objectives, p.objective_cards.len(), known_objectives)?;
            p.wonder_cards = fill(&mut wonders, p.wonder_cards.len(), known_wonders)?;
        }
        p.secrets.clear();
    }
    game.action_cards_left = actions;
    game.objective_cards_left = objectives;
    game.wonders_left = wonders;
    for p in &game.players {
        for card in &p.action_cards {
            if *card >= crate::content::incidents::great_persons::GREAT_PERSON_OFFSET {
                incidents.retain(|i| {
                    *i != *card - crate::content::incidents::great_persons::GREAT_PERSON_OFFSET
                });
            }
        }
    }
    game.incidents_left = incidents;
    // Subtract the publicly revealed blocks from the published catalogue. Their
    // two legal orientations are equivalent; exhaustion retains base terrain.
    let mut blocks = remaining_blocks(&game)?;
    blocks.shuffle(&mut game.rng);
    for b in &mut game.map.unexplored_blocks {
        b.block = blocks.pop().ok_or("Too many unexplored map blocks")?;
    }
    retain_rule_facts(&mut game);
    remember_known_hands(&mut game, &known);
    // Objective listeners belong to the simulated cards, not the source hands.
    let cache = game.cache.clone();
    Ok(Game::from_data(game.data(), cache, GameContext::Play))
}

fn remember_known_hands(game: &mut Game, known: &[Vec<HandCard>]) {
    // Keep only observed hand constraints at the branch, so a reroll of this
    // sanitized snapshot retains the same knowledge without source history.
    if known.iter().any(|cards| !cards.is_empty()) {
        crate::log::add_log_action(game, Action::StartTurn);
        for (player, cards) in known.iter().enumerate() {
            for card in cards {
                crate::log::add_action_log_item(
                    game,
                    player,
                    crate::log::ActionLogEntry::HandCard {
                        card: card.clone(),
                        from: HandCardLocation::Public,
                        to: HandCardLocation::Hand(player),
                    },
                    EventOrigin::Ability("Analysis known card".into()),
                    vec![],
                );
            }
        }
        game.undo_limit = game.log_index;
    }
}

fn remaining_blocks(game: &Game) -> Result<Vec<Block>, String> {
    fn original(t: &Terrain) -> &Terrain {
        if let Terrain::Exhausted(inner) = t {
            original(inner)
        } else {
            t
        }
    }
    let mut blocks = BLOCKS.to_vec();
    for position in get_map_setup(game.human_players_count()).free_positions {
        if game
            .map
            .unexplored_blocks
            .iter()
            .any(|b| b.position.top_tile == position.top_tile)
        {
            continue;
        }
        let index = blocks
            .iter()
            .position(|block| {
                [position.rotation, (position.rotation + 3) % 6]
                    .iter()
                    .any(|rotation| {
                        block.tiles(&position, *rotation).iter().all(|(p, t)| {
                            game.map
                                .tiles
                                .get(p)
                                .is_some_and(|actual| original(actual) == t)
                        })
                    })
            })
            .ok_or("Public map does not match the standard tile catalogue")?;
        blocks.remove(index);
    }
    Ok(blocks)
}

fn fill<T>(pile: &mut Vec<T>, count: usize, mut known: Vec<T>) -> Result<Vec<T>, String> {
    let remaining = count
        .checked_sub(known.len())
        .ok_or("Known cards exceed hand size")?;
    known.extend(deal(pile, remaining)?);
    Ok(known)
}

/// Track only observable transfers. A hidden departure makes the previously
/// known cards of that type uncertain; never inspect its true identity.
fn known_hands(game: &Game, seat: Option<usize>) -> Vec<Vec<HandCard>> {
    let mut known = vec![vec![]; game.players.len()];
    let hand = |location: &HandCardLocation| match location {
        HandCardLocation::Hand(p) | HandCardLocation::RevealedHand(p) => Some(*p),
        _ => None,
    };
    for (ai, age) in game.log.iter().enumerate() {
        for (ri, round) in age.rounds.iter().enumerate() {
            for (ti, turn) in round.turns.iter().enumerate() {
                let current = ai + 1 == game.log.len()
                    && ri + 1 == age.rounds.len()
                    && ti + 1 == round.turns.len();
                let limit = if current {
                    game.log_index
                } else {
                    turn.actions.len()
                };
                for action in turn.actions.iter().take(limit) {
                    for item in &action.items {
                        if let crate::log::ActionLogEntry::HandCard { card, from, to } = &item.entry
                        {
                            let visible = from.is_public()
                                || to.is_public()
                                || seat
                                    .is_some_and(|s| hand(from) == Some(s) || hand(to) == Some(s));
                            if let Some(p) = hand(from) {
                                known[p].retain(|c: &HandCard| {
                                    if visible {
                                        c != card
                                    } else {
                                        c.card_type() != card.card_type()
                                    }
                                });
                            }
                            if visible
                                && let Some(p) = hand(to)
                                && !known[p].contains(card)
                            {
                                known[p].push(card.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    // The requester's actual current hand is authoritative for their observation.
    if let Some(p) = seat {
        known[p].clear();
    }
    for cards in &mut known {
        cards.sort();
    }
    known
}

fn deal<T>(pile: &mut Vec<T>, count: usize) -> Result<Vec<T>, String> {
    if count > pile.len() {
        return Err("Visible card counts exceed the card catalogue".into());
    }
    Ok(pile.split_off(pile.len() - count))
}

fn retain_rule_facts(game: &mut Game) {
    let mut round = ActionLogRound::new(game.round);
    // Objectives inspect the current round and current turn only. Flatten away
    // secret responses, prose, origin/modifier card IDs and undo/redo snapshots.
    if let Some(source) = game.log.last().and_then(|a| a.rounds.last()) {
        for (ti, source_turn) in source.turns.iter().enumerate() {
            let mut turn = ActionLogTurn::new(source_turn.turn_type.clone());
            let limit = if ti + 1 == source.turns.len() {
                game.log_index
            } else {
                source_turn.actions.len()
            };
            for a in source_turn.actions.iter().take(limit) {
                let mut fact = ActionLogAction::new(Action::StartTurn, a.player, None, 0);
                fact.combat_stats = a.combat_stats.clone().map(|mut s| {
                    s.claimed_action_cards.clear();
                    s.selected_card = None;
                    s
                });
                for i in &a.items {
                    if matches!(
                        &i.entry,
                        crate::log::ActionLogEntry::HandCard {
                            card: HandCard::Wonder(_),
                            to: HandCardLocation::PlayToKeep,
                            ..
                        } | crate::log::ActionLogEntry::HandCard {
                            to: HandCardLocation::CompleteObjective(_),
                            ..
                        }
                    ) {
                        fact.items.push(ActionLogItem::new(
                            i.player,
                            i.entry.clone(),
                            EventOrigin::Ability("Analysis".into()),
                            vec![],
                        ));
                    }
                }
                if fact.combat_stats.is_some() || !fact.items.is_empty() {
                    turn.actions.push(fact);
                }
            }
            round.turns.push(turn);
        }
    }
    if round.turns.is_empty() {
        round.turns.push(ActionLogTurn::new(TurnType::Player(
            game.current_player_index,
        )));
    }
    game.log_index = round.turns.last().expect("turn exists").actions.len();
    game.undo_limit = game.log_index;
    let mut age = ActionLogAge::new(game.age);
    age.rounds.push(round);
    game.log = vec![age];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::GameOptions;
    use crate::game_api;
    use crate::playing_actions::PlayingAction;

    fn copy(g: &Game) -> Game {
        Game::from_data(g.cloned_data(), g.cache.clone(), GameContext::Play)
    }
    fn value(g: &Game) -> serde_json::Value {
        serde_json::to_value(g.cloned_data()).unwrap()
    }

    #[test]
    fn independent_of_hidden_composition_seed_and_private_history() {
        for seat in [None, Some(0), Some(1)] {
            let source = game_api::init(2, "real-secret-seed".into(), GameOptions::default());
            let before = value(&source);
            let mut changed = copy(&source);
            changed.seed = "different-source-secret".into();
            changed.rng = Rng::from_seed(9283);
            changed.messages.push("private message".into());
            changed.dice_roll_outcomes = vec![1, 2, 3];
            changed.dice_roll_log = vec![4, 5];
            changed.custom_ui_elements.clear();
            for p in &mut changed.players {
                if seat != Some(p.index) && p.is_human() {
                    std::mem::swap(&mut p.action_cards[0], &mut changed.action_cards_left[0]);
                    std::mem::swap(
                        &mut p.objective_cards[0],
                        &mut changed.objective_cards_left[0],
                    );
                }
            }
            changed.action_cards_left.reverse();
            changed.objective_cards_left.reverse();
            changed.incidents_left.reverse();
            changed.wonders_left.reverse();
            for b in &mut changed.map.unexplored_blocks {
                b.block = BLOCKS[0].clone();
            }
            for age in &mut changed.log {
                for round in &mut age.rounds {
                    for turn in &mut round.turns {
                        for a in &mut turn.actions {
                            a.action = Action::Response(
                                crate::content::persistent_events::EventResponse::SelectHandCards(
                                    vec![HandCard::ActionCard(37)],
                                ),
                            );
                            a.items.push(ActionLogItem::new(
                                a.player,
                                crate::log::ActionLogEntry::Text("source secret".into()),
                                EventOrigin::Ability("test".into()),
                                vec![],
                            ));
                            a.undo = serde_json::from_value(serde_json::json!([{"op":"replace","path":"/seed","value":"undo secret"}])).unwrap();
                        }
                    }
                }
            }
            let a = create(copy(&source), seat, "independent fake seed").unwrap();
            let b = create(changed, seat, "independent fake seed").unwrap();
            assert_eq!(value(&a), value(&b));
            assert_eq!(value(&source), before);
            assert!(!a.can_undo());
            assert!(!a.can_redo());
            assert!(!value(&a).to_string().contains("real-secret-seed"));
            assert_eq!(a.map.tiles, source.map.tiles);
            if let Some(p) = seat {
                assert_eq!(a.players[p].action_cards, source.players[p].action_cards);
                assert_eq!(
                    a.players[p].objective_cards,
                    source.players[p].objective_cards
                );
            }
        }
    }

    #[test]
    fn branch_can_progress_for_each_seat_and_draw_real_simulated_cards() {
        let source = game_api::init(2, "source".into(), GameOptions::default());
        let mut game = create(source, Some(0), "fake").unwrap();
        for _ in 0..2 {
            let p = game.active_player();
            game = game_api::execute(game, Action::Playing(PlayingAction::EndTurn), p);
        }
        let p = game.active_player();
        let active = game.active_player();
        crate::log::add_start_turn_action_if_needed(&mut game, active);
        let player = crate::events::EventPlayer::new(p, EventOrigin::Ability("test".into()));
        let before = game.players[p].action_cards.len();
        crate::action_card::gain_action_card_from_pile(&mut game, &player);
        assert_eq!(game.players[p].action_cards.len(), before + 1);
        let mut ids = game.action_cards_left.clone();
        ids.extend(
            game.players
                .iter()
                .flat_map(|p| p.action_cards.iter().copied()),
        );
        let total = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), total);
        assert!(ids.iter().all(|id| *id != 0));
    }

    #[test]
    fn fails_closed_for_private_continuations() {
        let mut game = game_api::init(2, "source".into(), GameOptions::default());
        game.events.push(
            crate::content::persistent_events::PersistentEventState::new(
                0,
                crate::content::persistent_events::PersistentEventType::ChooseActionCard,
                None,
            ),
        );
        assert!(!can_create(&game));
        assert!(create(game, None, "fake").is_err());
    }

    #[test]
    fn undo_and_redo_only_restore_simulated_state() {
        let source = game_api::init(2, "source-secret".into(), GameOptions::default());
        let game = create(source, Some(0), "simulation-only").unwrap();
        let p = game.active_player();
        let position = game.players[p].cities[0].position;
        let hand = game.players[p].action_cards.clone();
        let action = Action::Playing(PlayingAction::Collect(crate::collect::Collect::new(
            position,
            vec![crate::collect::PositionCollection::new(
                position,
                crate::resource_pile::ResourcePile::food(1),
            )],
            crate::playing_actions::PlayingActionType::Collect,
        )));
        let game = game_api::execute(game, action, p);
        assert!(game.can_undo());
        let game = game_api::execute(game, Action::Undo, p);
        assert!(!game.can_undo());
        assert!(game.can_redo());
        assert_eq!(game.players[p].action_cards, hand);
        let game = game_api::execute(game, Action::Redo, p);
        assert_eq!(game.seed, "simulation-only");
        assert!(!value(&game).to_string().contains("source-secret"));
    }

    #[test]
    fn publicly_revealed_opponent_card_stays_in_their_hand() {
        let mut source = game_api::init(2, "source".into(), GameOptions::default());
        let id = source.players[1].action_cards[0];
        let active = source.active_player();
        crate::log::add_start_turn_action_if_needed(&mut source, active);
        crate::log::add_action_log_item(
            &mut source,
            1,
            crate::log::ActionLogEntry::HandCard {
                card: HandCard::ActionCard(id),
                from: HandCardLocation::Public,
                to: HandCardLocation::Hand(1),
            },
            EventOrigin::Ability("public transfer".into()),
            vec![],
        );
        for seat in [None, Some(0)] {
            let game = create(copy(&source), seat, "fake").unwrap();
            assert_eq!(game.players[1].action_cards, vec![id]);
            assert!(!game.action_cards_left.contains(&id));
            let rerolled = create(game, seat, "second simulation seed").unwrap();
            assert_eq!(rerolled.players[1].action_cards, vec![id]);
            assert!(!rerolled.action_cards_left.contains(&id));
            assert!(!rerolled.can_undo());
        }
    }

    #[test]
    fn revealed_and_exhausted_map_consumes_canonical_tiles() {
        let mut source = game_api::init(2, "source".into(), GameOptions::default());
        let revealed = source.map.unexplored_blocks.remove(0);
        let rotation = (revealed.position.rotation + 3) % 6;
        source
            .map
            .add_block_tiles(&revealed.position, &revealed.block, rotation);
        let pool = remaining_blocks(&source).unwrap();
        assert_eq!(pool.len(), BLOCKS.len() - 1);
        let position = revealed.block.tiles(&revealed.position, rotation)[0].0;
        let terrain = source.map.tiles.get_mut(&position).unwrap();
        *terrain = Terrain::Exhausted(Box::new(terrain.clone()));
        assert_eq!(remaining_blocks(&source).unwrap(), pool);
        let public_map = source.map.tiles.clone();
        let game = create(source, None, "fake").unwrap();
        assert_eq!(game.map.tiles, public_map);
        let mut remaining = pool;
        for b in &game.map.unexplored_blocks {
            let i = remaining
                .iter()
                .position(|block| block == &b.block)
                .unwrap();
            remaining.remove(i);
        }
    }

    #[test]
    fn fails_closed_after_action_pile_refill() {
        let mut game = game_api::init(2, "source".into(), GameOptions::default());
        let active = game.active_player();
        crate::log::add_start_turn_action_if_needed(&mut game, active);
        game.action_cards_left.clear();
        game.action_cards_discarded = game.cache.get_action_cards().iter().map(|c| c.id).collect();
        let p = game.active_player();
        let player = crate::events::EventPlayer::new(p, EventOrigin::Ability("test".into()));
        crate::action_card::gain_action_card_from_pile(&mut game, &player);
        assert!(!game.action_cards_left.is_empty());
        assert!(!can_create(&game));
        assert!(create(game, None, "fake").is_err());
    }
}
