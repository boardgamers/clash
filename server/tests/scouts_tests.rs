use common::{JsonTest, move_action};
use server::action::Action;
use server::advance::Advance;
use server::card::HandCard;
use server::content::persistent_events::{
    EventResponse, PersistentEventRequest, PersistentEventType,
};
use server::game::Game;
use server::game_api;
use server::leader::Leader;
use server::map::Terrain;
use server::player::Player;
use server::position::Position;
use server::unit::{Unit, UnitType};
use std::panic::{AssertUnwindSafe, catch_unwind};

mod common;

const SCOUTS: u8 = 23;
const PELTASTS: u8 = 1;
const HIGH_MORALE: u8 = 6;

fn setup(attacker_cards: &[u8], defender_cards: &[u8]) -> Game {
    let mut game = JsonTest::new("tactics_cards").load_game("scout");
    game.players[0].action_cards = attacker_cards.to_vec();
    game.players[1].action_cards = defender_cards.to_vec();
    game
}

fn attack(game: Game) -> Game {
    game_api::execute(game, move_action(vec![0], Position::from_offset("C1")), 0)
}

fn choose(game: Game, player: usize, card: Option<u8>) -> Game {
    game_api::execute(
        game,
        Action::Response(EventResponse::SelectHandCards(
            card.into_iter().map(HandCard::ActionCard).collect(),
        )),
        player,
    )
}

macro_rules! round {
    ($game:expr) => {
        match &$game.current_event().event_type {
            PersistentEventType::CombatRoundStart(round) => round,
            _ => panic!("Expected combat round start"),
        }
    };
}

fn assert_status(game: &Game, player: usize, expected: &str) {
    assert_eq!(
        server::web_view::view(game, Some(player))["decision"]["opponentTactics"],
        expected
    );
    let filtered = game_api::strip_secret(game.clone(), Some(player));
    assert_eq!(
        server::web_view::view(&filtered, Some(player))["decision"]["opponentTactics"],
        expected
    );
}

fn request_cards(game: &Game) -> Vec<HandCard> {
    let PersistentEventRequest::SelectHandCards(request) =
        &game.current_event_handler().unwrap().request
    else {
        panic!("Expected tactics selection");
    };
    request.choices.clone()
}

fn view_cards(game: &Game, player: usize) -> Vec<u8> {
    server::web_view::view(game, Some(player))["decision"]["options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["value"]["ActionCard"].as_u64().unwrap() as u8)
        .collect()
}

fn restore(game: &Game) -> Game {
    let json = serde_json::to_string(&game.cloned_data()).unwrap();
    Game::from_data(
        serde_json::from_str(&json).unwrap(),
        game.cache.clone(),
        game.context.clone(),
    )
}

fn add_stale_scouts_choice(game: &mut Game) {
    let PersistentEventRequest::SelectHandCards(request) =
        &mut game.current_event_handler_mut().unwrap().request
    else {
        panic!("Expected tactics selection");
    };
    if !request.choices.contains(&HandCard::ActionCard(SCOUTS)) {
        request.choices.push(HandCard::ActionCard(SCOUTS));
    }
}

fn assert_stale_scouts_rejected(mut game: Game, player: usize) {
    add_stale_scouts_choice(&mut game);
    let game = restore(&game);
    assert!(request_cards(&game).contains(&HandCard::ActionCard(SCOUTS)));
    assert_eq!(view_cards(&game, player), vec![HIGH_MORALE]);
    assert_eq!(
        view_cards(&game_api::strip_secret(game.clone(), Some(player)), player),
        vec![HIGH_MORALE]
    );
    let Err(panic) = catch_unwind(AssertUnwindSafe(|| choose(game, player, Some(SCOUTS)))) else {
        panic!("A stale Scouts choice must fail authoritative validation");
    };
    let message = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied());
    assert_eq!(message, Some("Tactics card requirements are not met"));
}

fn npc_setup(civilization: &str, attacker_cards: &[u8]) -> Game {
    let mut game = setup(attacker_cards, &[]);
    // Human seats stay contiguous; neutral factions follow them.
    game.players[1].units.clear();
    game.players[1].cities = vec![server::city::City::new(1, Position::from_offset("B1"))];
    let mut npc = Player::new(game.cache.get_civilization(civilization), 2);
    npc.units.push(Unit::new(
        2,
        Position::from_offset("C1"),
        UnitType::Infantry,
        0,
    ));
    npc.next_unit_id = 1;
    npc.cities
        .push(server::city::City::new(2, Position::from_offset("C1")));
    game.players.push(npc);
    game
}

