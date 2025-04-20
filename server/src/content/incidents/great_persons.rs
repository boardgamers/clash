use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{ActionCard, ActionCardBuilder, gain_action_card_from_pile};
use crate::advance::gain_advance_without_payment;
use crate::city::MoodState;
use crate::city_pieces::Building;
use crate::construct::{Construct, construct};
use crate::content::advances;
use crate::content::advances::{economy, get_governments_uncached};
use crate::content::incidents::great_builders::{great_architect, great_engineer};
use crate::content::incidents::great_diplomat::{choose_diplomat_partner, great_diplomat};
use crate::content::incidents::great_explorer::great_explorer;
use crate::content::incidents::great_warlord::great_warlord;
use crate::content::persistent_events::{AdvanceRequest, PaymentRequest, PositionRequest};
use crate::game::Game;
use crate::incident::{Incident, IncidentBaseEffect, IncidentBuilder};
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::player_events::IncidentTarget;
use crate::playing_actions::ActionCost;
use crate::resource::ResourceType;
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
        incident(IncidentBaseEffect::ExhaustedLand, great_explorer(), |b| b),
        incident(IncidentBaseEffect::BarbariansMove, great_artist(), |b| b),
        incident(IncidentBaseEffect::BarbariansMove, great_prophet(), |b| b),
        incident(
            IncidentBaseEffect::BarbariansMove,
            great_philosopher(),
            |b| b,
        ),
        incident(IncidentBaseEffect::BarbariansMove, great_scientist(), |b| b),
        incident(IncidentBaseEffect::BarbariansMove, elder_statesman(), |b| b),
        incident(IncidentBaseEffect::ExhaustedLand, great_warlord(), |b| b),
        incident(IncidentBaseEffect::BarbariansMove, great_merchant(), |b| b),
        incident(IncidentBaseEffect::BarbariansMove, great_engineer(), |b| b),
        incident(
            IncidentBaseEffect::PiratesSpawnAndRaid,
            great_architect(),
            |b| b,
        ),
        incident(
            IncidentBaseEffect::PiratesSpawnAndRaid,
            great_athlete(),
            |b| b,
        ),
        incident(IncidentBaseEffect::BarbariansSpawn, great_diplomat(), |b| {
            choose_diplomat_partner(b)
        }),
        //todo add great seer when objective cards are implemented
    ]
}

fn incident(
    base: IncidentBaseEffect,
    action_card: ActionCard,
    on_gain: impl Fn(IncidentBuilder) -> IncidentBuilder,
) -> Incident {
    let card_id = action_card.id;
    let id = card_id - GREAT_PERSON_OFFSET;
    let civil_card = &action_card.civil_card;
    let name = civil_card.name.clone();
    let name2 = name.clone();

    let b = Incident::builder(id, &name, &civil_card.description.clone(), base)
        .with_action_card(action_card)
        .add_incident_payment_request(
            IncidentTarget::AllPlayers,
            10,
            move |game, player_index, incident| {
                let cost = if incident.active_player == player_index {
                    1
                } else {
                    2
                };
                let options = PaymentOptions::resources(ResourcePile::culture_tokens(cost));
                let p = game.player(player_index);
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
                game.player_mut(s.player_index).action_cards.push(card_id);
                i.selected_player = Some(s.player_index);
            },
        )
        // can add listeners in between for on_gain effect
        .add_simple_incident_listener(
            IncidentTarget::AllPlayers,
            0,
            move |_game, _player_index, _player_name, i| {
                if i.selected_player.is_some() {
                    i.consumed = true;
                }
            },
        );
    on_gain(b).build()
}

