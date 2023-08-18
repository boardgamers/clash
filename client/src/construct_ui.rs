use macroquad::math::{i32, u32};
use macroquad::ui::Ui;
use server::action::Action;
use server::city::City;
use server::city_pieces::Building;
use server::content::custom_actions::CustomAction;
use server::game::Game;
use server::map::{Map, Terrain};
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::PaymentOptions;
use std::cmp;

use crate::payment_ui::{payment_dialog, HasPayment, Payment, ResourcePayment};
use crate::resource_ui::{new_resource_map, ResourceType};
use crate::ui_state::CityMenu;
use crate::ui_state::{ActiveDialog, ActiveDialogUpdate};

pub fn add_construct_button(
    game: &Game,
    menu: &CityMenu,
    ui: &mut Ui,
    building: &Building,
    name: &str,
) -> Option<ActiveDialog> {
    let owner = menu.get_city_owner(game);
    let city = menu.get_city(game);
    if (menu.is_city_owner()) && city.can_construct(building, owner) {
        for pos in building_positions(building, city, &game.map) {
            if ui.button(
                None,
                format!(
                    "Build {}{}",
                    name,
                    pos.clone().map_or("".to_string(), |p| format!(" at {}", p))
                ),
            ) {
                return Some(ActiveDialog::ConstructionPayment(ConstructionPayment::new(
                    game,
                    menu.player_index,
                    menu.city_position.clone(),
                    ConstructionProject::Building(building.clone(), pos),
                )));
            }
        }
    }
    None
}

fn building_positions(building: &Building, city: &City, map: &Map) -> Vec<Option<Position>> {
    if building != &Building::Port {
        return vec![None];
    }

    map.tiles
        .iter()
        .filter_map(|(p, t)| {
            if *t == Terrain::Water && city.position.is_neighbor(p) {
                Some(Some(p.clone()))
            } else {
                None
            }
        })
        .collect()
}

pub fn add_wonder_buttons(game: &Game, menu: &CityMenu, ui: &mut Ui) -> Option<ActiveDialog> {
    let city = menu.get_city(game);
    let owner = menu.get_city_owner(game);
    for w in owner.wonder_cards.iter() {
        if city.can_build_wonder(w, owner, game)
            && ui.button(None, format!("Build Wonder {}", w.name))
        {
            return Some(ActiveDialog::ConstructionPayment(ConstructionPayment::new(
                game,
                menu.player_index,
                menu.city_position.clone(),
                ConstructionProject::Wonder(w.name.clone()),
            )));
        }
    }
    None
}

pub fn pay_construction_dialog(payment: &mut ConstructionPayment) -> ActiveDialogUpdate {
    payment_dialog(
        payment,
        |cp| cp.payment.get(ResourceType::Discount).current == 0,
        |cp| match &cp.project {
            ConstructionProject::Building(b, pos) => {
                ActiveDialogUpdate::Execute(Action::Playing(PlayingAction::Construct {
                    city_position: cp.city_position.clone(),
                    city_piece: b.clone(),
                    payment: cp.payment.to_resource_pile(),
                    port_position: pos.clone(),
                    temple_bonus: None,
                }))
            }
            ConstructionProject::Wonder(w) => ActiveDialogUpdate::Execute(Action::Playing(
                PlayingAction::Custom(CustomAction::ConstructWonder {
                    city_position: cp.city_position.clone(),
                    payment: cp.payment.to_resource_pile(),
                    wonder: w.clone(),
                }),
            )),
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

pub enum ConstructionProject {
    Building(Building, Option<Position>),
    Wonder(String),
}

pub struct ConstructionPayment {
    pub player_index: usize,
    pub city_position: Position,
    pub project: ConstructionProject,
    pub payment: Payment,
    pub payment_options: PaymentOptions,
}

impl ConstructionPayment {
    pub fn new(
        game: &Game,
        player_index: usize,
        city_position: Position,
        project: ConstructionProject,
    ) -> ConstructionPayment {
        let p = game.get_player(player_index);
        let cost = match &project {
            ConstructionProject::Building(b, _) => {
                p.construct_cost(b, p.get_city(&city_position).unwrap())
            }
            ConstructionProject::Wonder(name) => p
                .wonder_cards
                .iter()
                .find(|w| w.name == *name)
                .unwrap()
                .cost
                .clone(),
        };

        let payment_options = p.resources().get_payment_options(&cost);

        let payment = ConstructionPayment::new_payment(&payment_options);

        ConstructionPayment {
            player_index,
            city_position,
            project,
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
