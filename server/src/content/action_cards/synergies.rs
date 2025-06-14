use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{ActionCard, ActionCardBuilder, CivilCardTarget, discard_action_card};
use crate::advance::{Advance, gain_advance_without_payment};
use crate::card::HandCard;
use crate::content::ability::Ability;
use crate::content::action_cards::inspiration;
use crate::content::advances::theocracy::cities_that_can_add_units;
use crate::content::persistent_events::{
    AdvanceRequest, HandCardsRequest, PaymentRequest, PlayerRequest, PositionRequest,
};
use crate::content::tactics_cards::{
    TacticsCardFactory, archers, defensive_formation, flanking, high_ground, high_morale, surprise,
    wedge_formation,
};
use crate::game::Game;
use crate::objective_card::{deinit_objective_card, gain_objective_card};
use crate::player::{Player, gain_unit};
use crate::playing_actions::ActionCost;
use crate::resource_pile::ResourcePile;
use crate::unit::UnitType;
use crate::utils::{Shuffle, remove_element, remove_element_by};
use inspiration::possible_inspiration_advances;
use itertools::Itertools;

pub(crate) fn synergies_action_cards() -> Vec<ActionCard> {
    vec![
        new_plans(31, flanking),
        new_plans(32, high_morale),
        synergies(33, defensive_formation),
        synergies(34, archers),
        teach_us(35, defensive_formation),
        teach_us(36, archers),
        militia(37, wedge_formation),
        militia(38, high_morale),
        tech_trade(39, surprise),
        tech_trade(40, high_ground),
        new_ideas(41, high_morale),
        new_ideas(42, flanking),
    ]
}

fn new_plans(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "New Plans",
        "Draw 2 objective cards. \
        You may discard an objective card from your hand to keep 1 of them. \
        Reshuffle the discarded and not taken cards into the deck.",
        ActionCost::regular_with_cost(ResourcePile::culture_tokens(1)),
        move |_game, p, _| !p.objective_cards.is_empty(),
    )
    .add_hand_card_request(
        |e| &mut e.play_action_card,
        0,
        |game, p, _| {
            game.information_revealed();
            let mut new_cards = game.objective_cards_left.iter().take(2).collect_vec();
            new_cards.extend(&p.get(game).objective_cards);
            Some(HandCardsRequest::new(
                new_cards
                    .iter()
                    .map(|c| HandCard::ObjectiveCard(**c))
                    .collect_vec(),
                0..=2,
                "Select objective cards to draw and discard",
            ))
        },
        |game, s, _i| {
            match s.choice.len() {
                0 => {
                    s.log(game, "Selected none of the objective cards.");
                }
                2 => {
                    swap_objective_card(game, s.player_index, &s.choice);
                }
                _ => panic!("illegal selection"),
            }
            game.objective_cards_left.shuffle(&mut game.rng);
        },
    )
    .tactics_card(tactics_card)
    .build()
}

fn swap_objective_card(game: &mut Game, player: usize, hand_cards: &[HandCard]) {
    let p = game.player(player);
    game.add_info_log_item(&format!(
        "{p} discarded an objective card to draw a new one.",
    ));
    let mut ids = hand_cards
        .iter()
        .map(|c| {
            let HandCard::ObjectiveCard(id) = c else {
                panic!("not an objective card")
            };
            id
        })
        .collect_vec();
    let p = game.player_mut(player);
    let discarded = remove_element_by(&mut p.objective_cards, |c| ids.contains(&c))
        .expect("discarded objective card");
    deinit_objective_card(game, player, discarded);
    game.objective_cards_left.push(discarded);

    remove_element(&mut ids, &&discarded).expect("discarded objective card");
    let gained = ids[0];
    remove_element(&mut game.objective_cards_left, gained).expect("gained objective card");
    gain_objective_card(game, player, *gained);
}

pub(crate) fn validate_new_plans(cards: &[HandCard], game: &Game) -> Result<(), String> {
    match cards.len() {
        0 => Ok(()),
        2 => {
            let ids = &game
                .player(game.current_event().player.index)
                .objective_cards;
            if cards
                .iter()
                .filter(|c| {
                    if let HandCard::ObjectiveCard(id) = c {
                        ids.contains(id)
                    } else {
                        false
                    }
                })
                .count()
                == 1
            {
                Ok(())
            } else {
                Err("must select 1 card from your hand".to_string())
            }
        }
        _ => Err("must select 0 or 2 cards".to_string()),
    }
}

