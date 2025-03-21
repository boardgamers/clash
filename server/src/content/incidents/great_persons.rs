use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{ActionCard, ActionCardBuilder};
use crate::advance::gain_advance;
use crate::content::advances;
use crate::content::custom_phase_actions::{AdvanceRequest, PaymentRequest};
use crate::content::incidents::great_explorer::great_explorer;
use crate::game::Game;
use crate::incident::{Incident, IncidentBaseEffect};
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::player_events::IncidentTarget;
use crate::playing_actions::ActionType;
use crate::resource_pile::ResourcePile;
use std::vec;

pub(crate) fn great_person_incidents() -> Vec<Incident> {
    vec![great_person_incident(
        IncidentBaseEffect::ExhaustedLand,
        great_explorer(),
    )]
}

pub(crate) const GREAT_PERSON_OFFSET: u8 = 100;

fn great_person_incident(base: IncidentBaseEffect, action_card: ActionCard) -> Incident {
    let card_id = action_card.id;
    let id = card_id - GREAT_PERSON_OFFSET;
    let civil_card = &action_card.civil_card;
    let name = civil_card.name.clone();
    let name2 = name.clone();

    Incident::builder(id, &name, &civil_card.description.clone(), base)
        .with_action_card(action_card)
        .add_incident_payment_request(
            IncidentTarget::AllPlayers,
            0,
            move |game, player_index, incident| {
                let cost = if incident.active_player == player_index {
                    1
                } else {
                    2
                };
                let options = PaymentOptions::resources(ResourcePile::ideas(cost));
                let p = game.get_player(player_index);
                if p.can_afford(&options) {
                    Some(vec![PaymentRequest::new(
                        options,
                        "Pay to gain the Action Card",
                        true,
                    )])
                } else {
                    game.add_info_log_item(&format!(
                        "{} cannot afford to buy {name}",
                        p.get_name()
                    ));
                    None
                }
            },
            move |game, s, i| {
                let pile = &s.choice[0];
                if pile.is_empty() {
                    game.add_info_log_item(&format!(
                        "{} declined to gain the Action Card",
                        s.player_name
                    ));
                    return;
                }
                game.add_info_log_item(&format!("{} gained {name2} for {pile}", s.player_name));
                game.get_player_mut(s.player_index)
                    .action_cards
                    .push(card_id);
                i.consumed = true;
            },
        )
        .build()
}

pub(crate) fn great_person_action_card<F>(
    incident_id: u8,
    name: &str,
    description: &str,
    action_type: ActionType,
    free_advance_groups: &'static [&'static str],
    can_play: F,
) -> ActionCardBuilder
where
    F: Fn(&Game, &Player) -> bool + 'static,
{
    ActionCard::builder(
        incident_id + GREAT_PERSON_OFFSET,
        name,
        description,
        action_type,
        can_play,
    )
    .add_advance_request(
        |e| &mut e.on_play_action_card,
        10,
        move |game, player_index, _| {
            let p = game.get_player(player_index);
            Some(AdvanceRequest::new(
                free_advance_groups
                    .iter()
                    .flat_map(|g| advances::get_group(g).advances)
                    .filter(|a| p.can_advance_free(a))
                    .map(|a| a.name.clone())
                    .collect(),
            ))
        },
        |game, s, _| {
            let name = &s.choice;
            game.add_info_log_item(&format!("{} gained {}", s.player_name, name));
            gain_advance(game, name, s.player_index, ResourcePile::empty(), false);
        },
    )
}
