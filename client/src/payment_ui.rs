use macroquad::math::bool;
use server::resource_pile::ResourcePile;

use crate::resource_ui::ResourceType;
use crate::select_ui;
use crate::select_ui::{HasSelectableObject, SelectableObject};
use crate::ui_state::StateUpdate;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ResourcePayment {
    pub resource: ResourceType,
    pub selectable: SelectableObject,
}

impl ResourcePayment {
    pub fn new(resource: ResourceType, current: u32, min: u32, max: u32) -> ResourcePayment {
        ResourcePayment {
            resource,
            selectable: SelectableObject { current, min, max },
        }
    }
}

impl HasSelectableObject for ResourcePayment {
    fn counter(&self) -> &SelectableObject {
        &self.selectable
    }
    fn counter_mut(&mut self) -> &mut SelectableObject {
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
    fn payment(&self) -> &Payment;
}

pub fn payment_dialog<T: HasPayment>(
    has_payment: &T,
    is_valid: impl FnOnce(&T) -> bool,
    execute_action: impl FnOnce(&T) -> StateUpdate,
    show: impl Fn(&T, ResourceType) -> bool,
    plus: impl Fn(&T, ResourceType) -> StateUpdate,
    minus: impl Fn(&T, ResourceType) -> StateUpdate,
) -> StateUpdate {
    select_ui::dialog(
        has_payment,
        |p| p.payment().resources.clone(),
        |p| p.resource.to_string(),
        is_valid,
        execute_action,
        |c, o| show(c, o.resource),
        |c, o| plus(c, o.resource),
        |c, o| minus(c, o.resource),
    )
}
