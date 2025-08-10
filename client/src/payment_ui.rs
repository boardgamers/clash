use crate::client_state::{ActiveDialog, NO_UPDATE, RenderResult, StateUpdate};
use crate::dialog_ui::OkTooltip;
use crate::event_ui::event_help;
use crate::layout_ui::{
    ICON_SIZE, bottom_centered_text_with_offset, draw_scaled_icon_with_tooltip,
};
use crate::log_ui::MultilineText;
use crate::render_context::RenderContext;
use crate::resource_ui::{new_resource_map, resource_name};
use crate::select_ui;
use crate::select_ui::{CountSelector, HasCountSelectableObject, SELECT_RADIUS};
use crate::tooltip::show_tooltip_for_circle;
use itertools::Itertools;
use macroquad::math::{bool, vec2};
use server::payment::{PaymentOptions, ResourceReward};
use server::resource::ResourceType;
use server::resource_pile::ResourcePile;
use std::slice;

#[derive(PartialEq, Eq, Debug, Clone)]
pub(crate) struct ResourcePayment {
    pub resource: ResourceType,
    pub selectable: CountSelector,
}

impl ResourcePayment {
    pub(crate) fn new(resource: ResourceType, current: u8, min: u8, max: u8) -> ResourcePayment {
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
pub(crate) struct Payment<T: Clone> {
    pub value: T,
    pub name: String,
    pub cost: PaymentOptions,
    pub available: ResourcePile,
    pub optional: bool,
    pub current: Vec<ResourcePayment>,
}

impl<T> Payment<T>
where
    T: Clone,
{
    #[must_use]
    pub(crate) fn new(
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

    pub(crate) fn to_resource_pile(&self) -> ResourcePile {
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

    pub(crate) fn get_mut(&mut self, r: ResourceType) -> &mut ResourcePayment {
        self.current
            .iter_mut()
            .find(|p| p.resource == r)
            .unwrap_or_else(|| panic!("Resource {r} not found in payment"))
    }

    fn current(r: &[ResourcePayment], resource_type: ResourceType) -> u8 {
        r.iter()
            .find(|p| p.resource == resource_type)
            .map_or(0, |p| p.selectable.current)
    }
}

#[must_use]
pub(crate) fn new_gain(reward: &ResourceReward, name: &str) -> Payment<String> {
    let options = &reward.payment_options;
    let a = options.default.amount();
    let mut available = ResourcePile::empty();
    for r in options.possible_resource_types() {
        available += ResourcePile::of(r, a);
    }
    Payment::new(options, &available, name.to_string(), name, false)
}

pub(crate) fn payment_dialog<T: Clone>(
    rc: &RenderContext,
    payment: &Payment<T>,
    may_cancel: bool,
    to_dialog: impl FnOnce(Payment<T>) -> ActiveDialog,
    execute_action: impl FnOnce(ResourcePile) -> RenderResult,
) -> RenderResult {
    multi_payment_dialog(
        rc,
        slice::from_ref(payment),
        |v| to_dialog(v[0].clone()),
        may_cancel,
        |v| execute_action(v[0].clone()),
    )
}

pub(crate) fn multi_payment_dialog<T: Clone>(
    rc: &RenderContext,
    payments: &[Payment<T>],
    to_dialog: impl FnOnce(Vec<Payment<T>>) -> ActiveDialog,
    may_cancel: bool,
    execute_action: impl FnOnce(Vec<ResourcePile>) -> RenderResult,
) -> RenderResult {
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
            offset + vec2(0., -70.),
            &MultilineText::default(),
        );
        select_ui::count_dialog(
            rc,
            payment,
            |p| p.current.clone(),
            |rc, s, p| {
                let _ = draw_scaled_icon_with_tooltip(
                    rc,
                    &rc.assets().resources[&s.resource],
                    &MultilineText::default(),
                    p + vec2(-15., -15.),
                    ICON_SIZE,
                );
            },
            |s, p| {
                show_tooltip_for_circle(
                    rc,
                    &MultilineText::of(rc, resource_name(s.resource)),
                    p,
                    SELECT_RADIUS,
                );
            },
            || tooltip.clone(),
            || {
                exec = true;
                NO_UPDATE
            },
            |_, o| types.contains(&o.resource),
            |_, o| {
                added = Some(plus(payment.clone(), o.resource));
                NO_UPDATE
            },
            |_, o| {
                removed = Some(minus(payment.clone(), o.resource));
                NO_UPDATE
            },
            offset,
            may_cancel,
        )?;

        if let Some(p) = added {
            return StateUpdate::open_dialog(to_dialog(replace_updated_payment(&p, payments)));
        }
        if let Some(p) = removed {
            return StateUpdate::open_dialog(to_dialog(replace_updated_payment(&p, payments)));
        }

        if exec {
            return execute_action(payments.iter().map(Payment::to_resource_pile).collect());
        }
    }
    NO_UPDATE
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
            ResourcePayment::new(resource_type, e.1, 0, available.get(&resource_type))
        })
        .collect();

    resources.sort_by_key(|r| r.resource);
    resources
}

pub(crate) fn plus<T: Clone>(mut payment: Payment<T>, t: ResourceType) -> Payment<T> {
    payment.get_mut(t).selectable.current += 1;
    payment
}

pub(crate) fn minus<T: Clone>(mut payment: Payment<T>, t: ResourceType) -> Payment<T> {
    payment.get_mut(t).selectable.current -= 1;
    payment
}
