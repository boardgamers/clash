use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{gain_action_card_from_pile, ActionCard, ActionCardBuilder};
use crate::advance::gain_advance;
use crate::city::MoodState;
use crate::city_pieces::Building;
use crate::content::advances;
use crate::content::advances::{economy, get_governments};
use crate::content::custom_phase_actions::{AdvanceRequest, PaymentRequest, PositionRequest};
use crate::content::incidents::great_builder::great_engineer;
use crate::content::incidents::great_explorer::great_explorer;
use crate::content::incidents::great_warlord::great_warlord;
use crate::game::Game;
use crate::incident::{Incident, IncidentBaseEffect};
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::player_events::IncidentTarget;
use crate::playing_actions::{construct, ActionType, Construct};
use crate::resource_pile::ResourcePile;
use crate::utils::format_list;
use itertools::Itertools;
use std::vec;

pub(crate) const GREAT_PERSON_OFFSET: u8 = 100;

pub(crate) const GREAT_PERSON_DESCRIPTION: &str = "You make take the Event Card to your \
 hand for 1 culture token. If you pass, any other player (in player order) may take it \
 for 2 culture tokens: Action card: ";

pub(crate) fn great_person_incidents() -> Vec<Incident> {
    vec![
        great_person_incident(IncidentBaseEffect::ExhaustedLand, great_explorer()),
        great_person_incident(IncidentBaseEffect::BarbariansMove, great_artist()),
        great_person_incident(IncidentBaseEffect::BarbariansMove, great_prophet()),
        great_person_incident(IncidentBaseEffect::BarbariansMove, great_philosopher()),
        great_person_incident(IncidentBaseEffect::BarbariansMove, great_scientist()),
        great_person_incident(IncidentBaseEffect::BarbariansMove, elder_statesman()),
        great_person_incident(IncidentBaseEffect::ExhaustedLand, great_warlord()),
        great_person_incident(IncidentBaseEffect::BarbariansMove, great_merchant()),
        great_person_incident(IncidentBaseEffect::BarbariansMove, great_engineer()),
    ]
}

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
                let options = PaymentOptions::resources(ResourcePile::culture_tokens(cost));
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

pub(crate) fn great_person_action_card<F, S: AsRef<str> + Clone>(
    incident_id: u8,
    name: &str,
    description: &str,
    action_type: ActionType,
    free_advance_groups: &[S],
    can_play: F,
) -> ActionCardBuilder
where
    F: Fn(&Game, &Player) -> bool + 'static,
{
    let groups = free_advance_groups
        .iter()
        .map(|s| s.as_ref().to_string())
        .collect_vec();

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
            let choices = groups
                .iter()
                .flat_map(|g| advances::get_group(g.as_ref()).advances)
                .filter(|a| p.can_advance_free(a))
                .map(|a| a.name.clone())
                .collect();
            Some(AdvanceRequest::new(choices))
        },
        |game, s, _| {
            let name = &s.choice;
            game.add_info_log_item(&format!("{} gained {}", s.player_name, name));
            gain_advance(game, name, s.player_index, ResourcePile::empty(), false);
        },
    )
}

fn great_artist() -> ActionCard {
    let groups = &["Culture"];
    great_person_action_card(
        19,
        "Great Artist",
        &format!(
            "{} Then, you make one of your cities Happy.",
            great_person_description(groups)
        ),
        ActionType::regular(),
        groups,
        |_game, _player| true,
    )
    .add_position_request(
        |e| &mut e.on_play_action_card,
        0,
        |game, player_index, _| {
            let player = game.get_player(player_index);
            let cities = player
                .cities
                .iter()
                .filter(|c| c.mood_state != MoodState::Happy)
                .map(|c| c.position)
                .collect_vec();
            if cities.is_empty() {
                game.add_info_log_item("No cities to make happy");
            }
            Some(PositionRequest::new(cities, 1..=1, "Make a city Happy"))
        },
        |game, s, _| {
            let position = s.choice[0];
            game.add_info_log_item(&format!(
                "{} made city at {} Happy",
                s.player_name, position
            ));
            game.get_player_mut(s.player_index)
                .get_city_mut(position)
                .mood_state = MoodState::Happy;
        },
    )
    .build()
}