#[test]
fn scouts_not_offered_against_ordinary_barbarians_or_pirates_and_stale_choices_rejected() {
    for civilization in ["Barbarians", "Pirates"] {
        let mut game = npc_setup(civilization, &[SCOUTS, HIGH_MORALE]);
        if civilization == "Pirates" {
            // Exercise a real sea battle rather than merely relabeling a land army.
            let from = Position::from_offset("C2");
            let to = Position::from_offset("C1");
            game.map.tiles.insert(from, Terrain::Water);
            game.map.tiles.insert(to, Terrain::Water);
            game.players[0].units.truncate(1);
            for player in &mut game.players {
                player
                    .cities
                    .retain(|city| city.position != from && city.position != to);
                for unit in &mut player.units {
                    unit.unit_type = UnitType::Ship;
                }
            }
        }
        let game = attack(game.clone()); // rebuild the changed civilization's listeners
        assert_eq!(game.current_event().player.index, 0);
        assert_eq!(
            request_cards(&game),
            vec![HandCard::ActionCard(HIGH_MORALE)]
        );
        assert_eq!(view_cards(&game, 0), vec![HIGH_MORALE]);
        assert_status(&game, 0, "none");
        assert_stale_scouts_rejected(game, 0);
    }
}

#[test]
fn defending_scouts_not_offered_after_attacker_passes_or_has_no_tactics() {
    for attacker_cards in [vec![PELTASTS], vec![]] {
        let mut game = attack(setup(&attacker_cards, &[SCOUTS, HIGH_MORALE]));
        if !attacker_cards.is_empty() {
            game = choose(game, 0, None);
        }
        assert_eq!(game.current_event().player.index, 1);
        assert_eq!(round!(&game).attacker_strength.tactics_card, None);
        assert_eq!(
            request_cards(&game),
            vec![HandCard::ActionCard(HIGH_MORALE)]
        );
        assert_eq!(view_cards(&game, 1), vec![HIGH_MORALE]);
        assert_status(&game, 1, "none");
        assert_stale_scouts_rejected(game, 1);
    }
}

#[test]
fn attacking_scouts_remain_available_without_examining_the_defenders_hidden_hand() {
    for defender_cards in [vec![], vec![PELTASTS], vec![7]] {
        let game = attack(setup(&[SCOUTS, HIGH_MORALE], &defender_cards));
        assert_eq!(round!(&game).defender_strength.tactics_card, None);
        assert_eq!(
            request_cards(&game),
            vec![
                HandCard::ActionCard(HIGH_MORALE),
                HandCard::ActionCard(SCOUTS)
            ]
        );
        assert_eq!(view_cards(&game, 0), vec![HIGH_MORALE, SCOUTS]);
        assert_status(&game, 0, "pending");
        assert_eq!(
            view_cards(&game_api::strip_secret(game.clone(), Some(0)), 0),
            vec![HIGH_MORALE, SCOUTS]
        );
    }
}

#[test]
fn defending_scouts_see_a_just_played_card_and_steal_it_after_reveal() {
    let game = choose(attack(setup(&[PELTASTS], &[SCOUTS])), 0, Some(PELTASTS));
    // This request is built in the same event invocation as the attacker's
    // response. Eligibility must use the live round, not its stale saved copy.
    assert_eq!(request_cards(&game), vec![HandCard::ActionCard(SCOUTS)]);
    assert_eq!(round!(&game).attacker_strength.tactics_card, Some(PELTASTS));
    assert_status(&game, 1, "played");
    let mut filtered = game_api::strip_secret(game.clone(), Some(1));
    let PersistentEventType::CombatRoundStart(round) =
        &mut filtered.events.last_mut().unwrap().event_type
    else {
        unreachable!();
    };
    // Explicitly exercise the hidden-card sentinel as well as authoritative data.
    round.attacker_strength.tactics_card = Some(0);
    assert_eq!(view_cards(&filtered, 1), vec![SCOUTS]);
    let game = choose(restore(&game), 1, Some(SCOUTS));
    assert!(game.player(1).action_cards.contains(&PELTASTS));
    assert!(!game.player(1).action_cards.contains(&SCOUTS));
    let log = serde_json::to_string(&game.log).unwrap();
    assert!(log.contains("Ignore the enemy tactics"));
    assert!(!log.contains("Peltasts rolls"));
}