fn synergies(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    let mut b = ActionCard::builder(
        id,
        "Synergies",
        "Gain 2 advances from the same category without changing the Game Event counter. \
        Pay the price as usual.",
        ActionCost::regular(),
        move |game, p, _| !categories_with_2_affordable_advances(p, game).is_empty(),
    )
    .tactics_card(tactics_card)
    .add_advance_request(
        |e| &mut e.play_action_card,
        3,
        |game, p, _| {
            Some(AdvanceRequest::new(categories_with_2_affordable_advances(
                p.get(game),
                game,
            )))
        },
        |game, s, i| {
            let advance = &s.choice;
            s.log(
                game,
                &format!("Selected {} as first advance", advance.name(game)),
            );
            i.selected_advance = Some(*advance);
        },
    );
    b = pay_for_advance(b, 2);
    b = b.add_advance_request(
        |e| &mut e.play_action_card,
        1,
        |game, p, i| {
            let first = i.selected_advance.expect("advance not found");
            Some(AdvanceRequest::new(
                game.cache
                    .get_advance_groups()
                    .iter()
                    .find(|g| g.advances.iter().any(|a| a.advance == first))
                    .expect("Advance group not found")
                    .advances
                    .iter()
                    .filter(|a| p.get(game).can_advance(a.advance, game))
                    .map(|a| a.advance)
                    .collect_vec(),
            ))
        },
        |game, s, i| {
            let advance = &s.choice;
            s.log(
                game,
                &format!("Selected {} as second advance", advance.name(game)),
            );
            i.selected_advance = Some(*advance);
        },
    );
    b = pay_for_advance(b, 0);

    b.build()
}

fn pay_for_advance(b: ActionCardBuilder, priority: i32) -> ActionCardBuilder {
    b.add_payment_request_listener(
        |e| &mut e.play_action_card,
        priority,
        |game, player, i| {
            let p = player.get(game);
            let advance = i.selected_advance.expect("advance not found");
            Some(vec![PaymentRequest::mandatory(
                p.advance_cost(advance, game, game.execute_cost_trigger())
                    .cost,
                &format!("Pay for {}", advance.name(game)),
            )])
        },
        |game, s, i| {
            let advance = i.selected_advance.expect("advance not found");
            gain_advance_without_payment(game, advance, s.player_index, s.choice[0].clone(), false);
        },
    )
}

fn categories_with_2_affordable_advances(p: &Player, game: &Game) -> Vec<Advance> {
    game.cache
        .get_advance_groups()
        .iter()
        .flat_map(|g| {
            let vec = g
                .advances
                .iter()
                .filter(|a| !p.has_advance(a.advance))
                .collect_vec();
            if vec.len() < 2 {
                return vec![];
            }
            vec.iter()
                .permutations(2)
                .filter(|pair| {
                    let a = pair[0];
                    let b = pair[1];
                    let mut cost = p
                        .advance_cost(a.advance, game, game.execute_cost_trigger())
                        .cost;
                    cost.default += p
                        .advance_cost(b.advance, game, game.execute_cost_trigger())
                        .cost
                        .default;
                    p.can_afford(&cost)
                        && p.can_advance_free(a.advance, game)
                        && p.can_advance_free(b.advance, game)
                })
                .map(|pair| pair[0].advance)
                .collect_vec()
        })
        .collect()
}

fn teach_us(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Teach Us",
        "If you just captured a city: Gain 1 advance from the loser for free \
         without changing the Game Event counter.",
        ActionCost::free(),
        // is played by "use_teach_us"
        |_game, _player, _a| false,
    )
    .tactics_card(tactics_card)
    .build()
}

pub(crate) fn use_teach_us() -> Ability {
    // this action card is special - it's played directly after a battle - like objective cards
    Ability::builder(
        "Teach us",
        "If you just captured a city: Gain 1 advance from the loser for free \
             without changing the Game Event counter.",
    )
    .add_hand_card_request(
        |e| &mut e.combat_end,
        91,
        |game, p, s| {
            let player = p.index;
            s.selected_card = None;
            if s.is_winner(player)
                && s.battleground.is_city()
                && !teachable_advances(s.opponent_player(player, game), game.player(player), game)
                    .is_empty()
            {
                let p = game.player(player);
                let cards = p
                    .action_cards
                    .iter()
                    .filter(|a| game.cache.get_action_card(**a).civil_card.name == "Teach Us")
                    .map(|a| HandCard::ActionCard(*a))
                    .collect_vec();
                return Some(HandCardsRequest::new(cards, 0..=1, "Select Teach Us card"));
            }
            None
        },
        |game, s, e| {
            if s.choice.is_empty() {
                return;
            }
            let HandCard::ActionCard(id) = s.choice[0] else {
                panic!("Teach Us card not found");
            };
            discard_action_card(game, s.player_index, id);

            s.log(game, "Activate");
            e.selected_card = Some(id);
        },
    )
    .add_advance_request(
        |e| &mut e.combat_end,
        90,
        |game, player, e| {
            e.selected_card.map(|_| {
                AdvanceRequest::new(teachable_advances(
                    e.opponent_player(player.index, game),
                    player.get(game),
                    game,
                ))
            })
        },
        |game, s, _| {
            let advance = s.choice;
            s.log(game, &format!("Selected {} as advance", advance.name(game)));
            gain_advance_without_payment(
                game,
                advance,
                s.player_index,
                ResourcePile::empty(),
                false,
            );
        },
    )
    .build()
}

