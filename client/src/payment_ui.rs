use itertools::Itertools;
use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::OkTooltip;
use crate::event_ui::event_help;
use crate::layout_ui::{
    ICON_SIZE, bottom_centered_text_with_offset, draw_scaled_icon_with_tooltip,
};
use crate::render_context::RenderContext;
use crate::resource_ui::{new_resource_map, resource_name};
use crate::select_ui;
use crate::select_ui::{CountSelector, HasCountSelectableObject};
use crate::tooltip::show_tooltip_for_circle;
use macroquad::math::{bool, vec2};
use server::payment::PaymentOptions;
use server::resource::ResourceType;
use server::resource_pile::ResourcePile;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ResourcePayment {
    pub resource: ResourceType,
    pub selectable: CountSelector,
}

impl ResourcePayment {
    pub fn new(resource: ResourceType, current: u32, min: u32, max: u32) -> ResourcePayment {
        ResourcePayment {
            resource,
            selectable: CountSelector { current, min, max },
        }
    }
}

impl HasCountSelectableObject for ResourcePayment {
    fn counter(&self) -> &CountSelector {
        &self.selectable
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Payment<T: Clone> {
    pub value: T,
    pub name: String,
    pub cost: PaymentOptions,
    pub available: ResourcePile,
    pub optional: bool,
    pub current: Vec<ResourcePayment>,
}

impl<T> Payment<T> where T : Clone {
    #[must_use]
    pub fn new(
        cost: &PaymentOptions,
        available: &ResourcePile,
        value: T,
        name: &str,
        optional: bool,
    ) -> Payment<T> {
        Self {
            value,
            name: name.to_string(),
            cost: cost.clone(),
            available: available.clone(),
            optional,
            current: resource_payment(cost, available),
        }
    }

    pub fn to_resource_pile(&self) -> ResourcePile {
        let r = &self.current;
        ResourcePile::new(
            Self::current(r, ResourceType::Food),
            Self::current(r, ResourceType::Wood),
            Self::current(r, ResourceType::Ore),
            Self::current(r, ResourceType::Ideas),
            Self::current(r, ResourceType::Gold),
            Self::current(r, ResourceType::MoodTokens),
            Self::current(r, ResourceType::CultureTokens),
        )
    }

    pub fn get_mut(&mut self, r: ResourceType) -> &mut ResourcePayment {
        self.current
            .iter_mut()
            .find(|p| p.resource == r)
            .unwrap_or_else(|| panic!("Resource {r:?} not found in payment"))
    }
    pub fn get(&self, r: ResourceType) -> &ResourcePayment {
        self.current
            .iter()
            .find(|p| p.resource == r)
            .unwrap_or_else(|| panic!("Resource {r:?} not found in payment"))
    }

    fn current(r: &[ResourcePayment], resource_type: ResourceType) -> u32 {
        r.iter()
            .find(|p| p.resource == resource_type)
            .map_or(0, |p| p.selectable.current)
    }
}

#[must_use]
pub fn new_gain(options: &PaymentOptions, name: &str) -> Payment<String> {
    let a = options.default.amount();
    let mut available = ResourcePile::empty();
    for r in options.possible_resource_types() {
        available += ResourcePile::of(r, a);
    }
    Payment::new(options, &available, name.to_string(), name, false)
}

pub fn payment_dialog<T: Clone>(
    rc: &RenderContext,
    payment: &Payment<T>,
    may_cancel: bool,
    to_dialog: impl FnOnce(Payment<T>) -> ActiveDialog,
    execute_action: impl FnOnce(ResourcePile) -> StateUpdate,
) -> StateUpdate {
    multi_payment_dialog(
        rc,
        &[payment.clone()],
        |v| to_dialog(v[0].clone()),
        may_cancel,
        |v| execute_action(v[0].clone()),
    )
}

pub fn multi_payment_dialog<T: Clone>(
    rc: &RenderContext,
    payments: &[Payment<T>],
    to_dialog: impl FnOnce(Vec<Payment<T>>) -> ActiveDialog,
    may_cancel: bool,
    execute_action: impl FnOnce(Vec<ResourcePile>) -> StateUpdate,
) -> StateUpdate {
    let tooltip = ok_tooltip(payments, payments[0].available.clone());
    let mut exec = false;
    let mut added: Option<Payment<T>> = None;
    let mut removed: Option<Payment<T>> = None;

    for (i, payment) in payments.iter().enumerate() {
        let name = &payment.name;
        let cost = payment.cost.clone();
        let types = cost.possible_resource_types();
        let offset = vec2(0., i as f32 * -100.);
        let suffix: Vec<String> = if rc.state.active_dialog.is_modal() {
            payments
                .iter()
                .flat_map(|p| &p.cost.modifiers)
                .flat_map(|o| event_help(rc, o))
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
        let suffix = if suffix.is_empty() {
            ""
        } else {
            &format!(" (Modifiers: {})", suffix.join(", "))
        };
        bottom_centered_text_with_offset(
            rc,
            &format!("{name} for {cost}{suffix}"),
            offset + vec2(0., -30.),
        );
        let result = select_ui::count_dialog(
            rc,
            payment,
            |p| p.current.clone(),
            |s, p| {
                let _ = draw_scaled_icon_with_tooltip(
                    rc,
                    &rc.assets().resources[&s.resource],
                    &[],
                    p + vec2(0., -10.),
                    ICON_SIZE,
                );
            },
            |s, p| {
                show_tooltip_for_circle(
                    rc,
                    &[resource_name(s.resource).to_string()],
                    p + vec2(0., -10.),
                    ICON_SIZE,
                );
            },
            || tooltip.clone(),
            || {
                exec = true;
                StateUpdate::None
            },
            |_, o| types.contains(&o.resource),
            |_, o| {
                added = Some(plus(payment.clone(), o.resource));
                StateUpdate::None
            },
            |_, o| {
                removed = Some(minus(payment.clone(), o.resource));
                StateUpdate::None
            },
            offset,
            may_cancel,
        );

        if let Some(p) = added {
            return StateUpdate::OpenDialog(to_dialog(replace_updated_payment(&p, payments)));
        }
        if let Some(p) = removed {
            return StateUpdate::OpenDialog(to_dialog(replace_updated_payment(&p, payments)));
        }

        if exec {
            return execute_action(payments.iter().map(Payment::to_resource_pile).collect());
        }

        if !matches!(result, StateUpdate::None) {
            return result;
        }
    }
    StateUpdate::None
}

fn ok_tooltip<T: Clone>(payments: &[Payment<T>], mut available: ResourcePile) -> OkTooltip {
    let mut valid: Vec<String> = vec![];
    let mut invalid: Vec<String> = vec![];

    for payment in payments {
        let cost = &payment.cost;
        let pile = payment.to_resource_pile();
        let name = &payment.name;
        let tooltip = if payment.optional && pile.is_empty() {
            OkTooltip::Valid(format!("Pay nothing for {name}"))
        } else if available.has_at_least(&pile) && cost.is_valid_payment(&pile) {
            // make sure that we can afford all the payments
            available -= payment.to_resource_pile();
            OkTooltip::Valid(format!("Pay {pile} for {name}"))
        } else {
            OkTooltip::Invalid(format!(
                "You don't have {} for {}",
                payment.cost.default, name
            ))
        };
        match tooltip {
            OkTooltip::Valid(v) => valid.push(v),
            OkTooltip::Invalid(i) => invalid.push(i),
        }
    }

    if invalid.is_empty() {
        OkTooltip::Valid(valid.join(", "))
    } else {
        OkTooltip::Invalid(invalid.join(", "))
    }
}

fn replace_updated_payment<T: Clone>(payment: &Payment<T>, all: &[Payment<T>]) -> Vec<Payment<T>> {
    all.iter()
        .map(|e| {
            if e.name == payment.name {
                payment.clone()
            } else {
                e.clone()
            }
        })
        .collect_vec()
}

#[must_use]
fn resource_payment(options: &PaymentOptions, available: &ResourcePile) -> Vec<ResourcePayment> {
    let d = options.first_valid_payment(available).unwrap();
    let mut resources: Vec<ResourcePayment> = new_resource_map(&d)
        .into_iter()
        .map(|e| {
            let resource_type = e.0;
            ResourcePayment {
                resource: resource_type,
                selectable: CountSelector {
                    current: e.1,
                    min: 0,
                    max: available.get(&resource_type),
                },
            }
        })
        .collect();

    resources.sort_by_key(|r| r.resource);
    resources
}

pub fn plus<T: Clone>(mut payment: Payment<T>, t: ResourceType) -> Payment<T> {
    payment.get_mut(t).selectable.current += 1;
    payment
}

pub fn minus<T: Clone>(mut payment: Payment<T>, t: ResourceType) -> Payment<T> {
    payment.get_mut(t).selectable.current -= 1;
    payment
}
