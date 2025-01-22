use macroquad::math::{bool, vec2};
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
    fn counter_mut(&mut self) -> &mut CountSelector {
        &mut self.selectable
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

pub trait PaymentModelPayment {
    fn payment_model(&self) -> &PaymentModel;

    fn name(&self) -> &str;

    fn new_dialog(&self) -> ActiveDialog;
}

impl<T> HasPayment for T
where
    T: PaymentModelPayment,
{
    fn payment(&self) -> Payment {
        let PaymentModel::Sum(a) = self.payment_model().clone();
        let left = a.left;

        let mut resources: Vec<ResourcePayment> = new_resource_map(&a.default)
            .into_iter()
            .map(|e| ResourcePayment::new(e.0, e.1, 0, min(a.cost, e.1 + left.get(e.0))))
            .collect();
        resources.sort_by_key(|r| r.resource);

        Payment { resources }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn payment_dialog<T: HasPayment>(
    has_payment: &T,
    is_valid: impl FnOnce(&T) -> OkTooltip,
    execute_action: impl FnOnce() -> StateUpdate,
    show: impl Fn(&T, ResourceType) -> bool,
    plus: impl Fn(&T, ResourceType) -> StateUpdate,
    minus: impl Fn(&T, ResourceType) -> StateUpdate,
    rc: &RenderContext,
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
    )
}

pub fn payment_model_dialog<T: PaymentModelPayment>(
    payment: &T,
    rc: &RenderContext,
    execute_action: impl FnOnce(ResourcePile) -> StateUpdate,
) -> StateUpdate {
    bottom_center_text(rc, payment.name(), vec2(-200., -50.));

    payment_dialog(
        payment,
        |payment| payment_model_valid(payment),
        || execute_action(payment.payment().to_resource_pile()),
        |ap, r| ap.payment().get(r).selectable.max > 0,
        |ap, r| add(ap, r, 1),
        |ap, r| add(ap, r, -1),
        rc,
    )
}

fn payment_model_valid<T: PaymentModelPayment>(payment: &T) -> OkTooltip {
    let pile = payment.payment().to_resource_pile();
    let model = payment.payment_model();
    let name = payment.name();

    if model.is_valid(&pile) {
        OkTooltip::Valid(format!("Pay {pile} for {name}"))
    } else {
        OkTooltip::Invalid(format!("You don't have {} for {}", model.default(), name))
    }
}

fn add<T: PaymentModelPayment>(ap: &T, r: ResourceType, i: i32) -> StateUpdate {
    let new = ap;
    let mut binding = new.payment();
    let p = binding.get_mut(r);

    let c = p.counter_mut();
    c.current = (c.current as i32 + i) as u32;
    StateUpdate::OpenDialog(new.new_dialog())
}