pub(crate) fn teachable_advances(teacher: &Player, student: &Player, game: &Game) -> Vec<Advance> {
    teacher
        .advances
        .iter()
        .filter(|a| student.can_advance_free(*a, game))
        .collect()
}

fn militia(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Militia",
        "Gain 1 infantry in one of your cities.",
        ActionCost::cost(ResourcePile::culture_tokens(1)),
        |_game, player, _a| {
            player.available_units().infantry > 0 && !cities_that_can_add_units(player).is_empty()
        },
    )
    .tactics_card(tactics_card)
    .add_position_request(
        |e| &mut e.play_action_card,
        0,
        |game, p, _| {
            let player = p.get(game);
            let cities = cities_that_can_add_units(player);
            Some(PositionRequest::new(
                cities,
                1..=1,
                "Select city to add infantry",
            ))
        },
        |game, s, _| {
            let position = s.choice[0];
            let city = position;
            s.log(game, &format!("Selected city {city}",));

            gain_unit(s.player_index, position, UnitType::Infantry, game);
        },
    )
    .build()
}

fn tech_trade(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Technology Trade",
        "Gain 1 advance for free (without changing the Game Event counter) \
                that a player owns who has a unit or city within range 2 of your units or cities. \
                Then that player gains 1 advance from you for free the same way.",
        ActionCost::cost(ResourcePile::culture_tokens(1)),
        |game, player, _a| !possible_inspiration_advances(game, player).is_empty(),
    )
    .tactics_card(tactics_card)
    .target(CivilCardTarget::AllPlayers)
    .add_player_request(
        |e| &mut e.play_action_card,
        1,
        |game, p, a| {
            if a.active_player != Some(p.index) {
                // only active player can select the target player
                return None;
            }
            let player = p.get(game);
            let choices = game
                .players
                .iter()
                .filter(|teacher| !teachable_advances(teacher, player, game).is_empty())
                .map(|p| p.index)
                .collect();
            Some(PlayerRequest::new(
                choices,
                "Select player to trade advances with",
            ))
        },
        |game, s, a| {
            let p = s.choice;
            s.log(
                game,
                &format!("Selected {} as trade partner", game.player_name(p)),
            );
            a.selected_player = Some(p);
        },
    )
    .add_advance_request(
        |e| &mut e.play_action_card,
        0,
        |game, p, a| {
            let player_index = p.index;
            if a.active_player == Some(player_index) || a.selected_player == Some(player_index) {
                let trade_partner = if a.active_player == Some(player_index) {
                    a.selected_player
                } else {
                    a.active_player
                }
                .expect("target player not found");
                return Some(AdvanceRequest::new(teachable_advances(
                    game.player(trade_partner),
                    game.player(player_index),
                    game,
                )));
            }
            None
        },
        |game, s, _| {
            let advance = s.choice;
            s.log(
                game,
                &format!(
                    "Selected {} as advance for Technology Trade.",
                    advance.name(game)
                ),
            );
            gain_advance_without_payment(
                game,
                advance,
                s.player_index,
                ResourcePile::empty(),
                false,
            );
        },
    )
    .build()
}

fn new_ideas(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    let b = ActionCard::builder(
        id,
        "New Ideas",
        "Gain 1 advance for the regular price (without changing the Game Event counter), \
        then gain 2 ideas.",
        ActionCost::regular(),
        |game, player, _a| !advances_that_can_be_gained(player, game).is_empty(),
    )
    .tactics_card(tactics_card)
    .add_advance_request(
        |e| &mut e.play_action_card,
        2,
        |game, p, _| {
            let player = p.get(game);
            Some(AdvanceRequest::new(advances_that_can_be_gained(
                player, game,
            )))
        },
        |game, s, i| {
            let advance = &s.choice;
            s.log(
                game,
                &format!("Selected {} as advance for New Ideas.", advance.name(game)),
            );
            i.selected_advance = Some(*advance);
        },
    );
    pay_for_advance(b, 1)
        .add_simple_persistent_event_listener(
            |e| &mut e.play_action_card,
            0,
            |game, p, _| {
                p.gain_resources(game, ResourcePile::ideas(2));
            },
        )
        .build()
}

fn advances_that_can_be_gained(player: &Player, game: &Game) -> Vec<Advance> {
    game.cache
        .get_advances()
        .iter()
        .filter(|a| player.can_advance(a.advance, game))
        .map(|a| a.advance)
        .collect()
}
