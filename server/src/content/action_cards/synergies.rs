use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::Action;
use crate::action_card::{ActionCard, ActionCardBuilder};
use crate::advance::{gain_advance_without_payment};
use crate::content::advances;
use crate::content::advances::get_advance;
use crate::content::persistent_events::{AdvanceRequest, EventResponse, PaymentRequest};
use crate::content::tactics_cards::{TacticsCardFactory, archers, defensive_formation};
use crate::game::Game;
use crate::log::current_player_turn_log;
use crate::player::Player;
use crate::playing_actions::ActionType;
use itertools::Itertools;

pub(crate) fn synergies_action_cards() -> Vec<ActionCard> {
    vec![
        //todo add "New Plans" when objective cards are implemented
        synergies(33, defensive_formation),
        synergies(34, archers),
    ]
}

fn synergies(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    let mut b = ActionCard::builder(
        id,
        "Synergies",
        "Gain 2 advances from the same category without changing the Game Event counter. \
        Pay the price as usual.",
        ActionType::regular(),
        move |_game, p| !categories_with_2_affordable_advances(p).is_empty(),
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
            game.add_info_log_item(
                &format!("{} paid {} for advance {advance}", s.player_name, s.choice[0])
            );
            gain_advance_without_payment(
                game,
                &advance,
                s.player_index,
                s.choice[0].clone(),
                false,
            )
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
