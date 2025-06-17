use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{ActionCard, ActionCardBuilder};
use crate::barbarians::{barbarian_reinforcement, get_barbarians_player};
use crate::combat::move_with_possible_combat;
use crate::consts::STACK_LIMIT;
use crate::content::action_cards::inspiration::player_positions;
use crate::content::civilizations::rome::owner_of_sulla_in_range;
use crate::content::persistent_events::{PaymentRequest, PositionRequest};
use crate::content::tactics_cards::TacticsCardFactory;
use crate::game::Game;
use crate::movement::{MoveUnits, move_action_log};
use crate::player::Player;
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::utils::remove_element;
use crate::wonder::Wonder;
use itertools::Itertools;

pub(crate) fn mercenaries(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    let mut b = ActionCard::builder(
        id,
        "Mercenaries",
        "You may move any number of Barbarian armies 1 space each, which may start combat. \
        The armies must be within range 2 of your cities or army units. \
        Pay 1 food, wood, ore, or culture token for each army up front. \
        Reinforce all Barbarian cities where you moved units out of according to the usual rules.",
        |c| c.action().no_resources(),
        |game, p, _| {
            !barbarian_army_positions_in_range2(game, p).0.is_empty()
                && max_mercenary_payment(p) > 0
        },
    )
    .tactics_card(tactics_card)
    .add_position_request(
        |e| &mut e.play_action_card,
        2,
        |game, p, _| {
            let (r, log) = barbarian_army_positions_in_range2(game, p.get(game));
            for l in log {
                p.log(game, &l);
            }
            if r.is_empty() {
                return None;
            }
            let max = (r.len() as u8).min(max_mercenary_payment(p.get(game)));
            Some(PositionRequest::new(
                r,
                1..=max,
                "Select Barbarian armies to move",
            ))
        },
        |game, s, a| {
            s.log(
                game,
                &format!(
                    "Selected Barbarian armies to move: {}",
                    s.choice.iter().map(ToString::to_string).join(", "),
                ),
            );
            a.selected_positions.clone_from(&s.choice);
        },
    )
    .add_payment_request_listener(
        |e| &mut e.play_action_card,
        1,
        |game, p, a| {
            Some(vec![PaymentRequest::mandatory(
                p.payment_options().sum(
                    p.get(game),
                    a.selected_positions.len() as u8,
                    &[
                        ResourceType::Food,
                        ResourceType::Wood,
                        ResourceType::Ore,
                        ResourceType::CultureTokens,
                        ResourceType::Gold,
                    ],
                ),
                "Pay for mercenaries",
            )])
        },
        |_game, _s, _| {},
    );

    // one for each possible barbarian army
    for i in (0..28).rev() {
        b = move_army(b, i);
    }

    b.build()
}

fn move_army(b: ActionCardBuilder, i: i32) -> ActionCardBuilder {
    let base_prio = i * -5;
    let src_prio = base_prio - 2;
    let dst_prio = base_prio - 4;
    let reinforce_prio = base_prio - 6;
    let b = b
        .add_position_request(
            |e| &mut e.play_action_card,
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
                s.log(game, &format!("Selected Barbarian army to move: {pos}",));
                remove_element(&mut a.selected_positions, &pos);
                a.selected_position = Some(pos);
            },
        )
        .add_position_request(
            |e| &mut e.play_action_card,
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
                s.log(
                    game,
                    &format!("Selected destination for Barbarian army: {to}",),
                );

                let from = a.selected_position.expect("position not found");
                let b = get_barbarians_player(game);
                let barbarian = b.index;

                let units = b.get_units(from).iter().map(|u| u.id).collect_vec();

                let m = MoveUnits::new(units, to, None, ResourcePile::empty());
                s.log(game, &move_action_log(game, b, &m));

                move_with_possible_combat(game, barbarian, &m);
            },
        );

    barbarian_reinforcement(
        b,
        |e| &mut e.play_action_card,
        reinforce_prio,
        |game, _, a| {
            a.selected_position
                .and_then(|p| game.try_get_any_city(p))
                .is_some()
        },
        |a| a.selected_position,
    )
}

fn barbarian_army_positions_in_range2(
    game: &Game,
    player: &Player,
) -> (Vec<Position>, Vec<String>) {
    let my = player_positions(player);
    let mut log = Vec::new();

    let positions = get_barbarians_player(game)
        .units
        .iter()
        .map(|u| u.position)
        .filter(|&p| {
            my.iter().any(|my_pos| {
                if my_pos.distance(p) > 2 {
                    return false;
                }
                if owner_of_sulla_in_range(p, game)
                    .is_some_and(|sulla_owner| player.index != sulla_owner)
                {
                    log.push(format!(
                        "{player} cannot move Barbarian army at {p} because Sulla is in range",
                    ));
                    return false;
                }
                true
            })
        })
        .collect();

    log.sort();
    log.dedup();

    (positions, log)
}

fn max_mercenary_payment(player: &Player) -> u8 {
    let pile = &player.resources;
    let mut max = pile.food + pile.wood + pile.ore + pile.culture_tokens + pile.gold;
    if player.wonders_owned.contains(Wonder::Colosseum) {
        max += pile.mood_tokens;
    }
    max
}
