use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::OkTooltip;
use crate::layout_ui::{bottom_centered_text_with_offset, draw_icon};
use crate::render_context::RenderContext;
use crate::resource_ui::{new_resource_map, resource_name};
use crate::select_ui;
use crate::select_ui::{CountSelector, HasCountSelectableObject};
use macroquad::math::{bool, vec2};
use server::payment::{PaymentModel, SumPaymentOptions};
use server::resource::ResourceType;
use server::resource_pile::{PaymentOptions, ResourcePile};
use std::cmp;
use std::cmp::min;

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
    pub name: String,
    pub model: PaymentModel,
    pub optional: bool,
    pub current: Vec<ResourcePayment>,
    pub discount_used: u32,
}

impl Payment {
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
            .unwrap()
            .selectable
            .current
    }
    //
    // pub fn is_valid(&self) -> bool {
    //     self.model.is_valid(&self.to_resource_pile())
    // }
}
//
// pub trait HasPayment {
//     fn payment(&self) -> Payment;
// }

// #[allow(clippy::too_many_arguments)]
// pub fn payment_dialog(
//     payment: &Payment,
//     execute_action: impl FnOnce() -> StateUpdate,
//     show: impl Fn(&Payment, ResourceType) -> bool,
//     plus: impl FnOnce(&Payment, ResourceType) -> StateUpdate,
//     minus: impl FnOnce(&Payment, ResourceType) -> StateUpdate,
//     rc: &RenderContext,
//     offset: Vec2,
//     may_cancel: bool,
// ) -> StateUpdate {
//     select_ui::count_dialog(
//         rc,
//         payment,
//         |p| p.payment().resources.clone(),
//         |s, p| {
//             let _ = draw_icon(
//                 rc,
//                 &rc.assets().resources[&s.resource],
//                 resource_name(s.resource),
//                 p + vec2(0., -10.),
//             );
//         },
//         Payment::is_valid,
//         execute_action,
//         |c, o| show(c, o.resource),
//         |c, o| plus(c, o.resource),
//         |c, o| minus(c, o.resource),
//         offset,
//         may_cancel,
//     )
// }
//
// #[derive(Clone)]
// pub struct PaymentModelEntry {
//     pub name: String,
//     pub model: PaymentModel,
//     pub payment: Payment,
//     pub optional: bool,
// }

// impl HasPayment for PaymentModelEntry {
//     fn payment(&self) -> Payment {
//         let PaymentModel::Sum(a) = &self.model;
//         let left = &a.left;
//
//         let mut resources: Vec<ResourcePayment> = new_resource_map(&a.default)
//             .into_iter()
//             .map(|e| ResourcePayment::new(e.0, e.1, 0, min(a.cost, e.1 + left.get(e.0))))
//             .collect();
//         resources.sort_by_key(|r| r.resource);
//
//         Payment { resources }
//     }
// }

