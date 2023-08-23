use macroquad::hash;
use macroquad::math::{bool, Vec2};
use macroquad::ui::widgets::Group;
use server::resource_pile::ResourcePile;

use crate::dialog_ui::active_dialog_window;
use crate::resource_ui::ResourceType;
use crate::ui_state::{StateUpdate, StateUpdates};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct SelectableObject {
    pub current: u32,
    pub min: u32,
    pub max: u32,
}

pub trait HasSelectableObject {
    fn counter(&self) -> &SelectableObject;
}

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
    select_count_dialog(
        has_payment,
        |p| p.payment().resources.clone(),
        |p| format!("{} {}", &p.resource.to_string(), p.selectable.current),
        is_valid,
        execute_action,
        |c, o| show(c, o.resource),
        |c, o| plus(c, o.resource),
        |c, o| minus(c, o.resource),
    )
}

pub fn select_count_dialog<C, O: HasSelectableObject>(
    container: &C,
    get_objects: impl Fn(&C) -> Vec<O>,
    label: impl Fn(&O) -> String,
    is_valid: impl FnOnce(&C) -> bool,
    execute_action: impl FnOnce(&C) -> StateUpdate,
    show: impl Fn(&C, &O) -> bool,
    plus: impl Fn(&C, &O) -> StateUpdate,
    minus: impl Fn(&C, &O) -> StateUpdate,
) -> StateUpdate {
    let mut updates = StateUpdates::new();
    active_dialog_window(|ui| {
        for (i, p) in get_objects(container).iter().enumerate() {
            if show(container, p) {
                Group::new(hash!("res", i), Vec2::new(70., 200.)).ui(ui, |ui| {
                    ui.label(Vec2::new(0., 0.), &label(p));
                    let c = p.counter();
                    if c.current > c.min && ui.button(Vec2::new(0., 20.), "-") {
                        updates.add(minus(container, p));
                    }
                    if c.current < c.max && ui.button(Vec2::new(20., 20.), "+") {
                        updates.add(plus(container, p));
                    };
                });
            }
        }

        let valid = is_valid(container);
        let label = if valid { "OK" } else { "(OK)" };
        if ui.button(Vec2::new(0., 40.), label) && valid {
            updates.add(execute_action(container));
        };
        if ui.button(Vec2::new(80., 40.), "Cancel") {
            updates.add(StateUpdate::Cancel);
        };
    });
    updates.result()
}
