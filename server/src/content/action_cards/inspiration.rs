use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::ActionCard;
use crate::advance::gain_advance;
use crate::content::advances;
use crate::content::custom_phase_actions::AdvanceRequest;
use crate::content::tactics_cards::{encircled, peltasts};
use crate::game::Game;
use crate::player::Player;
use crate::playing_actions::ActionType;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::tactics_card::TacticsCard;
use itertools::Itertools;

pub(crate) fn inspiration_action_cards() -> Vec<ActionCard> {
    vec![
        advance(1, peltasts()),
        advance(2, encircled()),
        inspiration(3, encircled()),
        inspiration(4, peltasts()),
    ]
}

fn advance(id: u8, tactics_card: TacticsCard) -> ActionCard {
    ActionCard::builder(
        id,
        "Advance",
        "Pay 1 culture token: Gain 1 advance without changing the Game Event counter.",
        ActionType::free(),
        |_game, player| {
            player.resources.culture_tokens >= 1 && !possible_advances(player).is_empty()
        },
    )
    .with_tactics_card(tactics_card)
    .add_advance_request(
        |e| &mut e.on_play_action_card,
        0,
        |game, player, _| {
            Some(AdvanceRequest::new(possible_advances(
                game.get_player(player),
            )))
        },
        |game, sel, _| {
            let advance = sel.choice.clone();
            gain_advance(
                game,
                &advance,
                sel.player_index,
                ResourcePile::culture_tokens(1),
                false,
            );
            let name = &sel.player_name;
            game.add_info_log_item(&format!(
                "{name} gained {advance} for 1 culture token using the Advance action card.",
            ));
        },
    )
    .build()
}

fn possible_advances(player: &Player) -> Vec<String> {
    advances::get_all()
        .iter()
        .filter(|a| player.can_advance_free(a))
        .map(|a| a.name.clone())
        .collect()
}

fn inspiration(id: u8, tactics_card: TacticsCard) -> ActionCard {
    ActionCard::builder(
        id,
        "Inspiration",
        "Gain 1 advance for free (without changing the Game Event counter) \
        that a player owns who has a unit or city within range 2 of your units or cities.",
        ActionType::free(),
        |game, player| !possible_inspiration_advances(game, player).is_empty(),
    )
    .with_tactics_card(tactics_card)
    .add_advance_request(
        |e| &mut e.on_play_action_card,
        0,
        |game, player, _| {
            Some(AdvanceRequest::new(possible_inspiration_advances(
                game,
                game.get_player(player),
            )))
        },
        |game, sel, _| {
            let advance = sel.choice.clone();
            gain_advance(
                game,
                &advance,
                sel.player_index,
                ResourcePile::empty(),
                false,
            );
            let name = &sel.player_name;
            game.add_info_log_item(&format!(
                "{name} gained {advance} for free using Inspiration.",
            ));
        },
    )
    .build()
}

fn possible_inspiration_advances(game: &Game, player: &Player) -> Vec<String> {
    let players = players_in_range2(game, player)
        .iter()
        .map(|&i| game.get_player(i))
        .collect_vec();

    advances::get_all()
        .iter()
        .filter(|a| player.can_advance_free(a) && players.iter().any(|p| p.has_advance(&a.name)))
        .map(|a| a.name.clone())
        .collect()
}

fn players_in_range2(game: &Game, player: &Player) -> Vec<usize> {
    let my = positions(player);

    game.players
        .iter()
        .filter(|p| {
            if p.index == player.index || !p.is_human() {
                return false;
            }
            let other = positions(p);
            other
                .iter()
                .any(|pos| my.iter().any(|my_pos| my_pos.distance(*pos) <= 2))
        })
        .map(|p| p.index)
        .collect()
}

fn positions(player: &Player) -> Vec<Position> {
    player
        .units
        .iter()
        .map(|u| u.position)
        .chain(player.cities.iter().map(|c| c.position))
        .collect()
}
