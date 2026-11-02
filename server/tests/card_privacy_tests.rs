use server::action::Action;
use server::advance::Advance;
use server::cache::Cache;
use server::card::{HandCard, HandCardLocation};
use server::city::City;
use server::city_pieces::Building;
use server::construct::Construct;
use server::content::persistent_events::{
    EventResponse, PersistentEventRequest, PersistentEventType,
};
use server::events::EventOrigin;
use server::game::{Game, GameContext};
use server::game_api;
use server::game_data::GameData;
use server::leader::Leader;
use server::log::{ActionLogAction, ActionLogEntry, ActionLogItem};
use server::map::Terrain;
use server::movement::{MoveUnits, MovementAction};
use server::player::Player;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::unit::UnitType;
use server::wonder::Wonder;

fn fixture(name: &str) -> Game {
    let path = format!(
        "{}/tests/test_games/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let data: GameData = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let cache = Cache::new(&data.options);
    Game::from_data(data, cache, GameContext::Play)
}

fn add_mausoleum(game: &mut Game, player: usize) {
    game.players[player].cities[0]
        .pieces
        .wonders
        .push(Wonder::GreatMausoleum);
    game.players[player]
        .wonders_owned
        .insert(Wonder::GreatMausoleum);
}

fn move_to(destination: &str) -> Action {
    Action::Movement(MovementAction::Move(MoveUnits::new(
        vec![0],
        Position::from_offset(destination),
        None,
        ResourcePile::empty(),
    )))
}

fn select_card(id: u8) -> Action {
    Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
        id,
    )]))
}

fn actions(game: &Game) -> impl Iterator<Item = &ActionLogAction> {
    game.log.iter().flat_map(|age| {
        age.rounds
            .iter()
            .flat_map(|round| round.turns.iter().flat_map(|turn| &turn.actions))
    })
}

fn transfers(game: &Game) -> Vec<(HandCard, HandCardLocation, HandCardLocation)> {
    actions(game)
        .flat_map(|action| &action.items)
        .filter_map(|item| match &item.entry {
            ActionLogEntry::HandCard { card, from, to } => {
                Some((card.clone(), from.clone(), to.clone()))
            }
            _ => None,
        })
        .collect()
}

fn commitments(game: &Game) -> (Option<u8>, Option<u8>) {
    let round = game
        .events
        .iter()
        .rev()
        .find_map(|event| match &event.event_type {
            PersistentEventType::CombatRoundStart(round) => Some(round),
            _ => None,
        })
        .expect("pending combat round");
    (
        round.attacker_strength.tactics_card,
        round.defender_strength.tactics_card,
    )
}

fn round_phase(game: &Game) -> String {
    let round = game
        .events
        .iter()
        .rev()
        .find_map(|event| match &event.event_type {
            PersistentEventType::CombatRoundStart(round) => Some(round),
            _ => None,
        })
        .expect("pending combat round");
    serde_json::to_value(&round.phase)
        .unwrap()
        .as_str()
        .unwrap()
        .to_string()
}

// Check the actual wire representation, not just the strength field. Card IDs
// can also occur in hands/piles, event choices/responses/origins and log entries.
fn assert_no_serialized_card_identity(game: &Game, card: u8) {
    fn visit(value: &serde_json::Value, path: &str, card: u8, names: &[&str]) {
        match value {
            serde_json::Value::Object(fields) => {
                for (key, value) in fields {
                    let path = format!("{path}.{key}");
                    if matches!(
                        key.as_str(),
                        "ActionCard" | "TacticsCard" | "CivilCard" | "tactics_card"
                    ) {
                        assert_ne!(value.as_u64(), Some(u64::from(card)), "{path}");
                    }
                    if matches!(
                        key.as_str(),
                        "action_cards"
                            | "action_cards_left"
                            | "action_cards_discarded"
                            | "tactics_cards"
                            | "ignored_tactics_cards"
                    ) && let Some(ids) = value.as_array()
                    {
                        assert!(
                            !ids.contains(&serde_json::json!(card)),
                            "card {card} leaked at {path}: {ids:?}"
                        );
                    }
                    visit(value, &path, card, names);
                }
            }
            serde_json::Value::Array(values) => {
                for (i, value) in values.iter().enumerate() {
                    visit(value, &format!("{path}[{i}]"), card, names);
                }
            }
            serde_json::Value::String(text) => {
                for name in names {
                    assert!(!text.contains(name), "{name} leaked at {path}: {text}");
                }
            }
            _ => {}
        }
    }
    let card_info = game.cache.get_action_card(card);
    visit(
        &serde_json::to_value(game.cloned_data()).unwrap(),
        "$",
        card,
        &[
            &card_info.civil_card.name,
            &game.cache.get_tactics_card(card).name,
        ],
    );
}

