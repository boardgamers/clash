use std::collections::HashMap;
use std::fmt;

use macroquad::hash;
use macroquad::math::{bool, vec2, Vec2};
use macroquad::ui::root_ui;
use macroquad::ui::widgets::Group;
use server::resource_pile::ResourcePile;

#[derive(PartialEq, Eq, Debug, Clone, Hash, Ord, PartialOrd)]
pub enum ResourceType {
    Food,
    Wood,
    Ore,
    Ideas,
    Gold,
    MoodTokens,
    CultureTokens,
    Discount, //discount on building cost, which can be used for any resource that is not a token
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ResourcePayment {
    pub resource: ResourceType,
    pub current: u32,
    pub min: u32,
    pub max: u32,
}

#[derive(PartialEq, Eq, Debug)]
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
            .unwrap_or_else(|| panic!("Resource {:?} not found in payment", r))
    }
    pub fn get(&self, r: ResourceType) -> &ResourcePayment {
        self.resources
            .iter()
            .find(|p| p.resource == r)
            .unwrap_or_else(|| panic!("Resource {:?} not found in payment", r))
    }

    fn current(r: &[ResourcePayment], resource_type: ResourceType) -> u32 {
        r.iter()
            .find(|p| p.resource == resource_type)
            .unwrap()
            .current
    }
}

pub fn new_resource_map(p: &ResourcePile) -> HashMap<ResourceType, u32> {
    let mut m: HashMap<ResourceType, u32> = HashMap::new();
    add_resource(&mut m, p.food, ResourceType::Food);
    add_resource(&mut m, p.wood, ResourceType::Wood);
    add_resource(&mut m, p.ore, ResourceType::Ore);
    add_resource(&mut m, p.ideas, ResourceType::Ideas);
    add_resource(&mut m, p.gold as u32, ResourceType::Gold);
    add_resource(&mut m, p.mood_tokens, ResourceType::MoodTokens);
    add_resource(&mut m, p.culture_tokens, ResourceType::CultureTokens);
    add_resource(&mut m, 0, ResourceType::Discount);
    m
}

fn add_resource(m: &mut HashMap<ResourceType, u32>, amount: u32, resource_type: ResourceType) {
    m.insert(resource_type, amount);
}

pub trait HasPayment {
    fn payment(&self) -> &Payment;
}

pub fn payment_dialog<T: HasPayment>(
    has_payment: &mut T,
    is_valid: impl FnOnce(&T) -> bool,
    execute_action: impl FnOnce(&T),
    show: impl Fn(&T, ResourceType) -> bool,
    plus: impl Fn(&mut T, ResourceType),
    minus: impl Fn(&mut T, ResourceType),
) -> bool {
    let mut result = false;
    root_ui().window(hash!(), vec2(20., 510.), vec2(400., 200.), |ui| {
        for (i, p) in has_payment.payment().resources.clone().iter().enumerate() {
            if show(has_payment, p.resource.clone()) {
                Group::new(hash!("res", i), Vec2::new(70., 200.)).ui(ui, |ui| {
                    let s = format!("{} {}", &p.resource.to_string(), p.current);
                    ui.label(Vec2::new(0., 0.), &s);
                    if p.current > p.min && ui.button(Vec2::new(0., 20.), "-") {
                        minus(has_payment, p.resource.clone());
                    }
                    if p.current < p.max && ui.button(Vec2::new(20., 20.), "+") {
                        plus(has_payment, p.resource.clone());
                    };
                });
            }
        }

        let valid = is_valid(has_payment);
        let label = if valid { "OK" } else { "(OK)" };
        if ui.button(Vec2::new(0., 40.), label) && valid {
            execute_action(has_payment);
            result = true;
        };
    });
    result
}
