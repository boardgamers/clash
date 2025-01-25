use crate::client_state::{ActiveDialog, StateUpdate};
use crate::payment_ui::{new_payment, payment_model_dialog, Payment};
use crate::recruit_unit_ui::RecruitSelection;
use crate::render_context::RenderContext;
use server::action::Action;
use server::city::City;
use server::city_pieces::Building;
use server::content::custom_actions::CustomAction;
use server::map::Terrain;
use server::payment::PaymentModel;
use server::playing_actions::{Construct, PlayingAction, Recruit};
use server::position::Position;
use server::unit::UnitType;

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
    let city = rc.game.get_any_city(cp.city_position).unwrap();
    payment_model_dialog(
        rc,
        &[cp.payment.clone()],
        |p| {
            let mut new = cp.clone();
            new.payment = p[0].clone();
            ActiveDialog::ConstructionPayment(new)
        },
        true,
        |p| match &cp.project {
            ConstructionProject::Building(b, pos) => StateUpdate::execute_activation(
                Action::Playing(PlayingAction::Construct(Construct {
                    city_position: cp.city_position,
                    city_piece: *b,
                    payment: p[0].clone(),
                    port_position: *pos,
                    temple_bonus: None,
                })),
                vec![],
                city,
            ),
            ConstructionProject::Wonder(w) => StateUpdate::execute_activation(
                Action::Playing(PlayingAction::Custom(CustomAction::ConstructWonder {
                    city_position: cp.city_position,
                    payment: p[0].clone(),
                    wonder: w.clone(),
                })),
                vec![],
                city,
            ),
            ConstructionProject::Units(r) => StateUpdate::execute_activation(
                Action::Playing(PlayingAction::Recruit(Recruit {
                    city_position: cp.city_position,
                    units: r.amount.units.clone().to_vec(),
                    payment: p[0].clone(),
                    replaced_units: r.replaced_units.clone(),
                    leader_name: r.amount.leader_name.clone(),
                })),
                vec![],
                city,
            ),
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
            ConstructionProject::Building(b, _) => p.construct_cost(*b, city),
            ConstructionProject::Wonder(name) => p
                .wonder_cards
                .iter()
                .find(|w| w.name == *name)
                .unwrap()
                .cost
                .clone(),
            ConstructionProject::Units(sel) => PaymentModel::resources(
                sel.amount
                    .units
                    .clone()
                    .to_vec()
                    .iter()
                    .map(UnitType::cost)
                    .sum(),
            ),
        };

        let payment = new_payment(&cost, &rc.shown_player.resources, name, false);

        ConstructionPayment {
            player_index: city.player_index,
            city_position: city.position,
            project,
            payment,
        }
    }
}
