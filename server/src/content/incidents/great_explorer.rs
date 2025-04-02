use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::Action;
use crate::action_card::{ActionCard, ActionCardBuilder};
use crate::content::incidents::great_persons::{
    great_person_action_card, great_person_description,
};
use crate::content::persistent_events::{EventResponse, PaymentRequest, PositionRequest};
use crate::explore::move_to_unexplored_block;
use crate::game::Game;
use crate::log::current_player_turn_log;
use crate::map::{BlockPosition, UNEXPLORED_BLOCK, get_map_setup};
use crate::payment::PaymentOptions;
use crate::playing_actions::{ActionType, build_city};
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use itertools::Itertools;

pub(crate) fn great_explorer() -> ActionCard {
    let groups = &["Seafaring"];
    let mut builder = great_person_action_card(
        18,
        "Great Explorer",
        &format!(
            "{} Then, you may explore a region adjacent to a region \
            where you have a city. If you do, you may found a city in the explored region,\
            without using a Settler.",
            great_person_description(groups)
        ),
        ActionType::regular(),
        groups,
        |game, player| {
            !action_explore_request(game, player.index)
                .request
                .choices
                .is_empty()
        },
    );
    builder = explore_adjacent_block(builder);
    builder
        .add_position_request(
            |e| &mut e.play_action_card,
            8,
            |game, player_index, _| Some(place_city_request(game, player_index)),
            |game, s, a| {
                let pos = s.choice.first().copied();
                if let Some(pos) = pos {
                    game.add_info_log_item(&format!(
                        "{} decided to build a city at {pos}",
                        s.player_name
                    ));
                } else {
                    game.add_info_log_item(&format!(
                        "{} decided not to build a city",
                        s.player_name
                    ));
                }
                a.selected_position = pos;
            },
        )
        .add_payment_request_listener(
            |e| &mut e.play_action_card,
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
                build_city(game.player_mut(s.player_index), pos);
                game.add_info_log_item(&format!(
                    "{} built a city at {pos} for {}",
                    s.player_name, s.choice[0]
                ));
            },
        )
        .build()
}

pub(crate) fn explore_adjacent_block(builder: ActionCardBuilder) -> ActionCardBuilder {
    let builder1 = builder.add_position_request(
        |e| &mut e.play_action_card,
        9,
        |game, player_index, _| Some(action_explore_request(game, player_index)),
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
    );
    builder1
}

fn place_city_request(game: &mut Game, player_index: usize) -> PositionRequest {
    if !game.player(player_index).can_afford(&city_cost()) {
        game.add_info_log_item("Player cannot afford to build a city");
    }

    let a = current_player_turn_log(game)
        .items
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

    PositionRequest::new(choices, 0..=1, "Place a city for 2 food")
}

fn city_cost() -> PaymentOptions {
    PaymentOptions::resources(ResourcePile::food(2))
}

pub(crate) fn action_explore_request(game: &Game, player_index: usize) -> PositionRequest {
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

fn has_any_city(game: &Game, p: &BlockPosition, player: usize) -> bool {
    tiles(p)
        .iter()
        .any(|p| game.player(player).try_get_city(*p).is_some())
}
