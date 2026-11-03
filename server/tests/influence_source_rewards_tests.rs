use serde_json::{Value, json};
use server::action::{Action, execute_action};
use server::cache::Cache;
use server::city_pieces::Building;
use server::content::persistent_events::EventResponse;
use server::events::EventOrigin;
use server::game::{Game, GameContext};
use server::game_data::GameData;
use server::leader::Leader;
use server::log::{ActionLogBalance, ActionLogEntry};
use server::map::Terrain;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::unit::UnitType;
use server::web_view;

fn source() -> Position {
    Position::from_offset("C3")
}

fn other_source() -> Position {
    source().neighbors()[0]
}

fn target() -> Position {
    source().neighbors()[1]
}

fn fixture(unit_target: bool) -> Game {
    let mut data: Value =
        serde_json::from_str(include_str!("test_games/base/recruit.json")).unwrap();
    let p = &mut data["players"][0];
    p["civilization"] = json!("Persia");
    p["advances"] = json!(["Farming", "Mining", "Writing", "Myths", "Priesthood"]);
    p["available_leaders"] = json!(["Cyrus", "Xerxes"]);
    p["cities"] = json!([
        {"position":source(), "mood_state":"Happy", "city_pieces":{"academy":0}},
        {"position":other_source(), "mood_state":"Happy", "city_pieces":{"academy":0}}
    ]);
    p["units"] = json!([
        {"id":0, "position":source(), "unit_type":{"Leader":"Darius"}}
    ]);
    p["next_unit_id"] = json!(1);
    p["resources"] = json!({"food":2,"wood":7,"ore":7,"ideas":7,"gold":7,
        "mood_tokens":7,"culture_tokens":7});
    p["resource_limit"] = p["resources"].clone();
    p["objective_cards"] = json!([]);
    let p = &mut data["players"][1];
    p["objective_cards"] = json!([]);
    p["cities"] = json!([{
        "position":if unit_target { source().neighbors()[2] } else { target() },
        "mood_state":"Happy", "city_pieces":{"temple":1}
    }]);
    p["units"] = if unit_target {
        json!([{"id":0,"position":target(),"unit_type":"Infantry"}])
    } else {
        json!([])
    };
    p["next_unit_id"] = json!(1);
    data["players"].as_array_mut().unwrap().extend([
        json!({"id":2,"civilization":"Barbarians","next_unit_id":0}),
        json!({"id":3,"civilization":"Pirates","next_unit_id":0}),
    ]);
    data["actions_left"] = json!(3);
    data["dice_roll_outcomes"] = json!([10]); // A successful roll of 6.
    let data: GameData = serde_json::from_value(data).unwrap();
    let cache = Cache::new(&data.options);
    let mut game = Game::from_data(data, cache, GameContext::Play);
    game.map.tiles.clear();
    game.map.unexplored_blocks.clear();
    for position in std::iter::once(source()).chain(source().neighbors()) {
        game.map.tiles.insert(position, Terrain::Fertile);
    }
    game
}

fn offer(game: &Game, unit_target: bool) -> Value {
    web_view::view(game, Some(0))["influence"]
        .as_array()
        .unwrap()
        .iter()
        .find(|offer| {
            offer["position"] == json!(target())
                && if unit_target {
                    offer["variant"] == "Zoroastrianism"
                } else {
                    offer["name"] == "Temple"
                }
        })
        .expect("influence target must be offered")
        .clone()
}

