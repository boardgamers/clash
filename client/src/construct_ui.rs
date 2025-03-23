use crate::action_buttons::{base_or_custom_action, base_or_custom_available};
use crate::cards_ui::wonder_cards;
use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::{BaseOrCustomAction, BaseOrCustomDialog};
use crate::payment_ui::{payment_dialog, Payment};
use crate::recruit_unit_ui::RecruitSelection;
use crate::render_context::RenderContext;
use server::action::Action;
use server::city::City;
use server::city_pieces::Building;
use server::construct::Construct;
use server::content::custom_actions::{CustomAction, CustomActionType};
use server::events::EventOrigin;
use server::map::Terrain;
use server::playing_actions::{PlayingAction, PlayingActionType, Recruit};
use server::position::Position;
use server::recruit::recruit_cost;
use server::wonder::{ConstructWonder, Wonder};

pub fn new_building_positions(
    building: Building,
    rc: &RenderContext,
    city: &City,
) -> Vec<(Building, Option<Position>)> {
    if building != Building::Port {
        return vec![(building, None)];
    }

    rc.game
        .map
        .tiles
        .iter()
        .filter_map(|(p, t)| {
            if *t == Terrain::Water && city.position.is_neighbor(*p) {
                Some((building, Some(*p)))
            } else {
                None
            }
        })
        .collect()
}

pub fn pay_construction_dialog(rc: &RenderContext, cp: &ConstructionPayment) -> StateUpdate {
    let city = rc.game.get_any_city(cp.city_position);
    payment_dialog(
        rc,
        &cp.payment.clone(),
        true,
        |p| {
            let mut new = cp.clone();
            new.payment = p;
            ActiveDialog::ConstructionPayment(new)
        },
        |payment| match cp.project.clone() {
            ConstructionProject::Building(b, pos) => StateUpdate::execute_activation(
                Action::Playing(PlayingAction::Construct(
                    Construct::new(cp.city_position, b, payment).with_port_position(pos),
                )),
                vec![],
                city,
            ),
            ConstructionProject::Wonder(w, custom) => StateUpdate::execute_activation(
                construct_wonder_action(ConstructWonder::new(cp.city_position, w, payment), custom),
                vec![],
                city,
            ),
            ConstructionProject::Units(r) => StateUpdate::execute_activation(
                Action::Playing(PlayingAction::Recruit(Recruit {
                    city_position: cp.city_position,
                    units: r.amount.units.clone(),
                    payment,
                    replaced_units: r.replaced_units.clone(),
                    leader_name: r.amount.leader_name.clone(),
                })),
                vec![],
                city,
            ),
        },
    )
}

fn construct_wonder_action(c: ConstructWonder, custom: BaseOrCustomDialog) -> Action {
    match custom.custom {
        BaseOrCustomAction::Base => Action::Playing(PlayingAction::ConstructWonder(c)),
        BaseOrCustomAction::Custom { .. } => {
            Action::Playing(PlayingAction::Custom(CustomAction::GreatArchitect(c)))
        }
    }
}

#[derive(Clone)]
pub enum ConstructionProject {
    Building(Building, Option<Position>),
    Wonder(String, BaseOrCustomDialog),
    Units(RecruitSelection),
}

#[derive(Clone)]
pub struct ConstructionPayment {
    pub player_index: usize,
    pub city_position: Position,
    pub project: ConstructionProject,
    pub payment: Payment,
}

impl ConstructionPayment {
    pub fn new(
        rc: &RenderContext,
        city: &City,
        name: &str,
        project: ConstructionProject,
    ) -> ConstructionPayment {
        let p = rc.game.get_player(city.player_index);
        let cost = match &project {
            ConstructionProject::Building(b, _) => p.construct_cost(rc.game, *b, None),
            ConstructionProject::Wonder(name, _) => p.wonder_cost(
                wonder_cards(p).iter().find(|w| w.name == *name).unwrap(),
                city,
                None,
            ),
            ConstructionProject::Units(sel) => recruit_cost(
                rc.shown_player,
                &sel.amount.units,
                city.position,
                sel.amount.leader_name.as_ref(),
                &sel.replaced_units,
                None,
            )
            .unwrap(),
        }
        .cost;

        let payment = rc.new_payment(&cost, name, false);
        ConstructionPayment {
            player_index: city.player_index,
            city_position: city.position,
            project,
            payment,
        }
    }
}

pub fn can_play_construct_wonder(rc: &RenderContext) -> bool {
    base_or_custom_available(
        rc,
        &PlayingActionType::ConstructWonder,
        &CustomActionType::GreatArchitect,
    )
}

pub fn open_construct_wonder_dialog(rc: &RenderContext, city: &City, w: &Wonder) -> StateUpdate {
    base_or_custom_action(
        rc,
        &PlayingActionType::ConstructWonder,
        "Construct Wonder",
        &[(EventOrigin::Builtin("Great Architect".to_string()), CustomActionType::GreatArchitect)],
        |custom| {
            ActiveDialog::ConstructionPayment(ConstructionPayment::new(
                rc,
                city,
                &w.name,
                ConstructionProject::Wonder(w.name.clone(), custom),
            ))
        },
    )
}