fn great_prophet() -> ActionCard {
    let groups = &["Spirituality"];
    great_person_action_card(
        20,
        "Great Prophet",
        &format!(
            "{} Then, you build a Temple without activating the city.",
            great_person_description(groups)
        ),
        ActionType::regular(),
        groups,
        |_game, _player| true,
    )
    .add_position_request(
        |e| &mut e.on_play_action_card,
        1,
        |game, player_index, _| {
            let player = game.get_player(player_index);
            if !player.is_building_available(Building::Temple, game) {
                return None;
            }
            if !player.can_afford(&temple_cost(game, player)) {
                return None;
            }

            let cities = player
                .cities
                .iter()
                .filter(|c| c.pieces.temple.is_none())
                .map(|c| c.position)
                .collect_vec();
            if cities.is_empty() {
                game.add_info_log_item("No cities can build a Temple");
            }
            Some(PositionRequest::new(cities, 0..=1, "Build a Temple"))
        },
        |game, s, a| {
            let pos = s.choice.first().copied();
            if let Some(pos) = pos {
                game.add_info_log_item(&format!(
                    "{} decided to build a Temple at {pos}",
                    s.player_name
                ));
            } else {
                game.add_info_log_item(&format!("{} declined to build a Temple", s.player_name));
            }
            a.selected_position = pos;
        },
    )
    .add_payment_request_listener(
        |e| &mut e.on_play_action_card,
        0,
        |game, player, a| {
            a.selected_position?;
            Some(vec![PaymentRequest::new(
                temple_cost(game, game.get_player(player)),
                "Pay to build the Temple",
                true,
            )])
        },
        |game, s, a| {
            let pile = s.choice[0].clone();
            let name = &s.player_name;
            if pile.is_empty() {
                game.add_info_log_item(&format!("{name} declined to build the Temple"));
                return;
            }

            let pos = a.selected_position.expect("position not found");
            game.add_info_log_item(&format!("{name} built a Temple at {pos} for {pile}",));

            construct(
                game,
                s.player_index,
                &Construct::new(pos, Building::Temple, pile),
            );
        },
    )
    .build()
}

fn temple_cost(game: &Game, player: &Player) -> PaymentOptions {
    player.construct_cost(game, Building::Temple, None).cost
}

pub(crate) fn great_person_description<S: AsRef<str>>(free_advance_groups: &[S]) -> String {
    format!(
        "{GREAT_PERSON_DESCRIPTION} As a regular action, you may advance \
        in any {} technology for free and without changing the Game Event counter.",
        format_list(free_advance_groups, "no", "or")
    )
}

fn great_philosopher() -> ActionCard {
    let groups = &["Education"];
    great_person_action_card(
        21,
        "Great Philosopher",
        &format!("{} Then, gain 2 ideas.", great_person_description(groups)),
        ActionType::regular(),
        groups,
        |_game, _player| true,
    )
    .add_simple_persistent_event_listener(
        |e| &mut e.on_play_action_card,
        0,
        |game, player_index, player_name, _| {
            game.add_info_log_item(&format!("{player_name} gained 2 ideas",));
            game.get_player_mut(player_index)
                .gain_resources(ResourcePile::ideas(2));
        },
    )
    .build()
}

fn great_scientist() -> ActionCard {
    let groups = &["Science"];
    great_person_action_card(
        22,
        "Great Scientist",
        &format!(
            "{} Then, gain 1 idea and 1 Action Card.",
            great_person_description(groups)
        ),
        ActionType::regular(),
        groups,
        |_game, _player| true,
    )
    .add_simple_persistent_event_listener(
        |e| &mut e.on_play_action_card,
        0,
        |game, player_index, player_name, _| {
            game.add_info_log_item(&format!("{player_name} gained 1 idea",));
            game.get_player_mut(player_index)
                .gain_resources(ResourcePile::ideas(1));
            gain_action_card_from_pile(game, player_index);
        },
    )
    .build()
}

fn elder_statesman() -> ActionCard {
    let groups = get_governments()
        .into_iter()
        .map(|a| a.name.clone())
        .collect_vec();
    great_person_action_card(
        23,
        "Elder Statesman",
        &format!(
            "{} Then, draw 2 Action Cards.",
            great_person_description(&groups)
        ),
        ActionType::regular(),
        &groups,
        |_game, _player| true,
    )
    .add_simple_persistent_event_listener(
        |e| &mut e.on_play_action_card,
        0,
        |game, player_index, _player_name, _| {
            gain_action_card_from_pile(game, player_index);
            gain_action_card_from_pile(game, player_index);
        },
    )
    .build()
}

fn great_merchant() -> ActionCard {
    let groups = &["Economy"];
    economy::add_trade_routes(
        great_person_action_card(
            25,
            "Great Merchant",
            &format!(
                "{} Then, if you have the Trade Routes advance, gain the Trade Routes income.",
                great_person_description(groups)
            ),
            ActionType::regular(),
            groups,
            |_game, _player| true,
        ),
        |e| &mut e.on_play_action_card,
    )
    .build()
}
