use crate::client_state::{ActiveDialog, StateUpdate};
use crate::payment_ui::{Payment, payment_dialog};
use crate::recruit_unit_ui::RecruitSelection;
use crate::render_context::RenderContext;
use server::action::Action;
use server::city::City;
use server::city_pieces::Building;
use server::construct::Construct;
use server::playing_actions::{PlayingAction, Recruit};
use server::position::Position;
use server::recruit::recruit_cost;

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
            ConstructionProject::Units(r) => {
                let mut recruit = Recruit::new(&r.amount.units, cp.city_position, payment)
                    .with_replaced_units(&r.replaced_units);
                if let Some(l) = &r.amount.leader_name {
                    recruit = recruit.with_leader(l);
                }

                StateUpdate::execute_activation(
                    Action::Playing(PlayingAction::Recruit(recruit)),
                    vec![],
                    city,
                )
            }
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
        let p = rc.game.player(city.player_index);
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