#[test]
fn attacking_scouts_can_steal_a_later_defender_card_or_risk_a_pass() {
    for defender_card in [Some(PELTASTS), None] {
        let game = choose(attack(setup(&[SCOUTS], &[PELTASTS])), 0, Some(SCOUTS));
        assert_eq!(game.current_event().player.index, 1);
        let game = choose(game, 1, defender_card);
        assert_eq!(
            game.player(0).action_cards.contains(&PELTASTS),
            defender_card.is_some()
        );
        assert!(!game.player(0).action_cards.contains(&SCOUTS));
        let log = serde_json::to_string(&game.log).unwrap();
        assert_eq!(
            log.contains("Cannot use - opponent didn't play a tactics card"),
            defender_card.is_none()
        );
    }
}

fn loyalty_setup(attacker_cards: &[u8], loyalty_cards: &[u8]) -> Game {
    let mut game = npc_setup("Barbarians", attacker_cards);
    let mut celts = Player::new(game.cache.get_civilization("Celts"), 1);
    celts.advances.insert(Advance::Tactics);
    celts.action_cards = loyalty_cards.to_vec();
    celts.cities = std::mem::take(&mut game.players[1].cities);
    celts.units.push(Unit::new(
        1,
        Position::from_offset("B2"),
        UnitType::Leader(Leader::Viriatus),
        0,
    ));
    celts.next_unit_id = 1;
    game.players[1] = celts;
    game.clone() // register Viriatus's listeners
}

#[test]
fn scouts_can_steal_a_barbarian_card_supplied_by_viriatus_but_not_after_loyalty_passes() {
    for loyalty_card in [Some(PELTASTS), None] {
        let game = attack(loyalty_setup(&[SCOUTS, HIGH_MORALE], &[PELTASTS]));
        assert_eq!(
            serde_json::to_value(&round!(&game).phase).unwrap(),
            "Loyalty"
        );
        assert_eq!(game.current_event().player.index, 1);
        assert_status(&game, 1, "pending");
        let game = choose(game, 1, loyalty_card);
        assert_eq!(
            serde_json::to_value(&round!(&game).phase).unwrap(),
            "Default"
        );
        assert_eq!(game.current_event().player.index, 0);
        assert_status(
            &game,
            0,
            if loyalty_card.is_some() {
                "played"
            } else {
                "none"
            },
        );
        if loyalty_card.is_some() {
            assert!(request_cards(&game).contains(&HandCard::ActionCard(SCOUTS)));
            assert_eq!(view_cards(&game, 0), vec![HIGH_MORALE, SCOUTS]);
            assert_eq!(
                view_cards(&game_api::strip_secret(game.clone(), Some(0)), 0),
                vec![HIGH_MORALE, SCOUTS]
            );
            let game = choose(restore(&game), 0, Some(SCOUTS));
            assert!(game.player(0).action_cards.contains(&PELTASTS));
            assert!(!game.player(1).action_cards.contains(&PELTASTS));
            assert!(
                serde_json::to_string(&game.log)
                    .unwrap()
                    .contains("Ignore the enemy tactics")
            );
        } else {
            assert_eq!(
                request_cards(&game),
                vec![HandCard::ActionCard(HIGH_MORALE)]
            );
            assert_stale_scouts_rejected(game, 0);
        }
    }
}

#[test]
fn loyalty_can_supply_defending_scouts_before_the_human_attacker_chooses() {
    let game = attack(loyalty_setup(&[PELTASTS], &[SCOUTS]));
    assert_eq!(
        serde_json::to_value(&round!(&game).phase).unwrap(),
        "Loyalty"
    );
    assert_eq!(round!(&game).attacker_strength.tactics_card, None);
    assert_eq!(request_cards(&game), vec![HandCard::ActionCard(SCOUTS)]);
    assert_eq!(view_cards(&game, 1), vec![SCOUTS]);
    assert_status(&game, 1, "pending");
    let game = choose(game, 1, Some(SCOUTS));
    assert_eq!(round!(&game).defender_strength.tactics_card, Some(SCOUTS));
    let game = choose(game, 0, Some(PELTASTS));
    assert!(
        serde_json::to_string(&game.log)
            .unwrap()
            .contains("Ignore the enemy tactics")
    );
}

