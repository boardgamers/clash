use std::cmp;

use macroquad::math::{i32, u32};
use macroquad::ui::Ui;

use crate::city_ui::{CityMenu, IconActionVec};
use server::action::Action;
use server::city::City;
use server::city_pieces::Building;
use server::content::custom_actions::CustomAction;
use server::game::Game;
use server::map::{Map, Terrain};
use server::playing_actions::{Construct, PlayingAction, Recruit};
use server::position::Position;
use server::resource_pile::PaymentOptions;
use server::unit::UnitType;

use crate::client_state::{ActiveDialog, ShownPlayer, State, StateUpdate};
use crate::payment_ui::{payment_dialog, HasPayment, Payment, ResourcePayment};
use crate::recruit_unit_ui::RecruitSelection;
use crate::resource_ui::{new_resource_map, ResourceType};
use crate::select_ui::CountSelector;

pub fn add_construct_button<'a>(
    game: &'a Game,
    state: &'a State,
    menu: &'a CityMenu,
    icons: &'a mut IconActionVec,
    building: &Building,
    name: &str,
) {
    let owner = menu.get_city_owner(game);
    let city = menu.get_city(game);
    if menu.is_city_owner() && menu.player.can_play_action && city.can_construct(building, owner) {
        for pos in building_positions(building, city, &game.map) {
            let tooltip = format!(
                "Built {}{} for {}",
                name,
                pos.map_or(String::new(), |p| format!(" at {}", p)),
                owner.construct_cost(building, city).to_string(),
            );
            icons.push((
                &state.assets.buildings[&building],
                &tooltip,
                Box::new(move || {
                    StateUpdate::SetDialog(ActiveDialog::ConstructionPayment(ConstructionPayment::new(
                        game,
                        name,
                        menu.player.index,
                        menu.city_position,
                        ConstructionProject::Building(building.clone(), pos),
                    )))
                }),
            ));
        }
    }
}

fn building_positions(building: &Building, city: &City, map: &Map) -> Vec<Option<Position>> {
    if building != &Building::Port {
        return vec![None];
    }

    map.tiles
        .iter()
        .filter_map(|(p, t)| {
            if *t == Terrain::Water && city.position.is_neighbor(*p) {
                Some(Some(*p))
            } else {
                None
            }
        })
        .collect()
}

pub fn add_wonder_buttons(game: &Game, menu: &CityMenu, icons: &mut IconActionVec) {
    // todo
    // let city = menu.get_city(game);
    // let owner = menu.get_city_owner(game);
    // for w in &owner.wonder_cards {
    //     if city.can_build_wonder(w, owner, game)
    //         && ui.button(None, format!("Build Wonder {}", w.name))
    //     {
    //         return StateUpdate::SetDialog(ActiveDialog::ConstructionPayment(
    //             ConstructionPayment::new(
    //                 game,
    //                 &w.name,
    //                 menu.player.index,
    //                 menu.city_position,
    //                 ConstructionProject::Wonder(w.name.clone()),
    //             ),
    //         ));
    //     }
    // }
}

