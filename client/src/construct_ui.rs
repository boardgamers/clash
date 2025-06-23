use crate::client_state::{ActiveDialog, RenderResult, StateUpdate};
use crate::payment_ui::{Payment, payment_dialog};
use crate::recruit_unit_ui::RecruitSelection;
use crate::render_context::RenderContext;
use server::action::Action;
use server::city::City;
use server::city_pieces::Building;
use server::construct::Construct;
use server::player_events::CostInfo;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::recruit::Recruit;

pub(crate) fn pay_construction_dialog(
    rc: &RenderContext,
    cp: &ConstructionPayment,
) -> RenderResult {
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
                let recruit = Recruit::new(&r.amount.units, cp.city_position, payment)
                    .with_replaced_units(&r.replaced_units);
                StateUpdate::execute_activation(
                    Action::Playing(PlayingAction::Recruit(recruit)),
                    vec![],
                    city,
                )
            }
        },
    )
}

#[derive(Clone, Debug)]
pub(crate) enum ConstructionProject {
    Building(Building, Option<Position>),
    Units(RecruitSelection),
}

#[derive(Clone, Debug)]
pub(crate) struct ConstructionPayment {
    pub city_position: Position,
    pub project: ConstructionProject,
    pub payment: Payment<String>,
}

impl ConstructionPayment {
    pub(crate) fn new(
        rc: &RenderContext,
        city: &City,
        name: &str,
        project: ConstructionProject,
        cost: &CostInfo,
    ) -> ConstructionPayment {
        ConstructionPayment {
            city_position: city.position,
            project,
            payment: rc.new_payment(&cost.cost, name.to_string(), name, false),
        }
    }
}