pub fn payment_model_dialog(
    rc: &RenderContext,
    payments: &[Payment], // None means the player can pay nothing
    to_dialog: impl FnOnce(Vec<Payment>) -> ActiveDialog,
    may_cancel: bool,
    execute_action: impl FnOnce(Vec<ResourcePile>) -> StateUpdate,
) -> StateUpdate {
    let tooltip = ok_tooltip(payments, rc.shown_player.resources.clone());
    let mut exec = false;
    let mut added: Option<Payment> = None;
    let mut removed: Option<Payment> = None;

    for (i, payment) in payments.iter().enumerate() {
        let model = payment.model.clone();
        let types = show_types(&model);
        let offset = vec2(0., i as f32 * -100.);
        bottom_centered_text_with_offset(rc, &payment.name, offset + vec2(0., -30.));
        let result = select_ui::count_dialog(
            rc,
            payment,
            |p| p.current.clone(),
            |s, p| {
                let _ = draw_icon(
                    rc,
                    &rc.assets().resources[&s.resource],
                    resource_name(s.resource),
                    p + vec2(0., -10.),
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

        //
        // let result = payment_dialog(
        //     payment,
        //     |payment| {
        //         valid(tooltip, i, payment)
        //     },
        //     || {
        //         exec = Some(
        //             payment
        //                 .iter()
        //                 .map(|p| p.payment().to_resource_pile())
        //                 .collect(),
        //         );
        //         StateUpdate::None
        //     },
        //     |_ap, r| types.contains(&r),
        //     |ap, r| {
        //         plus()
        //         added = Some(add(ap, r, 1, payment));
        //         StateUpdate::None
        //     },
        //     |ap, r| {
        //         removed = Some(add(ap, r, -1, payment));
        //         StateUpdate::None
        //     },
        //     rc,
        //     offset,
        //     may_cancel,
        // );

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

fn ok_tooltip(payments: &[Payment], mut available: ResourcePile) -> OkTooltip {
    let mut valid: Vec<String> = vec![];
    let mut invalid: Vec<String> = vec![];

    for payment in payments {
        let model = &payment.model;
        let pile = payment.to_resource_pile();
        let name = &payment.name;
        let tooltip = if payment.optional && pile.is_empty() {
            OkTooltip::Valid(format!("Pay nothing for {name}"))
        } else if model.can_afford(&available) && model.is_valid_payment(&pile) {
            // make sure that we can afford all of the payments
            OkTooltip::Valid(format!("Pay {pile} for {name}"))
        } else {
            OkTooltip::Invalid(format!("You don't have {:?} for {}", payment.model, name))
        };
        match tooltip {
            OkTooltip::Valid(v) => valid.push(v),
            OkTooltip::Invalid(i) => invalid.push(i),
        }
        available -= payment.to_resource_pile();
    }

    if invalid.is_empty() {
        OkTooltip::Valid(valid.join(", "))
    } else {
        OkTooltip::Invalid(invalid.join(", "))
    }
}

fn replace_updated_payment(payment: &Payment, all: &[Payment]) -> Vec<Payment> {
    all.iter()
        .map(|e| {
            if e.name == payment.name {
                payment.clone()
            } else {
                e.clone()
            }
        })
        .collect::<Vec<_>>()
}

fn sum_payment(a: &SumPaymentOptions, available: &ResourcePile) -> Vec<ResourcePayment> {
    let mut cost_left = a.cost;

    a.types_by_preference
        .iter()
        .map(|t| {
            let have = available.get(*t);
            let used = min(have, cost_left);
            cost_left -= used;
            ResourcePayment::new(*t, used, 0, have)
        })
        .collect()
}

#[must_use]
pub fn new_payment(
    model: &PaymentModel,
    available: &ResourcePile,
    name: &str,
    optional: bool,
) -> Payment {
    let mut discount_used = 0;
    let resources = match model {
        PaymentModel::Sum(options) => sum_payment(options, available),
        PaymentModel::Resources(a) => {
            let options = a.get_payment_options(available);
            discount_used = a.discount - options.discount_left;
            resource_payment(&options)
        }
    };

    Payment {
        name: name.to_string(),
        model: model.clone(),
        optional,
        current: resources,
        discount_used,
    }
}

#[must_use]
fn resource_payment(options: &PaymentOptions) -> Vec<ResourcePayment> {
    let mut resources: Vec<ResourcePayment> = new_resource_map(&options.default)
        .into_iter()
        .map(|e| {
            let resource_type = e.0;
            let amount = e.1;
            match resource_type {
                ResourceType::Gold => ResourcePayment {
                    resource: resource_type,
                    selectable: CountSelector {
                        current: amount,
                        min: amount,
                        max: amount,
                    },
                },
                _ => ResourcePayment {
                    resource: resource_type,
                    selectable: CountSelector {
                        current: amount,
                        min: cmp::max(
                            0,
                            amount as i32 - options.discount_left as i32 - options.gold_left as i32,
                        ) as u32,
                        max: amount,
                    },
                },
            }
        })
        .collect();

    resources.sort_by_key(|r| r.resource);
    resources
}

#[must_use]
pub fn show_types(model: &PaymentModel) -> Vec<ResourceType> {
    match model {
        PaymentModel::Sum(options) => options.types_by_preference.clone(),
        PaymentModel::Resources(options) => options.cost.types(),
    }
}

pub fn plus(mut payment: Payment, t: ResourceType) -> Payment {
    match payment.model {
        PaymentModel::Sum(_) => {
            payment.get_mut(t).selectable.current += 1;
        }
        PaymentModel::Resources(_) => {
            {
                let gold = payment.get_mut(ResourceType::Gold);
                if gold.selectable.current > 0 {
                    gold.selectable.current -= 1;
                } else {
                    payment.discount_used += 1;
                }
            }
            payment.get_mut(t).selectable.current += 1;
        }
    }
    payment
}

pub fn minus(mut payment: Payment, t: ResourceType) -> Payment {
    match payment.model {
        PaymentModel::Sum(_) => {
            payment.get_mut(t).selectable.current -= 1;
        }
        PaymentModel::Resources(_) => {
            {
                if payment.discount_used > 0 {
                    payment.discount_used -= 1;
                } else {
                    payment.get_mut(ResourceType::Gold).selectable.current += 1;
                }
            }
            payment.get_mut(t).selectable.current -= 1;
        }
    }
    payment
}
