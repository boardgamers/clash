use server::resource_pile::{AdvancePaymentOptions, ResourcePile};
use std::collections::HashMap;
use std::fmt;

#[derive(PartialEq, Eq, Debug, Clone, Hash, Ord, PartialOrd)]
pub enum ResourceType {
    Food,
    Wood,
    Ore,
    Ideas,
    Gold,
    MoodTokens,
    CultureTokens,
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct ResearchPayment {
    pub player_index: usize,
    pub name: String,
    pub payment: Payment,
}

impl ResearchPayment {
    pub fn new(player_index: usize, name: String, payment: Payment) -> ResearchPayment {
        ResearchPayment {
            player_index,
            name,
            payment,
        }
    }

    pub fn valid(&self) -> bool {
        self.payment
            .resources
            .iter()
            .map(|r| r.current)
            .sum::<u32>()
            == 2
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
    pub fn new_advance_resource_payment(a: AdvancePaymentOptions) -> Payment {
        let left = HashMap::from([
            (ResourceType::Food, a.food_left),
            (ResourceType::Gold, a.gold_left),
        ]);

        let mut resources: Vec<ResourcePayment> = new_resource_map(a.default)
            .into_iter()
            .map(|e| ResourcePayment {
                resource: e.0.clone(),
                current: e.1,
                min: 0,
                max: e.1 + left.get(&e.0).unwrap_or(&(0u32)),
            })
            .collect();
        resources.sort_by_key(|r| r.resource.clone());

        Payment { resources }
    }

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

    fn current(r: &[ResourcePayment], resource_type: ResourceType) -> u32 {
        r.iter()
            .find(|p| p.resource == resource_type)
            .unwrap()
            .current
    }
}

pub fn new_resource_map(p: ResourcePile) -> HashMap<ResourceType, u32> {
    let mut m: HashMap<ResourceType, u32> = HashMap::new();
    add_resource(&mut m, p.food, ResourceType::Food);
    add_resource(&mut m, p.wood, ResourceType::Wood);
    add_resource(&mut m, p.ore, ResourceType::Ore);
    add_resource(&mut m, p.ideas, ResourceType::Ideas);
    add_resource(&mut m, p.gold as u32, ResourceType::Gold);
    add_resource(&mut m, p.mood_tokens, ResourceType::MoodTokens);
    add_resource(&mut m, p.culture_tokens, ResourceType::CultureTokens);
    m
}

fn add_resource(m: &mut HashMap<ResourceType, u32>, amount: u32, resource_type: ResourceType) {
    m.insert(resource_type, amount);
}
