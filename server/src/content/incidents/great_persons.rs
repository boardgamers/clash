use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{ActionCard, ActionCardBuilder, gain_action_card_from_pile};
use crate::action_cost::{ActionCostBuilder, ActionCostOncePerTurn};
use crate::advance::gain_advance_without_payment;
use crate::card::{HandCard, HandCardLocation, log_card_transfer};
use crate::city::{MoodState, set_city_mood};
use crate::city_pieces::Building;
use crate::construct::{Construct, execute_construct};
use crate::consts::MAX_HUMAN_PLAYERS;
use crate::content::advances::{AdvanceGroup, economy, get_governments_uncached};
use crate::content::effects::{GreatSeerEffect, GreatSeerObjective, PermanentEffect};
use crate::content::incidents::great_builders::{great_architect, great_engineer};
use crate::content::incidents::great_diplomat::{choose_diplomat_partner, great_diplomat};
use crate::content::incidents::great_explorer::great_explorer;
use crate::content::incidents::great_warlord::great_warlord;
use crate::content::persistent_events::{
    AdvanceRequest, HandCardsRequest, PaymentRequest, PositionRequest,
};
use crate::game::Game;
use crate::incident::{Incident, IncidentBaseEffect, IncidentBuilder};
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::player_events::IncidentTarget;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::utils::{format_list, remove_element};
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
        incident(IncidentBaseEffect::PiratesSpawnAndRaid, great_seer(), |b| b),
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

    let b = Incident::builder(id, &name, &civil_card.description.clone(), base)
        .with_action_card(action_card)
        .add_incident_payment_request(
            IncidentTarget::AllPlayers,
            10,
            move |game, player, incident| {
                let cost = if incident.active_player == player.index {
                    1
                } else {
                    2
                };
                let p = game.player(player.index);
                let options = player
                    .payment_options()
                    .resources(p, ResourcePile::culture_tokens(cost));
                if p.can_afford(&options) {
                    Some(vec![PaymentRequest::optional(
                        options,
                        "Pay to gain the Action Card",
                    )])
                } else {
                    player.log(game, &format!("Cannot afford to buy {name}"));
                    None
                }
            },
            move |game, s, i| {
                let pile = &s.choice[0];
                if pile.is_empty() {
                    s.log(game, "Declined to gain the Action Card");
                    return;
                }
                game.player_mut(s.player_index).action_cards.push(card_id);
                log_card_transfer(
                    game,
                    &HandCard::ActionCard(card_id),
                    HandCardLocation::Incident,
                    HandCardLocation::Hand(s.player_index),
                    &s.origin,
                );
                i.selected_player = Some(s.player_index);
            },
        )
        // can add listeners in between for on_gain effect
        .add_simple_incident_listener(
            IncidentTarget::AllPlayers,
            0,
            move |_game, _player_index, i| {
                if i.selected_player.is_some() {
                    i.consumed = true;
                }
            },
        );
    on_gain(b).build()
}

pub(crate) fn great_person_action_card<F>(
    incident_id: u8,
    name: &str,
    description: &str,
    cost: impl Fn(ActionCostBuilder) -> ActionCostOncePerTurn + Send + Sync + 'static,
    free_advance_groups: Vec<AdvanceGroup>,
    can_play: F,
) -> ActionCardBuilder
where
    F: Fn(&Game, &Player) -> bool + 'static + Sync + Send,
{
    ActionCard::builder(
        incident_id + GREAT_PERSON_OFFSET,
        name,
        description,
        cost,
        move |game, player, _| can_play(game, player),
    )
    .add_advance_request(
        |e| &mut e.play_action_card,
        10,
        move |game, player, _| {
            let p = player.get(game);
            let choices = free_advance_groups
                .iter()
                .flat_map(|g| &game.cache.get_advance_group(*g).advances)
                .filter(|a| p.can_advance_free(a.advance, game))
                .map(|a| a.advance)
                .collect();
            Some(AdvanceRequest::new(choices))
        },
        |game, s, _| {
            let name = s.choice;
            s.log(game, &format!("Gain {}", name.name(game)));
            gain_advance_without_payment(game, name, s.player_index, ResourcePile::empty(), false);
        },
    )
}

fn great_artist() -> ActionCard {
    let groups = vec![AdvanceGroup::Culture];
    great_person_action_card(
        19,
        "Great Artist",
        &format!(
            "{} Then, you make one of your cities Happy.",
            great_person_description(&groups)
        ),
        |c| c.action().no_resources(),
        groups,
        |_game, _player| true,
    )
    .add_position_request(
        |e| &mut e.play_action_card,
        0,
        |game, p, _| {
            let player = p.get(game);
            let cities = player
                .cities
                .iter()
                .filter(|c| c.mood_state != MoodState::Happy)
                .map(|c| c.position)
                .collect_vec();
            if cities.is_empty() {
                p.log(game, "No cities to make happy");
            }
            let needed = 1..=1;
            Some(PositionRequest::new(cities, needed, "Make a city Happy"))
        },
        |game, s, _| {
            set_city_mood(game, s.choice[0], &s.origin, MoodState::Happy);
        },
    )
    .build()
}

