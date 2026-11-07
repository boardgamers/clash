use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{ActionCard, ActionCardBuilder};
use crate::city::{found_city, is_valid_city_terrain_for_player};
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

// Printed Great Discoverer, E18 (English event-deck scan in docs/card-text-audit.json):
// the optional city costs 2 food even though no Settler is used.
pub(crate) fn great_explorer() -> ActionCard {
    let groups = vec![AdvanceGroup::Seafaring];
    let mut builder = great_person_card(
        18,
        GreatPersonType::ActionCard,
        "Great Explorer",
        &format!(
            "{} Then, you may explore a region adjacent to a region \
            where you have a city. If you do, you may pay 2 food to found a city in the explored region, \
            without using a Settler.",
            tech_great_person_description(&groups)
        ),
        |c| c.action().no_resources(),
        groups,
        |_game, _player| true,
    );
    builder = explore_adjacent_block(builder);
    builder
        .add_position_request(
            |e| &mut e.play_action_card,
            8,
            |game, p, a| place_city_request(game, p, &a.selected_positions),
            |game, s, a| {
                a.selected_position = None;
                if let Some(&pos) = s.choice.first() {
                    // Old saves can still contain a location offered without checking
                    // affordability or the city supply. Finish those choices safely.
                    if let Some(reason) = placement_blocker(game, &s.player()).or_else(|| {
                        city_site_blocker(game, &s.player(), pos, &a.selected_positions)
                    }) {
                        s.log(game, reason);
                        return;
                    }
                    s.log(game, &format!("Decided to build a city {pos}"));
                    a.selected_position = Some(pos);
                } else {
                    s.log(game, "decided not to build a city");
                }
            },
        )
        .add_payment_request_listener(
            |e| &mut e.play_action_card,
            7,
            |game, player, a| {
                let pos = a.selected_position?;
                if let Some(reason) = placement_blocker(game, player)
                    .or_else(|| city_site_blocker(game, player, pos, &a.selected_positions))
                {
                    player.log(game, reason);
                    return None;
                }
                Some(vec![PaymentRequest::mandatory(
                    city_cost(player.get(game), player.origin.clone()),
                    "Pay to build the city",
                )])
            },
            |game, s, a| {
                let pos = a.selected_position.expect("position not found");
                // A saved payment request may predate supply/site validation. The
                // normal payment validator still runs; refund an obsolete choice
                // rather than exceeding supply or rewriting any previous action.
                if let Some(reason) =
                    city_site_blocker(game, &s.player(), pos, &a.selected_positions)
                {
                    s.player().gain_resources(game, s.choice[0].clone());
                    s.log(game, reason);
                    return;
                }
                found_city(game, &s.player(), pos);
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
    positions: &[Position],
) -> Option<PositionRequest> {
    if positions.is_empty() {
        return None;
    }
    if let Some(reason) = placement_blocker(game, p) {
        p.log(game, reason);
        return None;
    }

    let choices = positions
        .iter()
        .copied()
        .filter(|pos| city_site_blocker(game, p, *pos, positions).is_none())
        .collect_vec();

    Some(PositionRequest::new(
        choices,
        0..=1,
        "Place a city for 2 food",
    ))
}

fn placement_blocker(game: &Game, p: &EventPlayer) -> Option<&'static str> {
    if !p.get(game).is_city_available() {
        return Some("You have reached your city limit");
    }
    if !p
        .get(game)
        .can_afford(&city_cost(p.get(game), check_event_origin()))
    {
        return Some("Player cannot afford to build a city");
    }
    None
}

fn city_site_blocker(
    game: &Game,
    p: &EventPlayer,
    pos: Position,
    explored_region: &[Position],
) -> Option<&'static str> {
    if !p.get(game).is_city_available() {
        return Some("You have reached your city limit");
    }
    if !explored_region.contains(&pos)
        || !game
            .map
            .get(pos)
            .is_some_and(|t| is_valid_city_terrain_for_player(t, p.get(game)))
        || game
            .players
            .iter()
            .any(|player| player.try_get_city(pos).is_some())
    {
        return Some("Move to an empty land tile to found a city");
    }
    None
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
