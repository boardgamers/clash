use crate::client_state::{ActiveDialog, StateUpdate};
use crate::payment_ui::{payment_dialog, Payment};
use crate::recruit_unit_ui::RecruitSelection;
use crate::render_context::RenderContext;
use server::action::Action;
use server::city::City;
use server::city_pieces::Building;
use server::construct::Construct;
use server::map::Terrain;
use server::playing_actions::{PlayingAction, Recruit};
use server::position::Position;
use server::recruit::recruit_cost;

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

#[derive(Clone)]
pub enum ConstructionProject {
    Building(Building, Option<Position>),
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
