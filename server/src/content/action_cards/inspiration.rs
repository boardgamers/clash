use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::ActionCard;
use crate::advance::gain_advance;
use crate::content::advances;
use crate::content::custom_phase_actions::AdvanceRequest;
use crate::content::tactics_cards::{encircled, peltasts};
use crate::player::Player;
use crate::playing_actions::ActionType;
use crate::resource_pile::ResourcePile;
use crate::tactics_card::TacticsCard;

pub(crate) fn inspiration_action_cards() -> Vec<ActionCard> {
    vec![advance(1, peltasts()), advance(2, encircled())]
}

fn advance(id: u8, tactics_card: TacticsCard) -> ActionCard {
    ActionCard::builder(
        id,
        "Advance",
        "Pay 1 idea: Gain 1 advance without changing the Game Event counter.",
        ActionType::free(),
        |_game, player| player.resources.ideas >= 1 && !possible_advances(player).is_empty(),
    )
    .with_tactics_card(tactics_card)
    .add_advance_request(
        |e| &mut e.on_play_action_card,
        0,
        |game, player, _| {
            let p = game.get_player(player);

            let advances = possible_advances(p);
            if advances.is_empty() {
                return None;
            }

            Some(AdvanceRequest::new(advances))
        },
        |game, sel, _| {
            let advance = sel.choice.clone();
            gain_advance(
                game,
                &advance,
                sel.player_index,
                ResourcePile::ideas(1),
                false,
            );
            let name = &sel.player_name;
            game.add_info_log_item(&format!("{name} gained {advance} for 1 idea.",));
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