#[test]
fn scouts_checker_uses_only_card_presence_and_public_choice_order_for_both_roles() {
    let base = attack(setup(&[SCOUTS], &[HIGH_MORALE]));
    for player in [0, 1] {
        let opponent = 1 - player;
        for civilization in ["Rome", "Barbarians", "Pirates"] {
            for phase in ["Loyalty", "Default"] {
                for card in [None, Some(0), Some(PELTASTS)] {
                    let mut game = base.clone();
                    game.players[opponent].civilization = game.cache.get_civilization(civilization);
                    let mut round = round!(&game).clone();
                    round.phase = serde_json::from_value(serde_json::json!(phase)).unwrap();
                    let strength = if opponent == 0 {
                        &mut round.attacker_strength
                    } else {
                        &mut round.defender_strength
                    };
                    strength.tactics_card = card;
                    let expected = card.is_some()
                        || (civilization == "Rome" && (phase == "Loyalty" || player == 0));
                    let checker = game
                        .cache
                        .get_tactics_card(SCOUTS)
                        .checker
                        .as_ref()
                        .unwrap();
                    assert_eq!(
                        checker(player, &game, &round),
                        expected,
                        "player={player}, opponent={civilization}, phase={phase:?}, card={card:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn scouts_respect_public_tactics_capability_including_a_borrowed_advance() {
    for borrowed in [false, true] {
        let mut game = setup(&[SCOUTS, HIGH_MORALE], &[]);
        game.players[1].advances.remove(Advance::Tactics);
        if borrowed {
            game.players[1].great_library_advance = Some(Advance::Tactics);
        }
        let game = attack(game.clone());
        assert_eq!(
            request_cards(&game).contains(&HandCard::ActionCard(SCOUTS)),
            borrowed
        );
        assert_status(&game, 0, if borrowed { "pending" } else { "none" });
        if !borrowed {
            assert_stale_scouts_rejected(game, 0);
        }
    }
}

#[test]
fn scouts_not_offered_when_sparta_publicly_denies_the_defenders_tactics() {
    let mut game = setup(&[SCOUTS, HIGH_MORALE], &[PELTASTS]);
    game.players[0].civilization = game.cache.get_civilization("Greece");
    game.players[0].advances.insert(Advance::Draft);
    game.players[1].civilization = game.cache.get_civilization("Rome");
    let game = attack(game.clone());
    assert!(round!(&game).defender_strength.deny_tactics_card);
    assert_eq!(
        request_cards(&game),
        vec![HandCard::ActionCard(HIGH_MORALE)]
    );
    assert_status(&game, 0, "none");
    assert_stale_scouts_rejected(game, 0);
}

#[test]
fn scouts_status_respects_trojan_horse_and_already_committed_cards() {
    let mut game = attack(setup(&[SCOUTS, HIGH_MORALE], &[]));
    let PersistentEventType::CombatRoundStart(round) =
        &mut game.events.last_mut().unwrap().event_type
    else {
        unreachable!();
    };
    round
        .combat
        .modifiers
        .push(server::combat::CombatModifier::TrojanHorse);
    assert_status(&game, 0, "none");
    assert_eq!(view_cards(&game, 0), vec![HIGH_MORALE]);
    let PersistentEventType::CombatRoundStart(round) =
        &mut game.events.last_mut().unwrap().event_type
    else {
        unreachable!();
    };
    // Presence takes precedence over public inability to make a new choice.
    round.defender_strength.tactics_card = Some(0);
    round.defender_strength.deny_tactics_card = true;
    game.players[1].advances.remove(Advance::Tactics);
    assert_status(&game, 0, "played");
    assert_eq!(view_cards(&game, 0), vec![HIGH_MORALE, SCOUTS]);
}

#[test]
fn loyalty_revalidates_scouts_from_a_stale_prompt() {
    let mut game = attack(loyalty_setup(&[PELTASTS], &[SCOUTS, HIGH_MORALE]));
    assert_status(&game, 1, "pending");
    assert!(request_cards(&game).contains(&HandCard::ActionCard(SCOUTS)));
    // Model an older pending request containing a now-ineligible card. The
    // supplying Celtic seat must be checked as the defending Barbarian army.
    game.players[0].advances.remove(Advance::Tactics);
    assert_status(&game, 1, "none");
    assert_stale_scouts_rejected(game, 1);
}
