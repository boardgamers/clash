use macroquad::math::{i32, u32};
use macroquad::ui::Ui;
use server::city_pieces::Building;
use server::game::{Action, Game};
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::PaymentOptions;
use std::cmp;

use crate::payment_ui::{
    new_resource_map, payment_dialog, HasPayment, Payment, ResourcePayment, ResourceType,
};
use crate::ui_state::ActiveDialog;
use crate::ui_state::CityMenu;

pub fn add_construct_button(
    game: &Game,
    menu: &CityMenu,
    ui: &mut Ui,
    building: &Building,
    name: &str,
) -> Option<ActiveDialog> {
    let owner = menu.get_city_owner(game);
    let city = menu.get_city(game);
    if (menu.is_city_owner())
        && city.can_construct(building, owner)
        && ui.button(None, format!("Build {}", name))
    {
        return Some(ActiveDialog::ConstructionPayment(ConstructionPayment::new(
            game,
            menu.player_index,
            menu.city_position.clone(),
            building.clone(),
        )));
    }
    None
}

pub fn pay_construction_dialog(game: &mut Game, payment: &mut ConstructionPayment) -> bool {
    payment_dialog(
        payment,
        |cp| cp.payment.get(ResourceType::Discount).current == 0,
        |cp| {
            game.execute_action(
                Action::PlayingAction(PlayingAction::Construct {
                    city_position: cp.city_position.clone(),
                    city_piece: cp.city_piece.clone(),
                    payment: cp.payment.to_resource_pile(),
                    temple_bonus: None,
                }),
                cp.player_index,
            )
        },
        |ap, r| match r {
            ResourceType::Gold => ap.payment_options.gold_left > 0,
            ResourceType::Discount => ap.payment_options.discount > 0,
            _ => ap.payment.get(r).max > 0,
        },
        |cp, r| {
            let gold = cp.payment.get_mut(ResourceType::Gold);
            if gold.current > 0 {
                gold.current -= 1;
            } else {
                cp.payment.get_mut(ResourceType::Discount).current += 1;
            }
            cp.payment.get_mut(r).current += 1;
        },
        |cp, r| {
            let discount = cp.payment.get_mut(ResourceType::Discount);
            if discount.current > 0 {
                discount.current -= 1;
            } else {
                cp.payment.get_mut(ResourceType::Gold).current += 1;
            }
            cp.payment.get_mut(r).current -= 1;
        },
    )
}

pub struct ConstructionPayment {
    pub player_index: usize,
    pub city_position: Position,
    pub city_piece: Building,
    pub payment: Payment,
    pub payment_options: PaymentOptions,
}

impl ConstructionPayment {
    pub fn new(
        game: &Game,
        player_index: usize,
        city_position: Position,
        city_piece: Building,
    ) -> ConstructionPayment {
        let p = game.get_player(player_index);
        let cost = p.construct_cost(&city_piece, p.get_city(&city_position).unwrap());
        let payment_options = p.resources().get_payment_options(&cost);

        let payment = ConstructionPayment::new_payment(&payment_options);

        ConstructionPayment {
            player_index,
            city_position,
            city_piece,
            payment,
            payment_options,
        }
    }

    pub fn new_payment(a: &PaymentOptions) -> Payment {
        let mut resources: Vec<ResourcePayment> = new_resource_map(&a.default)
            .into_iter()
            .map(|e| match e.0 {
                ResourceType::Discount | ResourceType::Gold => ResourcePayment {
                    resource: e.0.clone(),
                    current: e.1,
                    min: e.1,
                    max: e.1,
                },
                _ => ResourcePayment {
                    resource: e.0.clone(),
                    current: e.1,
                    min: cmp::max(0, e.1 as i32 - a.discount as i32 - a.gold_left as i32) as u32,
                    max: e.1,
                },
            })
            .collect();

        resources.sort_by_key(|r| r.resource.clone());

        Payment { resources }
    }
}

impl HasPayment for ConstructionPayment {
    fn payment(&self) -> &Payment {
        &self.payment
    }
}
