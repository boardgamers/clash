use crate::action_buttons::base_or_custom_action;
use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::BaseOrCustomDialog;
use crate::payment_ui::{Payment, payment_dialog};
use crate::render_context::RenderContext;
use server::city::City;
use server::happiness::{happiness_action, increase_happiness_cost};
use server::player::Player;
use server::playing_actions::{IncreaseHappiness, PlayingActionType};
use server::position::Position;

#[derive(Clone)]
pub struct IncreaseHappinessConfig {
    pub steps: Vec<(Position, u32)>,
    pub payment: Payment,
    pub custom: BaseOrCustomDialog,
}

impl IncreaseHappinessConfig {
    pub fn new(p: &Player, custom: BaseOrCustomDialog) -> IncreaseHappinessConfig {
        let steps = p.cities.iter().map(|c| (c.position, 0)).collect();
        IncreaseHappinessConfig {
            steps,
            payment: Self::happiness_payment(p, &[(p.cities[0].position, 0)], &custom),
            custom,
        }
    }

    fn happiness_payment(p: &Player, new_steps: &[(Position, u32)], custom: &BaseOrCustomDialog) -> Payment {
        let payment = new_steps
            .iter()
            .map(|(pos, steps)| {
                let city = p.get_city(*pos);
                increase_happiness_cost(p, city, *steps).unwrap().cost
            })
            .reduce(|mut a, b| {
                a.default += b.default;
                a
            })
            .unwrap();

        let mut pile = p.resources.clone();
        pile -= custom.action_type.cost().cost; 
        Payment::new(&payment, &pile, "Increase happiness", false)
    }
}

pub fn open_increase_happiness_dialog(
    rc: &RenderContext,
    actions: Vec<PlayingActionType>,
    init: impl Fn(IncreaseHappinessConfig) -> IncreaseHappinessConfig,
) -> StateUpdate {
    base_or_custom_action(rc, actions, "Increase happiness", |custom| {
        ActiveDialog::IncreaseHappiness(init(IncreaseHappinessConfig::new(rc.shown_player, custom)))
    })
}

pub fn increase_happiness_click(
    rc: &RenderContext,
    pos: Position,
    h: &IncreaseHappinessConfig,
) -> StateUpdate {
    if let Some(city) = rc.shown_player.try_get_city(pos) {
        StateUpdate::OpenDialog(ActiveDialog::IncreaseHappiness(add_increase_happiness(
            rc,
            city,
            h.clone(),
        )))
    } else {
        StateUpdate::None
    }
}

pub fn add_increase_happiness(
    rc: &RenderContext,
    city: &City,
    mut increase_happiness: IncreaseHappinessConfig,
) -> IncreaseHappinessConfig {
    let new_steps: Vec<(Position, u32)> = increase_happiness
        .steps
        .iter()
        .map(|(p, steps)| {
            let old_steps = *steps;
            if *p == city.position {
                if let Some(r) = increase_happiness_steps(rc, city, old_steps) {
                    return (*p, r);
                }
            }
            (*p, old_steps)
        })
        .collect();

    increase_happiness.payment =
        IncreaseHappinessConfig::happiness_payment(rc.shown_player, &new_steps, &increase_happiness.custom);
    increase_happiness.steps = new_steps;
    increase_happiness
}

fn increase_happiness_steps(rc: &RenderContext, city: &City, old_steps: u32) -> Option<u32> {
    if let Some(value) = increase_happiness_new_steps(rc, city, old_steps + 1) {
        return Some(value);
    }
    if let Some(value) = increase_happiness_new_steps(rc, city, 0) {
        return Some(value);
    }
    None
}

fn increase_happiness_new_steps(rc: &RenderContext, city: &City, new_steps: u32) -> Option<u32> {
    increase_happiness_cost(rc.shown_player, city, new_steps).map(|_| new_steps)
}

pub fn increase_happiness_menu(rc: &RenderContext, h: &IncreaseHappinessConfig) -> StateUpdate {
    payment_dialog(
        rc,
        &h.payment,
        true,
        |payment| {
            ActiveDialog::IncreaseHappiness(IncreaseHappinessConfig {
                steps: h.steps.clone(),
                payment,
                custom: h.custom.clone(),
            })
        },
        |payment| {
            StateUpdate::execute(happiness_action(
                &h.custom.action_type,
                IncreaseHappiness::new(h.steps.clone(), payment),
            ))
        },
    )
}