fn assert_serialized_discards(game: &Game, expected: &[u8]) {
    assert_eq!(game.action_cards_discarded, expected);
    assert_eq!(
        serde_json::to_value(game.cloned_data()).unwrap()["action_cards_discarded"],
        serde_json::json!(expected)
    );
}

fn assert_reveal(game: &Game, card: u8) {
    let expected = format!(
        "Reveal Tactics Card {}",
        game.cache.get_tactics_card(card).name
    );
    assert!(
        actions(game)
            .flat_map(|action| &action.items)
            .any(|item| { matches!(&item.entry, ActionLogEntry::Text(text) if text == &expected) })
    );
}

#[test]
fn attacker_commitment_is_private_until_scouts_reveals_the_cards() {
    let mut game = fixture("tactics_cards/scout");
    game.action_cards_discarded = vec![5, 9];
    game.players
        .push(Player::new(game.cache.get_civilization("Celts"), 2));
    let game = game_api::execute(game.clone(), move_to("C1"), 0);
    let game = game_api::execute(game, select_card(1), 0);
    assert_eq!(game.active_player(), 1);
    assert_eq!(commitments(&game).0, Some(1));
    assert_eq!(commitments(&game).1, None);
    assert_serialized_discards(&game, &[5, 9, 1]);
    let original = serde_json::to_value(game.cloned_data()).unwrap();

    for viewer in [Some(0), Some(1), Some(2), None] {
        let filtered = game_api::strip_secret(game.clone(), viewer);
        // Keep the commitment's presence, not its identity, for eligibility/status UI.
        assert_eq!(commitments(&filtered).0, Some(0));
        assert_eq!(commitments(&filtered).1, None);
        assert_serialized_discards(&filtered, &[5, 9, 0]);
        if viewer != Some(0) {
            assert_no_serialized_card_identity(&filtered, 1);
        }
        // Re-filtering an already private state must neither reveal anything
        // nor mask a legitimate public discard with the same sentinel.
        let twice = game_api::strip_secret(filtered.clone(), viewer);
        assert_serialized_discards(&twice, &[5, 9, 0]);
        assert_eq!(
            transfers(&filtered),
            vec![(
                HandCard::ActionCard(if viewer == Some(0) { 1 } else { 0 }),
                HandCardLocation::Hand(0),
                HandCardLocation::PlayToDiscardFaceDown,
            )],
            "face-down transfer for {viewer:?}"
        );
        let handler = filtered.current_event().player.handler.as_ref().unwrap();
        let PersistentEventRequest::SelectHandCards(request) = &handler.request else {
            panic!("defender should still choose a tactics card");
        };
        assert_eq!(
            request.choices,
            if viewer == Some(1) {
                vec![HandCard::ActionCard(23)]
            } else {
                vec![]
            }
        );
        for action in actions(&filtered) {
            assert!(action.undo.is_empty());
            if let Action::Response(EventResponse::SelectHandCards(cards)) = &action.action {
                assert!(cards.is_empty(), "raw response must not disclose a card");
            }
        }
    }

    assert_eq!(serde_json::to_value(game.cloned_data()).unwrap(), original);
    let filtered = game_api::strip_secret(game.clone(), Some(1));
    let view = server::web_view::view(&filtered, Some(1));
    assert_eq!(view["decision"]["opponentTactics"], "played");
    assert!(
        view["decision"]["options"]
            .as_array()
            .unwrap()
            .iter()
            .any(|option| option["value"]["ActionCard"] == 23)
    );
    // A filtered combat preview must not try to look up the discarded sentinel
    // as a real card. The authoritative game remains the only resolution input.
    let estimate = server::web_view::query(
        &filtered,
        1,
        serde_json::json!({
            "kind": "combatEstimate",
            "action": Action::Response(EventResponse::SelectHandCards(vec![])),
            "samples": 1,
        }),
    )
    .expect("filtered combat estimate");
    assert_eq!(estimate["samples"], 1);
    let scouts_estimate = server::web_view::query(
        &filtered,
        1,
        serde_json::json!({
            "kind": "combatEstimate",
            "action": select_card(23),
            "samples": 1,
        }),
    );
    assert_eq!(
        scouts_estimate.unwrap_err(),
        "Estimate unavailable for Scout"
    );

    let revealed = game_api::execute(game, select_card(23), 1);
    for viewer in [Some(0), Some(1), Some(2), None] {
        let filtered = game_api::strip_secret(revealed.clone(), viewer);
        assert_serialized_discards(&filtered, &[5, 9, 1, 23]);
        assert_reveal(&filtered, 1);
        assert_reveal(&filtered, 23);
    }
}

