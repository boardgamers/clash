use macroquad::ui::Ui;
use server::city_pieces::Building;
use server::game::{Action, Game};
use server::playing_actions::PlayingAction;

use crate::city_ui::{CityMenu, ConstructionPayment};
use crate::payment_ui::{payment_dialog, ResourceType};
use crate::ui::ActiveDialog;

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
