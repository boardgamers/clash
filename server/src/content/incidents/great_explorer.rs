use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{ActionCard, ActionCardBuilder};
use crate::city::{found_city, is_valid_city_terrain};
use crate::content::advances::AdvanceGroup;
use crate::content::incidents::great_persons::{
    GreatPersonType, great_person_card, tech_great_person_description,
};
use crate::content::persistent_events::{PaymentRequest, PositionRequest};
use crate::events::{EventOrigin, EventPlayer, check_event_origin};
use crate::explore::move_to_unexplored_block;
use crate::game::Game;
use crate::map::{BlockPosition, block_has_player_city, block_tiles, get_map_setup};
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use itertools::Itertools;

pub(crate) fn great_explorer() -> ActionCard {
    let groups = vec![AdvanceGroup::Seafaring];
    let mut builder = great_person_card(
        18,
        GreatPersonType::ActionCard,
        "Great Explorer",
        &format!(
            "{} Then, you may explore a region adjacent to a region \
            where you have a city. If you do, you may found a city in the explored region,\
            without using a Settler.",
            tech_great_person_description(&groups)
        ),
        |c| c.action().no_resources(),
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
            |game, p, a| Some(place_city_request(game, p, a.selected_positions.clone())),
            |game, s, a| {
                let pos = s.choice.first().copied();
                if let Some(pos) = pos {
                    s.log(game, &format!("Decided to build a city {pos}",));
                } else {
                    s.log(game, "decided not to build a city");
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
                    city_cost(player.get(game), player.origin.clone()),
                    "Pay to build the city",
                )])
            },
            |game, s, a| {
                found_city(
                    game,
                    &s.player(),
                    a.selected_position.expect("position not found"),
                );
            },
        )
        .build()
}

pub(crate) fn explore_adjacent_block(builder: ActionCardBuilder) -> ActionCardBuilder {
    builder.add_position_request(
        |e| &mut e.play_action_card,
        9,
        |game, p, _| Some(action_explore_request(game, p.index)),
        |game, s, a| {
            let Some(&position) = s.choice.first() else {
                s.log(game, "Decided not to explore");
                return;
            };
            s.log(game, &format!("Explored {position}"));
            let dest = game
                .map
                .unexplored_blocks
                .iter()
                .find(|b| block_tiles(&b.position).contains(&position))
                .cloned()
                .expect("Block not found");
            a.selected_positions = block_tiles(&dest.position);
            move_to_unexplored_block(game, &s.player(), &dest, &[], position, None);
        },
    )
}

fn place_city_request(
    game: &mut Game,
    p: &EventPlayer,
    positions: Vec<Position>,
) -> PositionRequest {
    if !p
        .get(game)
        .can_afford(&city_cost(p.get(game), check_event_origin()))
    {
        p.log(game, "Player cannot afford to build a city");
    }

    let choices = positions
        .into_iter()
        .filter(|p| game.map.get(*p).is_some_and(is_valid_city_terrain))
        .collect_vec();

    PositionRequest::new(choices, 0..=1, "Place a city for 2 food")
}

fn city_cost(player: &Player, origin: EventOrigin) -> PaymentOptions {
    PaymentOptions::resources(player, origin, ResourcePile::food(2))
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