fn great_prophet() -> ActionCard {
    let groups = vec![AdvanceGroup::Spirituality];
    great_person_action_card(
        20,
        "Great Prophet",
        &format!(
            "{} Then, you build a Temple without activating the city.",
            great_person_description(&groups)
        ),
        |c| c.action().no_resources(),
        groups,
        |_game, _player| true,
    )
    .add_position_request(
        |e| &mut e.play_action_card,
        1,
        |game, p, _| {
            let player = p.get(game);
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
                p.log(game, "No cities can build a Temple");
            }
            let needed = 0..=1;
            Some(PositionRequest::new(cities, needed, "Build a Temple"))
        },
        |game, s, a| {
            let pos = s.choice.first().copied();
            if let Some(pos) = pos {
                s.log(game, &format!("Decided to build a Temple at {pos}",));
            } else {
                s.log(game, "Declined to build a Temple");
            }
            a.selected_position = pos;
        },
    )
    .add_payment_request_listener(
        |e| &mut e.play_action_card,
        0,
        |game, player, a| {
            a.selected_position?;
            Some(vec![PaymentRequest::optional(
                temple_cost(game, player.get(game)),
                "Pay to build the Temple",
            )])
        },
        |game, s, a| {
            let pile = s.choice[0].clone();
            if pile.is_empty() {
                s.log(game, "Declined to build the Temple");
                return;
            }

            let pos = a.selected_position.expect("position not found");

            let () = execute_construct(
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
    player
        .building_cost(game, Building::Temple, game.execute_cost_trigger())
        .cost
}

pub(crate) fn great_person_description(free_advance_groups: &[AdvanceGroup]) -> String {
    format!(
        "{GREAT_PERSON_DESCRIPTION} You may advance \
        in any {} technology for free and without changing the Game Event counter.",
        format_list(
            &free_advance_groups
                .iter()
                .map(std::string::ToString::to_string)
                .collect_vec(),
            "no",
            "or"
        )
    )
}

fn great_philosopher() -> ActionCard {
    let groups = vec![AdvanceGroup::Education];
    great_person_action_card(
        21,
        "Great Philosopher",
        &format!("{} Then, gain 2 ideas.", great_person_description(&groups)),
        |c| c.action().no_resources(),
        groups,
        |_game, _player| true,
    )
    .add_simple_persistent_event_listener(
        |e| &mut e.play_action_card,
        0,
        |game, p, _| {
            p.gain_resources(game, ResourcePile::ideas(2));
        },
    )
    .build()
}

fn great_scientist() -> ActionCard {
    let groups = vec![AdvanceGroup::Science];
    great_person_action_card(
        22,
        "Great Scientist",
        &format!(
            "{} Then, gain 1 idea and 1 Action Card.",
            great_person_description(&groups)
        ),
        |c| c.action().no_resources(),
        groups,
        |_game, _player| true,
    )
    .add_simple_persistent_event_listener(
        |e| &mut e.play_action_card,
        0,
        |game, p, _| {
            p.gain_resources(game, ResourcePile::ideas(1));
            gain_action_card_from_pile(game, p.index, &p.origin);
        },
    )
    .build()
}

fn elder_statesman() -> ActionCard {
    let groups = get_governments_uncached() // cache is not ready yet
        .into_iter()
        .map(|a| a.advance_group)
        .collect_vec();
    great_person_action_card(
        23,
        "Elder Statesman",
        &format!(
            "{} Then, draw 2 Action Cards.",
            great_person_description(&groups)
        ),
        |c| c.action().no_resources(),
        groups,
        |_game, _player| true,
    )
    .add_simple_persistent_event_listener(
        |e| &mut e.play_action_card,
        0,
        |game, p, _| {
            gain_action_card_from_pile(game, p.index, &p.origin);
            gain_action_card_from_pile(game, p.index, &p.origin);
        },
    )
    .build()
}

fn great_merchant() -> ActionCard {
    let groups = vec![AdvanceGroup::Economy];
    economy::add_trade_routes(
        great_person_action_card(
            25,
            "Great Merchant",
            &format!(
                "{} Then, if you have the Trade Routes advance, gain the Trade Routes income.",
                great_person_description(&groups)
            ),
            |c| c.action().no_resources(),
            groups,
            |_game, _player| true,
        ),
        |e| &mut e.play_action_card,
    )
    .build()
}

fn great_athlete() -> ActionCard {
    let groups = vec![AdvanceGroup::Culture];
    great_person_action_card(
        56,
        "Great Athlete",
        &format!(
            "{} Then, you may convert any amount of culture tokens to mood tokens or vice versa.",
            great_person_description(&groups)
        ),
        |c| c.action().no_resources(),
        groups,
        |_game, player| player.resources.culture_tokens > 0 || player.resources.mood_tokens > 0,
    )
    .add_bool_request(
        |e| &mut e.play_action_card,
        1,
        |game, p, _| {
            let p = p.get(game);
            if p.resources.culture_tokens > 0 && p.resources.mood_tokens > 0 {
                Some("Convert culture to mood tokens?".to_string())
            } else {
                None
            }
        },
        |game, s, a| {
            a.answer = Some(s.choice);
            if s.choice {
                s.log(game, "Decided to convert culture to mood tokens");
            } else {
                s.log(game, "Decided to convert mood to culture tokens");
            }
        },
    )
    .add_payment_request_listener(
        |e| &mut e.play_action_card,
        0,
        |game, player, a| {
            let p = player.get(game);
            let culture_to_mood = if p.resources.culture_tokens > 0 && p.resources.mood_tokens > 0 {
                a.answer.expect("answer not found")
            } else {
                p.resources.culture_tokens > 0
            };
            let options = if culture_to_mood {
                player.payment_options().single_type(
                    p,
                    ResourceType::CultureTokens,
                    0..=p.resources.culture_tokens,
                )
            } else {
                player.payment_options().single_type(
                    p,
                    ResourceType::MoodTokens,
                    0..=p.resources.mood_tokens,
                )
            };

            Some(vec![PaymentRequest::optional(options, "Convert resources")])
        },
        |game, s, _| {
            let from = &s.choice[0];
            if from.is_empty() {
                s.log(game, "Declined to convert culture to mood");
                return;
            }
            let to = if from.culture_tokens > 0 {
                ResourcePile::mood_tokens(from.culture_tokens)
            } else {
                ResourcePile::culture_tokens(from.mood_tokens)
            };
            s.player().gain_resources(game, to);
        },
    )
    .build()
}

fn great_seer() -> ActionCard {
    let mut b = great_person_action_card::<_>(
        58,
        "Great Seer",
        &format!(
            "{GREAT_PERSON_DESCRIPTION} Draw 1 objective card per player. \
            Choose one for each player, including yourself. \
            The next time the player draws an objective card from the pile, \
            they draw the designated card instead.",
        ),
        |c| c.action().no_resources(),
        vec![],
        |_game, _player| true,
    );

    for i in 0..MAX_HUMAN_PLAYERS {
        b = choose_great_seer_cards(b, i);
    }
    b.build()
}

fn choose_great_seer_cards(b: ActionCardBuilder, player_order: usize) -> ActionCardBuilder {
    b.add_hand_card_request(
        |e| &mut e.play_action_card,
        (MAX_HUMAN_PLAYERS - player_order) as i32,
        move |game, p, _| {
            if player_order == 0 {
                game.information_revealed(); // new information revealed about objective cards
            }

            let players = game.human_players(p.index);
            let target = game.player_name(*players.get(player_order)?);
            let cards = game
                .objective_cards_left
                .iter()
                .take(players.len() - player_order)
                .map(|c| HandCard::ObjectiveCard(*c))
                .collect_vec();

            let suffix = if player_order == 0 { " (yourself)" } else { "" };

            Some(HandCardsRequest::new(
                cards,
                1..=1,
                &format!("Select an objective card for player {target}{suffix}"),
            ))
        },
        move |game, s, _| {
            let players = game.human_players(s.player_index);
            let target = players[player_order];
            let HandCard::ObjectiveCard(card) = &s.choice[0] else {
                panic!("Expected an objective card");
            };
            remove_element(&mut game.objective_cards_left, card);
            let objective = GreatSeerObjective {
                player: target,
                objective_card: *card,
            };
            log_card_transfer(
                game,
                &HandCard::ObjectiveCard(*card),
                HandCardLocation::DrawPile,
                HandCardLocation::GreatSeer(target),
                &s.origin,
            );

            if let Some(effect) = find_great_seer(game) {
                effect.assigned_objectives.push(objective);
            } else {
                let e = PermanentEffect::GreatSeer(GreatSeerEffect {
                    player: s.player_index,
                    assigned_objectives: vec![objective],
                });
                game.permanent_effects.push(e);
            }
        },
    )
}

pub(crate) fn find_great_seer(game: &mut Game) -> Option<&mut GreatSeerEffect> {
    game.permanent_effects.iter_mut().find_map(|e| {
        if let PermanentEffect::GreatSeer(p) = e {
            Some(p)
        } else {
            None
        }
    })
}
