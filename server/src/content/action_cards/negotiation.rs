use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::Action;
use crate::action_card::ActionCard;
use crate::content::builtin::Builtin;
use crate::content::effects::PermanentEffect;
use crate::content::incidents::great_diplomat::{DiplomaticRelations, Negotiations};
use crate::content::persistent_events::PlayerRequest;
use crate::content::tactics_cards::{TacticsCardFactory, martyr, scout};
use crate::game::Game;
use crate::log::current_player_turn_log;
use crate::playing_actions::{ActionType, PlayingAction};
use crate::resource_pile::ResourcePile;
use crate::utils::remove_element_by;

pub(crate) fn negotiation_action_cards() -> Vec<ActionCard> {
    vec![negotiations(23, scout), negotiations(24, martyr)]
}

fn negotiations(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Negotiations",
        "Select another player. This turn, you may not attack that player. \
        In their next turn, they may not attack you.",
        ActionType::cost(ResourcePile::culture_tokens(1)),
        move |game, _player| {
            !current_player_turn_log(game)
                .items
                .iter()
                .any(|i| match &i.action {
                    Action::Playing(PlayingAction::ActionCard(i)) if *i == id => false,
                    Action::Playing(_) | Action::Movement(_) => true,
                    _ => false,
                })
        },
    )
    .add_player_request(
        |e| &mut e.play_action_card,
        0,
        |game, player_index, _| {
            Some(PlayerRequest::new(
                game.players
                    .iter()
                    .filter(|p| p.index != player_index && p.is_human())
                    .map(|p| p.index)
                    .collect(),
                "Select a player to negotiate with",
            ))
        },
        |game, s, _| {
            game.permanent_effects
                .push(PermanentEffect::Negotiations(Negotiations {
                    relations: DiplomaticRelations::new(s.player_index, s.choice),
                    remaining_turns: 2,
                }));
            game.add_info_log_item(&format!(
                "{} and {} are in negotiations.",
                s.player_name,
                game.player_name(s.choice)
            ));
        },
    )
    .tactics_card(tactics_card)
    .build()
}

pub(crate) fn use_negotiations() -> Builtin {
    Builtin::builder("Negotiations", "")
        .add_simple_persistent_event_listener(
            |e| &mut e.turn_start,
            1,
            |game, player_index, player_name, ()| {
                if let Some(negotiations_partner) = negotiations_partner(game, player_index) {
                    let partner_name = game.player_name(negotiations_partner);
                    for e in &mut game.permanent_effects {
                        if let PermanentEffect::Negotiations(d) = e {
                            d.remaining_turns -= 1;
                            if d.remaining_turns == 0 {
                                game.add_info_log_item(&format!(
                                    "{player_name} may attack {partner_name} again.",
                                ));
                                remove_element_by(&mut game.permanent_effects, |e| {
                                    matches!(e, PermanentEffect::Negotiations(_))
                                });
                            } else {
                                game.add_info_log_item(&format!(
                                    "{player_name} may not attack {partner_name} this turn.",
                                ));
                            }
                            return;
                        }
                    }
                }
            },
        )
        .build()
}

pub(crate) fn negotiations_partner(game: &Game, p: usize) -> Option<usize> {
    game.permanent_effects.iter().find_map(|e| {
        if let PermanentEffect::Negotiations(d) = e {
            d.relations.partner(p)
        } else {
            None
        }
    })
}
