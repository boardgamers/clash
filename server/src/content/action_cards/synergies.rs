use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::Action;
use crate::action_card::{
    ActionCard, ActionCardBuilder, ActionCardInfo, CivilCardMatch, CivilCardOpportunity,
    CivilCardRequirement,
};
use crate::advance::gain_advance_without_payment;
use crate::content::advances;
use crate::content::advances::get_advance;
use crate::content::advances::theocracy::cities_that_can_add_units;
use crate::content::persistent_events::{
    AdvanceRequest, EventResponse, PaymentRequest, PositionRequest,
};
use crate::content::tactics_cards::{TacticsCardFactory, archers, defensive_formation, wedge_formation, high_morale};
use crate::game::Game;
use crate::log::current_player_turn_log;
use crate::player::Player;
use crate::playing_actions::ActionType;
use crate::resource_pile::ResourcePile;
use crate::unit::UnitType;
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
        |game, player, a| !advances_to_copy_for_loser(game, player, a).is_empty(),
    )
    .requirement(CivilCardRequirement::new(
        vec![CivilCardOpportunity::CaptureCity],
        true,
    ))
    .tactics_card(tactics_card)
    .add_advance_request(
        |e| &mut e.play_action_card,
        0,
        |game, player, a| {
            Some(AdvanceRequest::new(advances_to_copy_for_loser(
                game,
                game.player(player),
                a,
            )))
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

fn advances_to_copy_for_loser(game: &Game, winner: &Player, a: &ActionCardInfo) -> Vec<String> {
    let Some(action_log_index) = a.satisfying_action else {
        panic!("Satisfying action not found");
    };
    let Some(CivilCardMatch {
        opportunity: _,
        played_cards: _,
        opponent: Some(loser),
    }) = &current_player_turn_log(game).items[action_log_index].civil_card_match
    else {
        panic!("Capture city opportunity not found");
    };

    game.player(*loser)
        .advances
        .iter()
        .filter(|a| !winner.has_advance(&a.name) && winner.can_advance_free(a))
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
