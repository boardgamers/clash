use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{ActionCard, ActionCardBuilder};
use crate::city::{found_city, is_valid_city_terrain};
use crate::content::incidents::great_persons::{
    great_person_action_card, great_person_description,
};
use crate::content::persistent_events::{PaymentRequest, PositionRequest};
use crate::explore::move_to_unexplored_block;
use crate::game::Game;
use crate::map::{BlockPosition, block_has_player_city, block_tiles, get_map_setup};
use crate::payment::{PaymentOptions, PaymentReason};
use crate::player::Player;
use crate::playing_actions::ActionCost;
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
        ActionCost::regular(),
        groups,
        |game, player| {
            !action_explore_request(game, player.index)
                .choices
                .is_empty()
        },
    );
    builder = explore_adjacent_block(builder);
    builder
        .add_position_request(
            |e| &mut e.play_action_card,
            8,
            |game, player_index, a| {
                Some(place_city_request(
                    game,
                    player_index,
                    a.selected_positions.clone(),
                ))
            },
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
            |game, player, a| {
                a.selected_position?;
                Some(vec![PaymentRequest::mandatory(
                    city_cost(game.player(player)),
                    "Pay to build the city",
                )])
            },
            |game, s, a| {
                let pos = a.selected_position.expect("position not found");
                game.add_info_log_item(&format!(
                    "{} built a city at {pos} for {}",
                    s.player_name, s.choice[0]
                ));
                found_city(game, s.player_index, pos);
            },
        )
        .build()
}

pub(crate) fn explore_adjacent_block(builder: ActionCardBuilder) -> ActionCardBuilder {
    builder.add_position_request(
        |e| &mut e.play_action_card,
        9,
        |game, player_index, _| Some(action_explore_request(game, player_index)),
        |game, s, a| {
            let Some(&position) = s.choice.first() else {
                game.add_info_log_item(&format!("{} decided not to explore", s.player_name));
                return;
            };
            game.add_info_log_item(&format!("{} explored {}", s.player_name, position));
            let dest = game
                .map
                .unexplored_blocks
                .iter()
                .find(|b| block_tiles(&b.position).iter().any(|p| *p == position))
                .cloned()
                .expect("Block not found");
            a.selected_positions = block_tiles(&dest.position);
            move_to_unexplored_block(game, s.player_index, &dest, &[], position, None);
        },
    )
}

fn place_city_request(
    game: &mut Game,
    player_index: usize,
    positions: Vec<Position>,
) -> PositionRequest {
    let p = game.player(player_index);
    if !p.can_afford(&city_cost(p)) {
        game.add_info_log_item("Player cannot afford to build a city");
    }

    let choices = positions
        .into_iter()
        .filter(|p| game.map.get(*p).is_some_and(is_valid_city_terrain))
        .collect_vec();

    PositionRequest::new(choices, 0..=1, "Place a city for 2 food")
}

fn city_cost(player: &Player) -> PaymentOptions {
    PaymentOptions::resources(player, PaymentReason::ActionCard, ResourcePile::food(2))
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
                .any(|p| block_has_player_city(game, p, player_index) && block_adjacent(&b, p))
                .then_some(b.top_tile)
        })
        .collect_vec();
    PositionRequest::new(choices, 0..=1, "Choose a region to explore")
}

fn block_adjacent(p1: &BlockPosition, p2: &BlockPosition) -> bool {
    let v1 = block_tiles(p1);
    let v2 = block_tiles(p2);
    v1.iter().any(|p| v2.iter().any(|p2| p.is_neighbor(*p2)))
}
