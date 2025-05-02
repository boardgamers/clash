use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{ActionCard, ActionCardBuilder, CivilCardTarget, discard_action_card};
use crate::advance::{Advance, gain_advance_without_payment};
use crate::card::HandCard;
use crate::content::action_cards::inspiration;
use crate::content::advances::theocracy::cities_that_can_add_units;
use crate::content::builtin::Builtin;
use crate::content::persistent_events::{
    AdvanceRequest, HandCardsRequest, PaymentRequest, PlayerRequest, PositionRequest,
};
use crate::content::tactics_cards::{
    TacticsCardFactory, archers, defensive_formation, flanking, high_ground, high_morale, surprise,
    wedge_formation,
};
use crate::game::Game;
use crate::player::{Player, add_unit};
use crate::playing_actions::ActionCost;
use crate::resource_pile::ResourcePile;
use crate::unit::UnitType;
use inspiration::possible_inspiration_advances;
use itertools::Itertools;

pub(crate) fn synergies_action_cards() -> Vec<ActionCard> {
    vec![
        //todo add "New Plans" when objective cards are implemented
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
        |game, p, _,_| {
            Some(AdvanceRequest::new(categories_with_2_affordable_advances(
                game.player(p),
                game,
            )))
        },
        |game, sel, i,_| {
            let advance = &sel.choice;
            game.add_info_log_item(&format!(
                "{} selected {} as first advance for Synergies.",
                sel.player_name,
                advance.name(game)
            ));
            i.selected_advance = Some(*advance);
        },
    );
    b = pay_for_advance(b, 2);
    b = b.add_advance_request(
        |e| &mut e.play_action_card,
        1,
        |game, p, i,_| {
            let first = i.selected_advance.expect("advance not found");
            Some(AdvanceRequest::new(
                game.cache
                    .get_advance_groups()
                    .iter()
                    .find(|g| g.advances.iter().any(|a| a.advance == first))
                    .expect("Advance group not found")
                    .advances
                    .iter()
                    .filter(|a| game.player(p).can_advance(a.advance, game))
                    .map(|a| a.advance)
                    .collect_vec(),
            ))
        },
        |game, sel, i,_| {
            let advance = &sel.choice;
            game.add_info_log_item(&format!(
                "{} selected {} as second advance for Synergies.",
                sel.player_name,
                advance.name(game)
            ));
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
        |game, player_index, i| {
            let p = game.player(player_index);
            let advance = i.selected_advance.expect("advance not found");
            Some(vec![PaymentRequest::mandatory(
                p.advance_cost(advance, game, game.execute_cost_trigger())
                    .cost,
                &format!("Pay for {}", advance.name(game)),
            )])
        },
        |game, s, i| {
            let advance = i.selected_advance.expect("advance not found");
            game.add_info_log_item(&format!(
                "{} paid {} for advance {}",
                s.player_name,
                s.choice[0],
                advance.name(game),
            ));
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

pub(crate) fn use_teach_us() -> Builtin {
    // this action card is special - it's played directly after a battle - like objective cards
    Builtin::builder(
        "Teach us",
        "If you just captured a city: Gain 1 advance from the loser for free \
             without changing the Game Event counter.",
    )
    .add_hand_card_request(
        |e| &mut e.combat_end,
        91,
        |game, player_index, e,info| {
            let stats = &e.combat.stats;

            if info.owning_player == player_index && stats.is_winner(player_index) && stats.battleground.is_city() {
                let p = game.player(player_index);
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
        |game, s, e,_| {
            if s.choice.is_empty() {
                return;
            }
            let HandCard::ActionCard(id) = s.choice[0] else {
                panic!("Teach Us card not found");
            };
            discard_action_card(game, s.player_index, id);

            game.add_info_log_item(&format!("{} selected to use Teach Us.", s.player_name));
            e.selected_card = Some(id);
        },
    )
    .add_advance_request(
        |e| &mut e.combat_end,
        90,
        |game, player, e, _| {
            e.selected_card.map(|_| {
                let vec = teachable_advances(
                    game.player(e.combat.opponent(player)),
                    game.player(player),
                    game,
                );
                AdvanceRequest::new(vec)
            })
        },
        |game, sel, _,_| {
            let advance = sel.choice;
            game.add_info_log_item(&format!(
                "{} selected {} as advance for Teach Us.",
                sel.player_name,
                advance.name(game)
            ));
            gain_advance_without_payment(
                game,
                advance,
                sel.player_index,
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
        |game, player_index, _,_| {
            let player = game.player(player_index);
            let cities = cities_that_can_add_units(player);
            Some(PositionRequest::new(
                cities,
                1..=1,
                "Select city to add infantry",
            ))
        },
        |game, s, _,_| {
            let position = s.choice[0];
            let city = position;
            game.add_info_log_item(&format!(
                "{} selected {} as city for Militia.",
                s.player_name, city
            ));

            add_unit(s.player_index, position, UnitType::Infantry, game);
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
        |game, player_index, a,_| {
            if a.active_player != Some(player_index) {
                // only active player can select the target player
                return None;
            }
            let player = game.player(player_index);
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
        |game, s, a,_| {
            let p = s.choice;
            game.add_info_log_item(&format!(
                "{} selected {} as player for Technology Trade.",
                s.player_name,
                game.player_name(p)
            ));
            a.selected_player = Some(p);
        },
    )
    .add_advance_request(
        |e| &mut e.play_action_card,
        0,
        |game, player_index, a,_| {
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
        |game, sel, _,_| {
            let advance = sel.choice;
            game.add_info_log_item(&format!(
                "{} selected {} as advance for Technology Trade.",
                sel.player_name,
                advance.name(game)
            ));
            gain_advance_without_payment(
                game,
                advance,
                sel.player_index,
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
        |game, player_index, _,_| {
            let player = game.player(player_index);
            Some(AdvanceRequest::new(advances_that_can_be_gained(
                player, game,
            )))
        },
        |game, sel, i,_| {
            let advance = &sel.choice;
            game.add_info_log_item(&format!(
                "{} selected {} as advance for New Ideas.",
                sel.player_name,
                advance.name(game)
            ));
            i.selected_advance = Some(*advance);
        },
    );
    pay_for_advance(b, 1)
        .add_simple_persistent_event_listener(
            |e| &mut e.play_action_card,
            0,
            |game, player_index, player_name, _| {
                game.add_info_log_item(&format!("{player_name} used gain 2 ideas from New Ideas."));
                game.player_mut(player_index)
                    .gain_resources(ResourcePile::ideas(2));
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