pub(crate) fn great_person_action_card<F, S: AsRef<str> + Clone>(
    incident_id: u8,
    name: &str,
    description: &str,
    action_type: ActionCost,
    free_advance_groups: &[S],
    can_play: F,
) -> ActionCardBuilder
where
    F: Fn(&Game, &Player) -> bool + 'static + Sync + Send,
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
        move |game, player, _| can_play(game, player),
    )
    .add_advance_request(
        |e| &mut e.play_action_card,
        10,
        move |game, player_index, _| {
            let p = game.player(player_index);
            let choices = groups
                .iter()
                .flat_map(|g| &advances::get_group(g.as_ref()).advances)
                .filter(|a| p.can_advance_free(a.advance))
                .map(|a| a.advance)
                .collect();
            Some(AdvanceRequest::new(choices))
        },
        |game, s, _| {
            let name = s.choice;
            game.add_info_log_item(&format!("{} gained {}", s.player_name, name));
            gain_advance_without_payment(game, name, s.player_index, ResourcePile::empty(), false);
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
        ActionCost::regular(),
        groups,
        |_game, _player| true,
    )
    .add_position_request(
        |e| &mut e.play_action_card,
        0,
        |game, player_index, _| {
            let player = game.player(player_index);
            let cities = player
                .cities
                .iter()
                .filter(|c| c.mood_state != MoodState::Happy)
                .map(|c| c.position)
                .collect_vec();
            if cities.is_empty() {
                game.add_info_log_item("No cities to make happy");
            }
            let needed = 1..=1;
            Some(PositionRequest::new(cities, needed, "Make a city Happy"))
        },
        |game, s, _| {
            let position = s.choice[0];
            game.add_info_log_item(&format!(
                "{} made city at {} Happy",
                s.player_name, position
            ));
            game.player_mut(s.player_index)
                .get_city_mut(position)
                .set_mood_state(MoodState::Happy);
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
        ActionCost::regular(),
        groups,
        |_game, _player| true,
    )
    .add_position_request(
        |e| &mut e.play_action_card,
        1,
        |game, player_index, _| {
            let player = game.player(player_index);
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
            let needed = 0..=1;
            Some(PositionRequest::new(cities, needed, "Build a Temple"))
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
        |e| &mut e.play_action_card,
        0,
        |game, player, a| {
            a.selected_position?;
            Some(vec![PaymentRequest::new(
                temple_cost(game, game.player(player)),
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

            let () = construct(
                game,
                s.player_index,
                &Construct::new(pos, Building::Temple, pile),
            )
            .expect("Cannot build Temple");
        },
    )
    .build()
}

fn temple_cost(game: &Game, player: &Player) -> PaymentOptions {
    player.building_cost(game, Building::Temple, None).cost
}

pub(crate) fn great_person_description<S: AsRef<str>>(free_advance_groups: &[S]) -> String {
    format!(
        "{GREAT_PERSON_DESCRIPTION} You may advance \
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
        ActionCost::regular(),
        groups,
        |_game, _player| true,
    )
    .add_simple_persistent_event_listener(
        |e| &mut e.play_action_card,
        0,
        |game, player_index, player_name, _| {
            game.add_info_log_item(&format!("{player_name} gained 2 ideas",));
            game.player_mut(player_index)
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
        ActionCost::regular(),
        groups,
        |_game, _player| true,
    )
    .add_simple_persistent_event_listener(
        |e| &mut e.play_action_card,
        0,
        |game, player_index, player_name, _| {
            game.add_info_log_item(&format!("{player_name} gained 1 idea",));
            game.player_mut(player_index)
                .gain_resources(ResourcePile::ideas(1));
            gain_action_card_from_pile(game, player_index);
        },
    )
    .build()
}

fn elder_statesman() -> ActionCard {
    let groups = get_governments_uncached() // cache is not ready yet
        .iter()
        .map(|a| a.name.clone())
        .collect_vec();
    great_person_action_card(
        23,
        "Elder Statesman",
        &format!(
            "{} Then, draw 2 Action Cards.",
            great_person_description(&groups)
        ),
        ActionCost::regular(),
        &groups,
        |_game, _player| true,
    )
    .add_simple_persistent_event_listener(
        |e| &mut e.play_action_card,
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
            ActionCost::regular(),
            groups,
            |_game, _player| true,
        ),
        |e| &mut e.play_action_card,
    )
    .build()
}

fn great_athlete() -> ActionCard {
    let groups = &["Culture"];
    great_person_action_card(
        56,
        "Great Athlete",
        &format!(
            "{} Then, you may convert any amount of culture tokens to mood tokens or vice versa.",
            great_person_description(groups)
        ),
        ActionCost::regular(),
        groups,
        |_game, player| player.resources.culture_tokens > 0 || player.resources.mood_tokens > 0,
    )
    .add_bool_request(
        |e| &mut e.play_action_card,
        1,
        |game, player_index, _| {
            let p = game.player(player_index);
            if p.resources.culture_tokens > 0 && p.resources.mood_tokens > 0 {
                Some("Convert culture to mood tokens?".to_string())
            } else {
                None
            }
        },
        |game, s, a| {
            a.answer = Some(s.choice);
            if s.choice {
                game.add_info_log_item(&format!(
                    "{} decided to convert culture to mood tokens",
                    s.player_name
                ));
            } else {
                game.add_info_log_item(&format!(
                    "{} decided to convert mood to culture tokens",
                    s.player_name
                ));
            }
        },
    )
    .add_payment_request_listener(
        |e| &mut e.play_action_card,
        0,
        |game, player, a| {
            let p = game.player(player);
            let culture_to_mood = if p.resources.culture_tokens > 0 && p.resources.mood_tokens > 0 {
                a.answer.expect("answer not found")
            } else {
                p.resources.culture_tokens > 0
            };
            let options = if culture_to_mood {
                PaymentOptions::single_type(
                    ResourceType::CultureTokens,
                    0..=p.resources.culture_tokens,
                )
            } else {
                PaymentOptions::single_type(ResourceType::MoodTokens, 0..=p.resources.mood_tokens)
            };

            Some(vec![PaymentRequest::new(
                options,
                "Convert resources",
                true,
            )])
        },
        |game, s, _| {
            let from = &s.choice[0];
            if from.is_empty() {
                game.add_info_log_item(&format!(
                    "{} declined to convert culture to mood",
                    s.player_name
                ));
                return;
            }
            let to = if from.culture_tokens > 0 {
                ResourcePile::mood_tokens(from.culture_tokens)
            } else {
                ResourcePile::culture_tokens(from.mood_tokens)
            };
            game.add_info_log_item(&format!("{} converted {from} mood to {to}", s.player_name));
            game.player_mut(s.player_index).gain_resources(to);
        },
    )
    .build()
}
