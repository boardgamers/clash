use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::Action;
use crate::action_card::ActionCard;
use crate::content::custom_phase_actions::{EventResponse, PaymentRequest, PositionRequest};
use crate::content::incidents::great_persons::great_person_action_card;
use crate::explore::move_to_unexplored_block;
use crate::game::Game;
use crate::map::{get_map_setup, BlockPosition, UNEXPLORED_BLOCK};
use crate::payment::PaymentOptions;
use crate::playing_actions::{build_city, ActionType};
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use itertools::Itertools;

pub(crate) fn great_explorer() -> ActionCard {
    great_person_action_card(
        18,
        "Great Explorer",
        "todo",
        ActionType::regular(),
        &["Seafaring"],
        |_game, _player| true,
    )
    .add_position_request(
        |e| &mut e.on_play_action_card,
        9,
        |game, player_index, _| Some(explore_choices(game, player_index)),
        |game, s, _| {
            let position = s.choice[0];
            game.add_info_log_item(&format!("{} explored {}", s.player_name, position));
            let dest = game
                .map
                .unexplored_blocks
                .iter()
                .find(|b| tiles(&b.position).iter().any(|p| *p == position))
                .cloned()
                .expect("Block not found");
            move_to_unexplored_block(game, s.player_index, &dest, &[], position, None);
        },
    )
    .add_position_request(
        |e| &mut e.on_play_action_card,
        8,
        |game, player_index, _| {
            if !game.get_player(player_index).can_afford(&city_cost()) {
                game.add_info_log_item("Player cannot afford to build a city");
            }

            let a = game
                .action_log
                .iter()
                .rev()
                .find_map(|l| {
                    if let Action::Response(EventResponse::SelectPositions(p)) = &l.action {
                        Some(p[0])
                    } else {
                        None
                    }
                })
                .expect("position not found");

            let setup = get_map_setup(game.human_players_count());
            let choices = setup
                .free_positions
                .into_iter()
                .find_map(|p| {
                    let t = tiles(&p);
                    t.contains(&a).then_some(t)
                })
                .expect("position not found");

            Some(PositionRequest::new(
                choices,
                0..=1,
                "Place a city for 2 food",
            ))
        },
        |game, s, a| {
            let pos = s.choice.first().copied();
            if let Some(pos) = pos {
                game.add_info_log_item(&format!(
                    "{} decided to build a city at {pos}",
                    s.player_name
                ));
            }
            a.selected_position = pos;
        },
    )
    .add_payment_request_listener(
        |e| &mut e.on_play_action_card,
        7,
        |_game, _player, a| {
            a.selected_position?;
            Some(vec![PaymentRequest::new(
                city_cost(),
                "Pay to build the city",
                false,
            )])
        },
        |game, s, a| {
            let pos = a.selected_position.expect("position not found");
            build_city(game.get_player_mut(s.player_index), pos);
            game.add_info_log_item(&format!("{} built a city at {pos}", s.player_name));
        },
    )
    .build()
}

fn city_cost() -> PaymentOptions {
    PaymentOptions::resources(ResourcePile::food(2))
}

fn explore_choices(game: &mut Game, player_index: usize) -> PositionRequest {
    let setup = get_map_setup(game.human_players_count());
    let free = &setup
        .free_positions
        .into_iter()
        .chain(setup.home_positions.iter().map(|h| h.position.clone()))
        .collect_vec();
    let choices = game
        .map
        .unexplored_blocks
        .clone()
        .into_iter()
        .map(|b| b.position)
        .filter_map(|b| {
            free.iter()
                .any(|p| has_any_city(game, p, player_index) && block_adjacent(&b, p))
                .then_some(b.top_tile)
        })
        .collect_vec();
    if choices.is_empty() {
        game.add_info_log_item("No valid positions to explore");
    }
    PositionRequest::new(choices, 0..=1, "Choose a region to explore")
}

fn block_adjacent(p1: &BlockPosition, p2: &BlockPosition) -> bool {
    let v1 = tiles(p1);
    let v2 = tiles(p2);
    v1.iter().any(|p| v2.iter().any(|p2| p.is_neighbor(*p2)))
}

fn tiles(p1: &BlockPosition) -> Vec<Position> {
    UNEXPLORED_BLOCK
        .tiles(p1, p1.rotation)
        .into_iter()
        .map(|(p, _)| p)
        .collect_vec()
}

fn has_any_city(game: &mut Game, p: &BlockPosition, player: usize) -> bool {
    tiles(p)
        .iter()
        .any(|p| game.get_player(player).try_get_city(*p).is_some())
}
