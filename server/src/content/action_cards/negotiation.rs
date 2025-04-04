use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::Action;
use crate::action_card::ActionCard;
use crate::content::builtin::Builtin;
use crate::content::effects::{CollectEffect, PermanentEffect};
use crate::content::incidents::great_diplomat::{DiplomaticRelations, Negotiations};
use crate::content::persistent_events::PlayerRequest;
use crate::content::tactics_cards::{
    TacticsCardFactory, archers, defensive_formation, encircled, high_ground, martyr, scout,
};
use crate::game::Game;
use crate::log::current_player_turn_log;
use crate::playing_actions::{ActionType, PlayingAction};
use crate::resource_pile::ResourcePile;
use crate::utils::remove_element_by;

pub(crate) fn negotiation_action_cards() -> Vec<ActionCard> {
    vec![
        negotiations(23, scout),
        negotiations(24, martyr),
        leadership(25, high_ground),
        leadership(26, archers),
        assassination(27, high_ground),
        assassination(28, archers),
        overproduction(29, defensive_formation),
        overproduction(30, encircled),
    ]
}

fn negotiations(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Negotiations",
        "Select another player. This turn, you may not attack that player. \
        In their next turn, they may not attack you.",
        ActionType::cost(ResourcePile::culture_tokens(1)),
        move |game, _player, _| {
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

                    let mut delete = Vec::new();
                    let mut remain = Vec::new();
                    for (i, e) in &mut game.permanent_effects.iter_mut().enumerate() {
                        if let PermanentEffect::Negotiations(d) = e {
                            d.remaining_turns -= 1;
                            if d.remaining_turns == 0 {
                                delete.push(i);
                            } else {
                                remain.push(i);
                            }
                        }
                    }
                    // must be in reverse order to not mess up the indices during deletion
                    for i in delete.iter().rev() {
                        game.add_info_log_item(&format!(
                            "{player_name} may attack {partner_name} again.",
                        ));
                        game.permanent_effects.remove(*i);
                    }
                    for _ in remain {
                        game.add_info_log_item(&format!(
                            "{player_name} may not attack {partner_name} this turn.",
                        ));
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

fn leadership(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Leadership",
        "Gain 1 action.",
        ActionType::cost(ResourcePile::culture_tokens(1)),
        move |_game, _player, _| true,
    )
    .tactics_card(tactics_card)
    .add_simple_persistent_event_listener(
        |e| &mut e.play_action_card,
        0,
        |game, _player_index, player_name, _| {
            game.add_info_log_item(&format!("{player_name} used Leadership to gain an action."));
            game.actions_left += 1;
        },
    )
    .build()
}

fn assassination(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Assassination",
        "Select a player (not affected by Assassination already) \
        to lose an action in their next turn.",
        ActionType::cost(ResourcePile::culture_tokens(1)),
        move |game, p, _| !opponents_not_affected_by_assassination(game, p.index).is_empty(),
    )
    .tactics_card(tactics_card)
    .add_player_request(
        |e| &mut e.play_action_card,
        0,
        |game, player_index, _| {
            Some(PlayerRequest::new(
                opponents_not_affected_by_assassination(game, player_index),
                "Select a player to assassinate",
            ))
        },
        |game, s, _| {
            game.permanent_effects
                .push(PermanentEffect::Assassination(s.choice));
            game.add_info_log_item(&format!(
                "{} has been assassinated by {}.",
                game.player_name(s.choice),
                s.player_name
            ));
        },
    )
    .build()
}

fn opponents_not_affected_by_assassination(game: &Game, player_index: usize) -> Vec<usize> {
    game.players
        .iter()
        .filter(|p| {
            let pi = p.index;
            pi != player_index
                && p.is_human()
                && !game
                    .permanent_effects
                    .iter()
                    .any(|e| is_assassinated(e, pi))
        })
        .map(|p| p.index)
        .collect()
}

pub(crate) fn use_assassination() -> Builtin {
    Builtin::builder("Assassination", "")
        .add_simple_persistent_event_listener(
            |e| &mut e.turn_start,
            2,
            |game, player_index, player_name, ()| {
                if remove_element_by(&mut game.permanent_effects, |e| {
                    is_assassinated(e, player_index)
                })
                .is_some()
                {
                    game.actions_left -= 1;
                    game.add_info_log_item(&format!(
                        "{player_name} has lost an action due to assassination."
                    ));
                }
            },
        )
        .build()
}

fn is_assassinated(e: &PermanentEffect, player: usize) -> bool {
    matches!(e, PermanentEffect::Assassination(p) if player == *p)
}

fn overproduction(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Overproduction",
        "You may collect from 2 additional tiles this turn. \
        (Cannot combine with Production Focus or another Overproduction.)",
        ActionType::cost(ResourcePile::culture_tokens(1)),
        move |game, _p, _| {
            !game
                .permanent_effects
                .iter()
                .any(|e| matches!(e, PermanentEffect::Collect(_)))
        },
    )
    .tactics_card(tactics_card)
    .add_simple_persistent_event_listener(
        |e| &mut e.play_action_card,
        0,
        |game, _player_index, player_name, _| {
            game.permanent_effects
                .push(PermanentEffect::Collect(CollectEffect::Overproduction));
            game.add_info_log_item(&format!(
                "{player_name} can use Overproduction to collect from 2 additional tiles."
            ));
        },
    )
    .build()
}
