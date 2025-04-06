use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::Action;
use crate::action_card::{ActionCard, ActionCardBuilder, CivilCardTarget, discard_action_card};
use crate::advance::gain_advance_without_payment;
use crate::card::HandCard;
use crate::content::action_cards::{get_action_card, inspiration};
use crate::content::advances;
use crate::content::advances::get_advance;
use crate::content::advances::theocracy::cities_that_can_add_units;
use crate::content::builtin::Builtin;
use crate::content::persistent_events::{
    AdvanceRequest, EventResponse, HandCardsRequest, PaymentRequest, PlayerRequest, PositionRequest,
};
use crate::content::tactics_cards::{
    TacticsCardFactory, archers, defensive_formation, flanking, high_ground, high_morale, surprise,
    wedge_formation,
};
use crate::game::Game;
use crate::log::current_player_turn_log;
use crate::player::Player;
use crate::playing_actions::ActionType;
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
        ActionType::regular(),
        move |_game, p, _| !categories_with_2_affordable_advances(p).is_empty(),
    )
    .tactics_card(tactics_card)
    .add_advance_request(
        |e| &mut e.play_action_card,
        3,
        |game, p, _| {
            Some(AdvanceRequest::new(categories_with_2_affordable_advances(
                game.player(p),
            )))
        },
        |game, sel, _| {
            game.add_info_log_item(&format!(
                "{} selected {} as first advance for Synergies.",
                sel.player_name, sel.choice
            ));
        },
    );
    b = pay_for_advance(b, 2);
    b = b.add_advance_request(
        |e| &mut e.play_action_card,
        1,
        |game, p, _| {
            let first = last_advance(game);
            Some(AdvanceRequest::new(
                advances::get_groups()
                    .iter()
                    .find(|g| g.advances.iter().any(|a| a.name == first))
                    .expect("Advance group not found")
                    .advances
                    .iter()
                    .filter(|a| game.player(p).can_advance(a))
                    .map(|a| a.name.clone())
                    .collect_vec(),
            ))
        },
        |game, sel, _| {
            game.add_info_log_item(&format!(
                "{} selected {} as second advance for Synergies.",
                sel.player_name, sel.choice
            ));
        },
    );
    b = pay_for_advance(b, 0);

    b.build()
}

fn pay_for_advance(b: ActionCardBuilder, priority: i32) -> ActionCardBuilder {
    b.add_payment_request_listener(
        |e| &mut e.play_action_card,
        priority,
        |game, player_index, _| {
            let p = game.player(player_index);
            let a = last_advance(game);
            Some(vec![PaymentRequest::new(
                p.advance_cost(&get_advance(&a), None).cost,
                &format!("Pay for {a}"),
                false,
            )])
        },
        |game, s, _| {
            let advance = last_advance(game);
            game.add_info_log_item(&format!(
                "{} paid {} for advance {advance}",
                s.player_name, s.choice[0]
            ));
            gain_advance_without_payment(
                game,
                &advance,
                s.player_index,
                s.choice[0].clone(),
                false,
            );
        },
    )
}

fn last_advance(game: &Game) -> String {
    for i in current_player_turn_log(game).items.iter().rev() {
        if let Action::Response(EventResponse::SelectAdvance(a)) = &i.action {
            return a.clone();
        }
    }
    panic!("Advance action not found");
}