#[test]
fn loyalty_defender_commitment_is_private_before_attacker_chooses() {
    // Mirror the live Viriatus/Tactical Retreat fixture, but leave the human
    // attacker's card pending so Loyalty cannot immediately reveal its card.
    let mut game = fixture("civilizations/china/fast_war");
    game.action_cards_discarded = vec![5, 9];
    game.current_player_index = 1;
    game.players[0].civilization = game.cache.get_civilization("Celts");
    game.players[0].action_cards = vec![17];
    game.players[0].units[0].unit_type = UnitType::Leader(Leader::Viriatus);
    game.players[0].units[0].position = Position::from_offset("E7");
    game.players[0].units[0].id = 42;
    game.players[0].next_unit_id = 43;
    for terrain in game.map.tiles.values_mut() {
        *terrain = Terrain::Fertile;
    }
    game.players[1].cities = vec![City::new(1, Position::from_offset("D1"))];
    game.players[1].units[0].position = Position::from_offset("E8");
    game.players[1].advances.insert(Advance::Tactics);
    game.players[1].action_cards = vec![1];
    let mut barbarians = Player::new(game.cache.get_civilization("Barbarians"), 2);
    let mut unit = game.players[1].units[0].clone();
    unit.player_index = 2;
    unit.position = Position::from_offset("D8");
    barbarians.units = vec![unit];
    barbarians.next_unit_id = 1;
    game.players.push(barbarians);
    game.players
        .push(Player::new(game.cache.get_civilization("Greece"), 3));

    let game = game_api::execute(game.clone(), move_to("D8"), 1);
    assert_eq!(game.active_player(), 0);
    let game = game_api::execute(game, select_card(17), 0);
    assert_eq!(game.active_player(), 1);
    assert_eq!(round_phase(&game), "Default");
    assert_eq!(commitments(&game).0, None);
    assert_eq!(commitments(&game).1, Some(17));
    assert_serialized_discards(&game, &[5, 9, 17]);
    let original = serde_json::to_value(game.cloned_data()).unwrap();

    for viewer in [Some(0), Some(1), Some(2), Some(3), None] {
        let filtered = game_api::strip_secret(game.clone(), viewer);
        assert_eq!(commitments(&filtered).0, None);
        assert_eq!(commitments(&filtered).1, Some(0));
        assert_serialized_discards(&filtered, &[5, 9, 0]);
        if viewer != Some(0) {
            // Includes the Barbarian combatant: only its Celtic controller owns
            // the committed hand card and may see its face-down transfer.
            assert_no_serialized_card_identity(&filtered, 17);
        }
        assert_eq!(
            transfers(&filtered),
            vec![(
                HandCard::ActionCard(if viewer == Some(0) { 17 } else { 0 }),
                HandCardLocation::Hand(0),
                HandCardLocation::PlayToDiscardFaceDown,
            )],
            "the Celtic hand owner, not the Barbarian participant, knows the card"
        );
    }

    // Match Great Mausoleum's front insertion when the *controller* owns it;
    // the Barbarian combatant does not. An older public occurrence stays public.
    let mut mausoleum = game.clone();
    add_mausoleum(&mut mausoleum, 0);
    mausoleum.action_cards_discarded = vec![17, 5, 9, 17];
    for viewer in [Some(0), Some(1), Some(2), Some(3), None] {
        let filtered = game_api::strip_secret(mausoleum.clone(), viewer);
        assert_serialized_discards(&filtered, &[0, 5, 9, 17]);
    }

    assert_eq!(serde_json::to_value(game.cloned_data()).unwrap(), original);
    let revealed = game_api::execute(game, select_card(1), 1);
    assert_eq!(
        revealed.active_player(),
        0,
        "Loyalty controller chooses retreat"
    );
    assert_eq!(round_phase(&revealed), "TacticsCard");
    for viewer in [Some(0), Some(1), Some(2), Some(3), None] {
        let filtered = game_api::strip_secret(revealed.clone(), viewer);
        assert_eq!(commitments(&filtered).0, Some(1));
        assert_eq!(commitments(&filtered).1, Some(17));
        assert_serialized_discards(&filtered, &[5, 9, 17, 1]);
        assert_reveal(&filtered, 1);
        assert_reveal(&filtered, 17);
    }
}

#[test]
fn masking_preserves_public_discard_duplicates_and_mausoleum_insertion_order() {
    for mausoleum in [false, true] {
        let mut game = fixture("tactics_cards/scout");
        // Reused cards can leave earlier public occurrences in the discard list.
        game.action_cards_discarded = vec![1, 5, 9];
        if mausoleum {
            add_mausoleum(&mut game, 0);
        }
        let game = game_api::execute(game, move_to("C1"), 0);
        let game = game_api::execute(game, select_card(1), 0);
        let expected = if mausoleum {
            vec![0, 1, 5, 9]
        } else {
            vec![1, 5, 9, 0]
        };
        for viewer in [Some(0), Some(1), None] {
            let filtered = game_api::strip_secret(game.clone(), viewer);
            assert_serialized_discards(&filtered, &expected);
            assert_eq!(commitments(&filtered).0, Some(0));
            // Re-filtering must not hide the earlier public occurrence as well.
            let filtered = game_api::strip_secret(filtered, viewer);
            assert_serialized_discards(&filtered, &expected);
        }
    }
}