pub fn pay_construction_dialog(
    game: &Game,
    payment: &ConstructionPayment,
    player: &ShownPlayer,
) -> StateUpdate {
    payment_dialog(
        player,
        &format!("Pay for {}", payment.name),
        vec![],
        payment,
        |cp| cp.payment.get(ResourceType::Discount).selectable.current == 0,
        |cp| match &cp.project {
            ConstructionProject::Building(b, pos) => StateUpdate::execute_activation(
                Action::Playing(PlayingAction::Construct(Construct {
                    city_position: cp.city_position,
                    city_piece: b.clone(),
                    payment: cp.payment.to_resource_pile(),
                    port_position: *pos,
                    temple_bonus: None,
                })),
                vec![],
                game.get_any_city(cp.city_position).unwrap(),
            ),
            ConstructionProject::Wonder(w) => StateUpdate::execute_activation(
                Action::Playing(PlayingAction::Custom(CustomAction::ConstructWonder {
                    city_position: cp.city_position,
                    payment: cp.payment.to_resource_pile(),
                    wonder: w.clone(),
                })),
                vec![],
                game.get_any_city(cp.city_position).unwrap(),
            ),
            ConstructionProject::Units(r) => StateUpdate::execute_activation(
                Action::Playing(PlayingAction::Recruit(Recruit {
                    city_position: cp.city_position,
                    units: r.amount.units.clone().to_vec(),
                    payment: cp.payment.to_resource_pile(),
                    replaced_units: r.replaced_units.clone(),
                    leader_index: r.amount.leader_index,
                })),
                vec![],
                game.get_any_city(cp.city_position).unwrap(),
            ),
        },
        |ap, r| match r {
            ResourceType::Gold => ap.payment_options.gold_left > 0,
            ResourceType::Discount => ap.payment_options.discount > 0,
            _ => ap.payment.get(r).selectable.max > 0,
        },
        |cp, r| {
            let mut new = cp.clone();
            let gold = new.payment.get_mut(ResourceType::Gold);
            if gold.selectable.current > 0 {
                gold.selectable.current -= 1;
            } else {
                new.payment
                    .get_mut(ResourceType::Discount)
                    .selectable
                    .current += 1;
            }
            new.payment.get_mut(r).selectable.current += 1;
            StateUpdate::SetDialog(ActiveDialog::ConstructionPayment(new))
        },
        |cp, r| {
            let mut new = cp.clone();
            let discount = new.payment.get_mut(ResourceType::Discount);
            if discount.selectable.current > 0 {
                discount.selectable.current -= 1;
            } else {
                new.payment.get_mut(ResourceType::Gold).selectable.current += 1;
            }
            new.payment.get_mut(r).selectable.current -= 1;
            StateUpdate::SetDialog(ActiveDialog::ConstructionPayment(new))
        },
    )
}

#[derive(Clone)]
pub enum ConstructionProject {
    Building(Building, Option<Position>),
    Wonder(String),
    Units(RecruitSelection),
}

#[derive(Clone)]
pub struct ConstructionPayment {
    pub name: String,
    pub player_index: usize,
    pub city_position: Position,
    pub project: ConstructionProject,
    pub payment: Payment,
    pub payment_options: PaymentOptions,
}

impl ConstructionPayment {
    pub fn new(
        game: &Game,
        name: &str,
        player_index: usize,
        city_position: Position,
        project: ConstructionProject,
    ) -> ConstructionPayment {
        let p = game.get_player(player_index);
        let cost = match &project {
            ConstructionProject::Building(b, _) => {
                p.construct_cost(b, p.get_city(city_position).unwrap())
            }
            ConstructionProject::Wonder(name) => p
                .wonder_cards
                .iter()
                .find(|w| w.name == *name)
                .unwrap()
                .cost
                .clone(),
            ConstructionProject::Units(sel) => sel
                .amount
                .units
                .clone()
                .to_vec()
                .iter()
                .map(UnitType::cost)
                .sum(),
        };

        let payment_options = p.resources.get_payment_options(&cost);

        let payment = ConstructionPayment::new_payment(&payment_options);

        ConstructionPayment {
            name: name.to_string(),
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
                    resource: e.0,
                    selectable: CountSelector {
                        current: e.1,
                        min: e.1,
                        max: e.1,
                    },
                },
                _ => ResourcePayment {
                    resource: e.0,
                    selectable: CountSelector {
                        current: e.1,
                        min: cmp::max(0, e.1 as i32 - a.discount as i32 - a.gold_left as i32)
                            as u32,
                        max: e.1,
                    },
                },
            })
            .collect();

        resources.sort_by_key(|r| r.resource);

        Payment { resources }
    }
}

impl HasPayment for ConstructionPayment {
    fn payment(&self) -> &Payment {
        &self.payment
    }
}