fn origin(offer: &Value, position: Position) -> &Value {
    offer["origins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|origin| origin["position"] == json!(position))
        .expect("source must be offered")
}

fn cultural_unity_reward() -> Value {
    json!([{"name":"Cultural Unity","resources":ResourcePile::mood_tokens(1)}])
}

fn play_origin(game: Game, origin: &Value) -> Game {
    assert_eq!(origin["payment"], json!({}), "fixture needs no range boost");
    let action: Action = serde_json::from_value(origin["action"].clone()).unwrap();
    assert!(
        !serde_json::to_string(&action)
            .unwrap()
            .contains("successRewards"),
        "reward previews must not alter the serialized action schema"
    );
    execute_action(game, action, 0)
}

fn logged_source_rewards(game: &Game) -> Value {
    let rewards: Vec<Value> = game
        .log
        .iter()
        .flat_map(|age| &age.rounds)
        .flat_map(|round| &round.turns)
        .flat_map(|turn| &turn.actions)
        .flat_map(|action| &action.items)
        .filter(|item| item.origin == EventOrigin::LeaderAbility("Cultural Unity".into()))
        .map(|item| {
            assert_eq!(item.player, 0);
            let ActionLogEntry::Resources {
                resources,
                balance: ActionLogBalance::Gain,
            } = &item.entry
            else {
                panic!("Cultural Unity must retain its resource gain log entry")
            };
            json!({"name":item.origin.name(game),"resources":resources})
        })
        .collect();
    json!(rewards)
}

fn assert_target_owner(game: &Game, unit_target: bool, success: bool) {
    if unit_target {
        let owner = usize::from(!success);
        assert!(
            game.player(owner)
                .units
                .iter()
                .any(|unit| { unit.position == target() && unit.unit_type == UnitType::Infantry })
        );
        assert!(
            !game
                .player(1 - owner)
                .units
                .iter()
                .any(|unit| { unit.position == target() && unit.unit_type == UnitType::Infantry })
        );
    } else {
        assert_eq!(
            game.get_any_city(target())
                .pieces
                .building_owner(Building::Temple),
            Some(usize::from(!success))
        );
    }
    assert_eq!(game.successful_cultural_influence, success);
}

#[test]
fn structure_and_unit_source_previews_match_successful_awards_and_logs() {
    for unit_target in [false, true] {
        for starting_position in [source(), other_source()] {
            let game = fixture(unit_target);
            let saved = serde_json::to_value(game.cloned_data()).unwrap();
            let preview = offer(&game, unit_target);
            assert_eq!(offer(&game, unit_target), preview);
            assert_eq!(serde_json::to_value(game.cloned_data()).unwrap(), saved);
            assert_eq!(
                origin(&preview, source())["successRewards"],
                cultural_unity_reward()
            );
            assert_eq!(
                origin(&preview, other_source())["successRewards"],
                json!([])
            );
            let selected = origin(&preview, starting_position);
            let rewards = selected["successRewards"].clone();
            let mood = game.player(0).resources.mood_tokens;
            let game = play_origin(game, selected);
            assert!(game.events.is_empty());
            assert_target_owner(&game, unit_target, true);
            assert_eq!(logged_source_rewards(&game), rewards);
            assert_eq!(
                game.player(0).resources.mood_tokens,
                mood + u8::from(starting_position == source()),
                "Cultural Unity applies only at Darius' exact source, even above resource_limit"
            );
        }
    }
}

#[test]
fn moved_missing_and_replaced_leaders_do_not_reward_the_old_source() {
    for unit_target in [false, true] {
        for state in ["moved", "missing", "replaced"] {
            let mut game = fixture(unit_target);
            match state {
                "moved" => game.player_mut(0).get_unit_mut(0).position = other_source(),
                "missing" => game.player_mut(0).units.clear(),
                "replaced" => {
                    game.player_mut(0).get_unit_mut(0).unit_type = UnitType::Leader(Leader::Xerxes);
                }
                _ => unreachable!(),
            }
            // These are setup changes; rebuild the leader listeners as on load.
            let game = Game::from_data(game.cloned_data(), game.cache.clone(), GameContext::Play);
            let preview = offer(&game, unit_target);
            assert_eq!(origin(&preview, source())["successRewards"], json!([]));
            assert_eq!(
                origin(&preview, other_source())["successRewards"],
                if state == "moved" {
                    cultural_unity_reward()
                } else {
                    json!([])
                }
            );
            let mood = game.player(0).resources.mood_tokens;
            let game = play_origin(game, origin(&preview, source()));
            assert_target_owner(&game, unit_target, true);
            assert_eq!(game.player(0).resources.mood_tokens, mood);
            assert_eq!(logged_source_rewards(&game), json!([]));
        }
    }
}

#[test]
fn source_rewards_are_conditional_on_success_including_paid_roll_boosts() {
    for unit_target in [false, true] {
        for accept_boost in [false, true] {
            let mut game = fixture(unit_target);
            game.dice_roll_outcomes = vec![0]; // Roll 1, followed by an optional +4 boost.
            let preview = offer(&game, unit_target);
            assert_eq!(
                origin(&preview, source())["successRewards"],
                cultural_unity_reward()
            );
            let mood = game.player(0).resources.mood_tokens;
            let game = play_origin(game, origin(&preview, source()));
            assert!(game.current_event_handler().is_some());
            assert_eq!(game.player(0).resources.mood_tokens, mood);
            assert_eq!(logged_source_rewards(&game), json!([]));
            let payment = if accept_boost {
                ResourcePile::culture_tokens(4)
            } else {
                ResourcePile::empty()
            };
            let game = execute_action(
                game,
                Action::Response(EventResponse::Payment(vec![payment])),
                0,
            );
            assert!(game.events.is_empty());
            assert_target_owner(&game, unit_target, accept_boost);
            assert_eq!(
                game.player(0).resources.mood_tokens,
                mood + u8::from(accept_boost)
            );
            assert_eq!(
                logged_source_rewards(&game),
                if accept_boost {
                    cultural_unity_reward()
                } else {
                    json!([])
                }
            );
        }
    }
}

#[test]
fn universal_conversion_reward_is_not_listed_as_a_source_reward() {
    for unit_target in [false, true] {
        for starting_position in [source(), other_source()] {
            let mut game = fixture(unit_target);
            game.player_mut(0)
                .advances
                .insert(server::advance::Advance::Conversion);
            let game = Game::from_data(game.cloned_data(), game.cache.clone(), GameContext::Play);
            let preview = offer(&game, unit_target);
            let selected = origin(&preview, starting_position);
            assert_eq!(selected["rollBonus"], 1);
            assert_eq!(
                selected["successRewards"],
                if starting_position == source() {
                    cultural_unity_reward()
                } else {
                    json!([])
                }
            );
            let culture = game.player(0).resources.culture_tokens;
            let game = play_origin(game, selected);
            assert_target_owner(&game, unit_target, true);
            assert_eq!(game.player(0).resources.culture_tokens, culture + 1);
            assert_eq!(logged_source_rewards(&game), selected["successRewards"]);
        }
    }
}