fn categories_with_2_affordable_advances(p: &Player) -> Vec<String> {
    advances::get_groups()
        .iter()
        .flat_map(|g| {
            let vec = g
                .advances
                .iter()
                .filter(|a| !p.has_advance(&a.name))
                .collect_vec();
            if vec.len() < 2 {
                return vec![];
            }
            vec.iter()
                .permutations(2)
                .filter(|pair| {
                    let a = pair[0];
                    let b = pair[1];
                    let mut cost = p.advance_cost(a, None).cost;
                    cost.default += p.advance_cost(b, None).cost.default;
                    p.can_afford(&cost) && p.can_advance_free(a) && p.can_advance_free(b)
                })
                .map(|pair| pair[0].name.clone())
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
        ActionType::free(),
        // is played by "use_teach_us"
        |_game, _player, _a| true,
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
        |game, player_index, e| {
            let stats = &e.combat.stats;
            if stats.is_winner(player_index) && stats.battleground.is_city() {
                let p = game.player(player_index);
                let cards = p
                    .action_cards
                    .iter()
                    .filter(|a| get_action_card(**a).civil_card.name == "Teach Us")
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

            game.add_info_log_item(&format!("{} selected to use Teach Us.", s.player_name));
            e.selected_card = Some(id);
        },
    )
    .add_advance_request(
        |e| &mut e.combat_end,
        90,
        |game, player, e| {
            e.selected_card.map(|_| {
                let vec =
                    teachable_advances(game.player(e.combat.opponent(player)), game.player(player));
                AdvanceRequest::new(vec)
            })
        },
        |game, sel, _| {
            let advance = &sel.choice;
            game.add_info_log_item(&format!(
                "{} selected {advance} as advance for Teach Us.",
                sel.player_name,
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

pub(crate) fn teachable_advances(teacher: &Player, student: &Player) -> Vec<String> {
    teacher
        .advances
        .iter()
        .filter(|a| !student.has_advance(&a.name) && student.can_advance_free(a))
        .map(|a| a.name.clone())
        .collect()
}

fn militia(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Militia",
        "Gain 1 infantry in one of your cities.",
        ActionType::cost(ResourcePile::culture_tokens(1)),
        |_game, player, _a| {
            player.available_units().infantry > 0 && !cities_that_can_add_units(player).is_empty()
        },
    )
    .tactics_card(tactics_card)
    .add_position_request(
        |e| &mut e.play_action_card,
        0,
        |game, player_index, _| {
            let player = game.player(player_index);
            let cities = cities_that_can_add_units(player);
            Some(PositionRequest::new(
                cities,
                1..=1,
                "Select city to add infantry",
            ))
        },
        |game, s, _| {
            let city = s.choice[0];
            game.add_info_log_item(&format!(
                "{} selected {} as city for Militia.",
                s.player_name, city
            ));

            game.player_mut(s.player_index)
                .add_unit(city, UnitType::Infantry);
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
        ActionType::cost(ResourcePile::culture_tokens(1)),
        |game, player, _a| !possible_inspiration_advances(game, player).is_empty(),
    )
    .tactics_card(tactics_card)
    .target(CivilCardTarget::AllPlayers)
    .add_player_request(
        |e| &mut e.play_action_card,
        1,
        |game, player_index, a| {
            if a.active_player != Some(player_index) {
                // only active player can select the target player
                return None;
            }
            let player = game.player(player_index);
            let choices = game
                .players
                .iter()
                .filter(|teacher| !teachable_advances(teacher, player).is_empty())
                .map(|p| p.index)
                .collect();
            Some(PlayerRequest::new(
                choices,
                "Select player to trade advances with",
            ))
        },
        |game, s, a| {
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
        |game, player_index, a| {
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
                )));
            }
            None
        },
        |game, sel, _| {
            let advance = &sel.choice;
            game.add_info_log_item(&format!(
                "{} selected {advance} as advance for Technology Trade.",
                sel.player_name,
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
        ActionType::regular(),
        |_game, player, _a| !advances_that_can_be_gained(player).is_empty(),
    )
    .tactics_card(tactics_card)
    .add_advance_request(
        |e| &mut e.play_action_card,
        2,
        |game, player_index, _| {
            let player = game.player(player_index);
            Some(AdvanceRequest::new(advances_that_can_be_gained(player)))
        },
        |game, sel, _| {
            let advance = &sel.choice;
            game.add_info_log_item(&format!(
                "{} selected {advance} as advance for New Ideas.",
                sel.player_name,
            ));
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

fn advances_that_can_be_gained(player: &Player) -> Vec<String> {
    advances::get_all()
        .iter()
        .filter(|a| !player.has_advance(&a.name) && player.can_advance(a))
        .map(|a| a.name.clone())
        .collect()
}
