use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{ActionCard, ActionCardBuilder};
use crate::barbarians::{barbarian_reinforcement, get_barbarians_player};
use crate::combat::move_with_possible_combat;
use crate::consts::STACK_LIMIT;
use crate::content::action_cards::inspiration::player_positions;
use crate::content::custom_phase_actions::{PaymentRequest, PositionRequest};
use crate::content::tactics_cards::TacticsCardFactory;
use crate::game::Game;
use crate::log::move_action_log;
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::playing_actions::ActionType;
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::unit::MoveUnits;
use crate::utils::remove_element;
use itertools::Itertools;

pub(crate) fn mercenaries(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    let mut b = ActionCard::builder(
        id,
        "Mercenaries",
        "You may move any number of Barbarian armies 1 space each, which may start combat. \
        The armies must be within range 2 of your cities or army units. \
        Pay 1 food, wood, ore, or culture token for each army up front. \
        Reinforce all Barbarian cities where you moved units out of according to the usual rules.",
        ActionType::regular(),
        |game, p| {
            !barbarian_army_positions_in_range2(game, p).is_empty() && max_mercenary_payment(p) > 0
        },
    )
    .tactics_card(tactics_card)
    .add_position_request(
        |e| &mut e.on_play_action_card,
        100,
        |game, player, _| {
            let p = game.get_player(player);
            let r = barbarian_army_positions_in_range2(game, p);
            if r.is_empty() {
                return None;
            }
            let max = (r.len() as u8).min(max_mercenary_payment(p));
            Some(PositionRequest::new(
                r,
                1..=max,
                "Select Barbarian armies to move",
            ))
        },
        |game, s, a| {
            game.add_info_log_item(&format!(
                "{} selected Barbarian armies to move: {}",
                s.player_name,
                s.choice.iter().map(ToString::to_string).join(", "),
            ));
            a.selected_positions.clone_from(&s.choice);
        },
    )
    .add_payment_request_listener(
        |e| &mut e.on_play_action_card,
        99,
        |_game, _player, a| {
            Some(vec![PaymentRequest::new(
                PaymentOptions::sum(
                    a.selected_positions.len() as u32,
                    &[
                        ResourceType::Food,
                        ResourceType::Wood,
                        ResourceType::Ore,
                        ResourceType::CultureTokens,
                        ResourceType::Gold,
                    ],
                ),
                "Pay for mercenaries",
                false,
            )])
        },
        |game, s, _| {
            game.add_info_log_item(&format!(
                "{} paid for mercenaries: {}",
                s.player_name, s.choice[0]
            ));
        },
    );

    // one for each possible barbarian army
    for i in (0..28).rev() {
        b = move_army(b, i);
    }

    b.build()
}

fn move_army(b: ActionCardBuilder, i: i32) -> ActionCardBuilder {
    let src_prio = i * 3 + 2;
    let dst_prio = i * 3 + 1;
    let reinforce_prio = i * 3;
    let b = b
        .add_position_request(
            |e| &mut e.on_play_action_card,
            src_prio,
            |_game, _player, a| {
                a.selected_position = None;

                Some(PositionRequest::new(
                    a.selected_positions.clone(),
                    1..=1,
                    "Select Barbarian army to move",
                ))
            },
            |game, s, a| {
                let pos = s.choice[0];
                game.add_info_log_item(&format!(
                    "{} selected Barbarian army to move: {}",
                    s.player_name, pos
                ));
                remove_element(&mut a.selected_positions, &pos);
                a.selected_position = Some(pos);
            },
        )
        .add_position_request(
            |e| &mut e.on_play_action_card,
            dst_prio,
            |game, _player, a| {
                let from = a.selected_position?;
                let barbarian = get_barbarians_player(game);
                let units = barbarian.get_units(from).iter().map(|u| u.id).collect_vec();

                let destinations = from
                    .neighbors()
                    .into_iter()
                    .filter(|&to| {
                        game.map.is_land(to)
                            && units.len() + barbarian.get_units(to).len() <= STACK_LIMIT
                    })
                    .collect_vec();

                Some(PositionRequest::new(
                    destinations,
                    1..=1,
                    "Select destination for Barbarian army",
                ))
            },
            |game, s, a| {
                let to = s.choice[0];
                game.add_info_log_item(&format!(
                    "{} selected destination for Barbarian army: {}",
                    s.player_name, to
                ));

                let from = a.selected_position.expect("position not found");
                let b = get_barbarians_player(game);
                let barbarian = b.index;

                let units = b.get_units(from).iter().map(|u| u.id).collect_vec();

                let m = MoveUnits::new(units, to, None, ResourcePile::empty());
                game.add_info_log_item(&move_action_log(game, b, &m));

                move_with_possible_combat(game, barbarian, from, &m);
            },
        );

    barbarian_reinforcement(
        b,
        |e| &mut e.on_play_action_card,
        reinforce_prio,
        |game, _, a| {
            a.selected_position
                .and_then(|p| game.try_get_any_city(p))
                .is_some()
        },
        |a| a.selected_position.expect("position not found"),
    )
}

fn barbarian_army_positions_in_range2(game: &Game, player: &Player) -> Vec<Position> {
    let my = player_positions(player);

    get_barbarians_player(game)
        .units
        .iter()
        .map(|u| u.position)
        .filter(|b| my.iter().any(|my_pos| my_pos.distance(*b) <= 2))
        .collect()
}

fn max_mercenary_payment(player: &Player) -> u8 {
    let pile = &player.resources;
    (pile.food + pile.wood + pile.ore + pile.culture_tokens + pile.gold) as u8
}
