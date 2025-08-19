use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::{Action, gain_action, lose_action};
use crate::action_card::ActionCard;
use crate::content::ability::Ability;
use crate::content::action_cards::development::collect_special_action;
use crate::content::effects::{CollectEffect, PermanentEffect};
use crate::content::incidents::great_diplomat::{DiplomaticRelations, Negotiations};
use crate::content::persistent_events::PlayerRequest;
use crate::content::tactics_cards::{
    TacticsCardFactory, archers, defensive_formation, encircled, high_ground, martyr, scout,
};
use crate::game::Game;
use crate::log::{add_start_turn_action_if_needed, current_turn_log_without_redo};
use crate::playing_actions::PlayingAction;
use crate::utils::remove_element_by;

pub(crate) fn negotiation_action_cards() -> Vec<ActionCard> {
    vec![
        negotiations(23, scout),
        negotiations(24, martyr),
        leadership(25, high_ground),
        leadership(26, archers),
        assassination(27, high_ground),
        assassination(28, archers),
        mass_production(29, defensive_formation),
        mass_production(30, encircled),
    ]
}

fn negotiations(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Negotiations",
        "At the start of your turn: Select another player. \
        This turn, you may not attack that player. In their next turn, they may not attack you.",
        |c| c.free_action().culture_tokens(1),
        move |game, _player, _| {
            !current_turn_log_without_redo(game)
                .actions
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
        |game, player, _| {
            Some(PlayerRequest::new(
                game.players
                    .iter()
                    .filter(|p| p.index != player.index && p.is_human())
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
            s.log(
                game,
                &format!("Started negotiations with {}", game.player_name(s.choice)),
            );
        },
    )
    .tactics_card(tactics_card)
    .build()
}

pub(crate) fn use_negotiations() -> Ability {
    Ability::builder("Negotiations", "")
        .add_simple_persistent_event_listener(
            |e| &mut e.turn_start,
            1,
            |game, p, ()| {
                if let Some(negotiations_partner) = negotiations_partner(game, p.index) {
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
                    add_start_turn_action_if_needed(game);
                    // must be in reverse order to not mess up the indices during deletion
                    for i in delete.iter().rev() {
                        p.log(game, &format!("May attack {partner_name} again.",));
                        game.permanent_effects.remove(*i);
                    }
                    for _ in remain {
                        p.log(game, &format!("May not attack {partner_name} this turn.",));
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
        |c| c.free_action().culture_tokens(1),
        move |_game, _player, _| true,
    )
    .tactics_card(tactics_card)
    .add_simple_persistent_event_listener(
        |e| &mut e.play_action_card,
        0,
        |game, p, _| {
            gain_action(game, p);
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
        |c| c.free_action().culture_tokens(1),
        move |game, p, _| !opponents_not_affected_by_assassination(game, p.index).is_empty(),
    )
    .tactics_card(tactics_card)
    .add_player_request(
        |e| &mut e.play_action_card,
        0,
        |game, p, _| {
            Some(PlayerRequest::new(
                opponents_not_affected_by_assassination(game, p.index),
                "Select a player to assassinate",
            ))
        },
        |game, s, _| {
            game.permanent_effects
                .push(PermanentEffect::AssassinationLoseAction(s.choice));
            s.log(
                game,
                &format!("Assassinated {}", game.player_name(s.choice),),
            );
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

pub(crate) fn use_assassination() -> Ability {
    Ability::builder("Assassination", "")
        .add_simple_persistent_event_listener(
            |e| &mut e.turn_start,
            2,
            |game, p, ()| {
                if remove_element_by(&mut game.permanent_effects, |e| is_assassinated(e, p.index))
                    .is_some()
                {
                    add_start_turn_action_if_needed(game);
                    lose_action(game, p);
                }
            },
        )
        .build()
}

fn is_assassinated(e: &PermanentEffect, player: usize) -> bool {
    matches!(e, PermanentEffect::AssassinationLoseAction(p) if player == *p)
}

fn mass_production(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Mass Production",
        "You may collect from 2 additional tiles this turn. \
        (Cannot combine with Production Focus or another Mass Production.)",
        |c| c.free_action().no_resources(),
        move |game, p, _| collect_special_action(game, p),
    )
    .tactics_card(tactics_card)
    .add_simple_persistent_event_listener(
        |e| &mut e.play_action_card,
        0,
        |game, p, _| {
            game.permanent_effects
                .push(PermanentEffect::Collect(CollectEffect::MassProduction));
            p.log(
                game,
                "Can use Mass Production to collect from 2 additional tiles.",
            );
        },
    )
    .build()
}
