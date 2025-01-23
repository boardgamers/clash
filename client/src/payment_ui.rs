use macroquad::math::{bool, vec2, Vec2};
use server::payment::PaymentModel;
use server::resource_pile::ResourcePile;
use std::cmp::min;

use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::OkTooltip;
use crate::layout_ui::{bottom_center_text, draw_icon};
use crate::render_context::RenderContext;
use crate::resource_ui::{new_resource_map, resource_name};
use crate::select_ui;
use crate::select_ui::{CountSelector, HasCountSelectableObject};
use server::resource::ResourceType;

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
pub struct Payment {
    pub resources: Vec<ResourcePayment>,
}

impl Payment {
    pub fn to_resource_pile(&self) -> ResourcePile {
        let r = &self.resources;
        ResourcePile::new(
            Self::current(r, ResourceType::Food),
            Self::current(r, ResourceType::Wood),
            Self::current(r, ResourceType::Ore),
            Self::current(r, ResourceType::Ideas),
            Self::current(r, ResourceType::Gold) as i32,
            Self::current(r, ResourceType::MoodTokens),
            Self::current(r, ResourceType::CultureTokens),
        )
    }

    pub fn get_mut(&mut self, r: ResourceType) -> &mut ResourcePayment {
        self.resources
            .iter_mut()
            .find(|p| p.resource == r)
            .unwrap_or_else(|| panic!("Resource {r:?} not found in payment"))
    }
    pub fn get(&self, r: ResourceType) -> &ResourcePayment {
        self.resources
            .iter()
            .find(|p| p.resource == r)
            .unwrap_or_else(|| panic!("Resource {r:?} not found in payment"))
    }

    fn current(r: &[ResourcePayment], resource_type: ResourceType) -> u32 {
        r.iter()
            .find(|p| p.resource == resource_type)
            .unwrap()
            .selectable
            .current
    }
}

pub trait HasPayment {
    fn payment(&self) -> Payment;
}

#[allow(clippy::too_many_arguments)]
pub fn payment_dialog<T: HasPayment>(
    has_payment: &T,
    is_valid: impl FnOnce(&T) -> OkTooltip,
    execute_action: impl FnOnce() -> StateUpdate,
    show: impl Fn(&T, ResourceType) -> bool,
    plus: impl FnOnce(&T, ResourceType) -> StateUpdate,
    minus: impl FnOnce(&T, ResourceType) -> StateUpdate,
    rc: &RenderContext,
    offset: Vec2,
) -> StateUpdate {
    select_ui::count_dialog(
        rc,
        has_payment,
        |p| p.payment().resources.clone(),
        |s, p| {
            let _ = draw_icon(
                rc,
                &rc.assets().resources[&s.resource],
                resource_name(s.resource),
                p + vec2(0., -10.),
            );
        },
        is_valid,
        execute_action,
        |c, o| show(c, o.resource),
        |c, o| plus(c, o.resource),
        |c, o| minus(c, o.resource),
        offset,
    )
}

#[derive(Clone)]
pub struct PaymentModelEntry {
    pub name: String,
    pub model: PaymentModel,
    pub optional: bool,
}

impl HasPayment for PaymentModelEntry {
    fn payment(&self) -> Payment {
        let PaymentModel::Sum(a) = &self.model;
        let left = &a.left;

        let mut resources: Vec<ResourcePayment> = new_resource_map(&a.default)
            .into_iter()
            .map(|e| ResourcePayment::new(e.0, e.1, 0, min(a.cost, e.1 + left.get(e.0))))
            .collect();
        resources.sort_by_key(|r| r.resource);

        Payment { resources }
    }
}

pub fn payment_model_dialog(
    rc: &RenderContext,
    payment: &[PaymentModelEntry], // None means the player can pay nothing
    to_dialog: impl FnOnce(Vec<PaymentModelEntry>) -> ActiveDialog,
    execute_action: impl FnOnce(Vec<ResourcePile>) -> StateUpdate,
) -> StateUpdate {
    let mut valid = payment.iter().map(payment_model_valid).collect::<Vec<_>>();
    let mut exec: Option<Vec<ResourcePile>> = None;
    let mut added: Option<Vec<PaymentModelEntry>> = None;
    let mut removed: Option<Vec<PaymentModelEntry>> = None;

    for (i, p) in payment.iter().enumerate() {
        bottom_center_text(rc, &p.name, vec2(0., i as f32 * -100.));
        let model = p.model.clone();
        let types = model.show_types();
        let offset = vec2(0., i as f32 * -100.);
        let result = payment_dialog(
            p,
            |payment| {
                valid[i] = payment_model_valid(payment);
                valid
                    .iter()
                    .find(|v| !v.is_valid())
                    .unwrap_or(&valid[0])
                    .clone()
            },
            || {
                exec = Some(
                    payment
                        .iter()
                        .map(|p| p.payment().to_resource_pile())
                        .collect(),
                );
                StateUpdate::None
            },
            |_ap, r| types.contains(&r),
            |ap, r| {
                added = Some(add(ap, r, 1, payment));
                StateUpdate::None
            },
            |ap, r| {
                removed = Some(add(ap, r, -1, payment));
                StateUpdate::None
            },
            rc,
            offset,
        );

        if let Some(v) = added {
            return StateUpdate::OpenDialog(to_dialog(v));
        }
        if let Some(v) = removed {
            return StateUpdate::OpenDialog(to_dialog(v));
        }

        if let Some(v) = exec {
            return execute_action(v);
        }

        if !matches!(result, StateUpdate::None) {
            return result;
        }
    }
    StateUpdate::None
}

fn payment_model_valid(payment: &PaymentModelEntry) -> OkTooltip {
    let model = &payment.model;
    let pile = model.default();
    let name = &payment.name;

    if payment.optional && pile.is_empty() {
        return OkTooltip::Valid(format!("Pay nothing for {name}"));
    }

    if model.is_valid(pile) {
        OkTooltip::Valid(format!("Pay {pile} for {name}"))
    } else {
        OkTooltip::Invalid(format!("You don't have {} for {}", model.default(), name))
    }
}

fn add(
    ap: &PaymentModelEntry,
    r: ResourceType,
    diff: i32,
    all: &[PaymentModelEntry],
) -> Vec<PaymentModelEntry> {
    let mut new = ap.clone();
    new.model.add_type(r, diff);

    all.iter()
        .map(|e| {
            if e.name == ap.name {
                new.clone()
            } else {
                e.clone()
            }
        })
        .collect::<Vec<_>>()
}