#[test]
fn ordinary_observatory_draw_is_visible_only_to_the_owner() {
    let mut game = fixture("base/construct");
    game.action_cards_left = vec![17];
    let game = game_api::execute(
        game,
        Action::Playing(PlayingAction::Construct(Construct::new(
            Position::from_offset("C2"),
            Building::Observatory,
            ResourcePile::new(1, 1, 1, 0, 0, 0, 0),
        ))),
        0,
    );
    assert!(transfers(&game).contains(&(
        HandCard::ActionCard(17),
        HandCardLocation::DrawPile,
        HandCardLocation::Hand(0),
    )));
    for viewer in [Some(0), Some(1), None] {
        let filtered = game_api::strip_secret(game.clone(), viewer);
        let draws: Vec<_> = transfers(&filtered)
            .into_iter()
            .filter(|(_, from, _)| from == &HandCardLocation::DrawPile)
            .collect();
        assert_eq!(
            draws,
            vec![(
                HandCard::ActionCard(if viewer == Some(0) { 17 } else { 0 }),
                HandCardLocation::DrawPile,
                HandCardLocation::Hand(0),
            )]
        );
    }
}

fn game_with_transfer(card: HandCard, from: HandCardLocation, to: HandCardLocation) -> Game {
    let mut game = fixture("tactics_cards/scout");
    game.players
        .push(Player::new(game.cache.get_civilization("Celts"), 2));
    let mut action = ActionLogAction::new(Action::StartTurn, 0, None, 0);
    action.items.push(ActionLogItem::new(
        0,
        ActionLogEntry::hand_card(card, from, to),
        EventOrigin::Advance(Advance::Tactics),
        vec![],
    ));
    game.log[0].rounds[0].turns[0].actions.push(action);
    game.log_index = 1;
    game
}

#[test]
fn hidden_draws_and_transfers_require_an_actual_hand_owner() {
    use HandCardLocation::{DrawPile, DrawPilePeeked, GreatSeer, Hand, PlayToDiscardFaceDown};
    for (card, hidden) in [
        (HandCard::ActionCard(17), HandCard::ActionCard(0)),
        (HandCard::ObjectiveCard(1), HandCard::ObjectiveCard(0)),
        (
            HandCard::Wonder(Wonder::Pyramids),
            HandCard::Wonder(Wonder::Hidden),
        ),
    ] {
        for (from, to, owners) in [
            (DrawPile, Hand(0), vec![0]),
            (Hand(0), Hand(1), vec![0, 1]),
            (Hand(0), DrawPile, vec![0]),
            (Hand(0), PlayToDiscardFaceDown, vec![0]),
            (DrawPile, GreatSeer(0), vec![]),
            (DrawPilePeeked(0), DrawPile, vec![]),
        ] {
            let game = game_with_transfer(card.clone(), from.clone(), to.clone());
            for viewer in [Some(0), Some(1), Some(2), None] {
                let filtered = game_api::strip_secret(game.clone(), viewer);
                let expected = if viewer.is_some_and(|p| owners.contains(&p)) {
                    &card
                } else {
                    &hidden
                };
                assert_eq!(
                    transfers(&filtered),
                    vec![(expected.clone(), from.clone(), to.clone())],
                    "{card:?}: {from:?} -> {to:?} for {viewer:?}"
                );
            }
        }
    }
}

#[test]
fn public_card_transfers_keep_their_identity_for_every_viewer() {
    use HandCardLocation::{DiscardPile, Hand, PlayToDiscard, Public, RevealedHand};
    for card in [
        HandCard::ActionCard(17),
        HandCard::ObjectiveCard(1),
        HandCard::Wonder(Wonder::Pyramids),
    ] {
        for (from, to) in [
            (Hand(0), DiscardPile),
            (DiscardPile, Hand(0)),
            (Hand(0), PlayToDiscard),
            (Public, Hand(0)),
            (RevealedHand(0), Hand(1)),
        ] {
            let game = game_with_transfer(card.clone(), from.clone(), to.clone());
            for viewer in [Some(0), Some(1), Some(2), None] {
                let filtered = game_api::strip_secret(game.clone(), viewer);
                assert_eq!(
                    transfers(&filtered),
                    vec![(card.clone(), from.clone(), to.clone())],
                    "{card:?}: {from:?} -> {to:?} for {viewer:?}"
                );
            }
        }
    }
}
