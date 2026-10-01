use crate::objective_card::{Objective, ObjectiveProgress};
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;

pub(crate) fn optimized_storage() -> Objective {
    Objective::builder(
        "Optimized Storage",
        "You have at least 3 food, 3 ore, and 3 wood.",
    )
    .status_phase_progress(|_game, player| {
        let r = &player.resources;
        vec![
            ObjectiveProgress::new("Food", r.food as usize, 3),
            ObjectiveProgress::new("Ore", r.ore as usize, 3),
            ObjectiveProgress::new("Wood", r.wood as usize, 3),
        ]
    })
    .build()
}

pub(crate) fn supplies(objective: &'static str, r: ResourceType) -> Objective {
    pay_resources(objective, ResourcePile::of(r, 5), ResourcePile::of(r, 2))
}

pub(crate) fn eureka() -> Objective {
    supplies("Eureka!", ResourceType::Ideas)
}

pub(crate) fn wealth() -> Objective {
    supplies("Wealth", ResourceType::Gold)
}

pub(crate) fn ore_supplies() -> Objective {
    supplies("Ore Supplies", ResourceType::Ore)
}

pub(crate) fn wood_supplies() -> Objective {
    supplies("Wood Supplies", ResourceType::Wood)
}

pub(crate) fn food_supplies() -> Objective {
    supplies("Food Supplies", ResourceType::Food)
}

pub(crate) fn pay_resources(
    objective: &'static str,
    want: ResourcePile,
    pay: ResourcePile,
) -> Objective {
    let suffix = if pay.gold == 0 { " (not gold)" } else { "" };
    Objective::builder(
        objective,
        &format!("You have at least {want}: Pay {pay}{suffix}."),
    )
    .status_phase_progress(move |_game, player| {
        want.clone()
            .into_iter()
            .filter(|(_, amount)| *amount > 0)
            .map(|(resource, amount)| {
                ObjectiveProgress::new(
                    resource.to_string(),
                    player.resources.get(&resource) as usize,
                    amount as usize,
                )
            })
            .collect()
    })
    .status_phase_update(move |game, player| {
        player.lose_resources(game, pay.clone());
        player.log(game, &format!("Pay {pay} for {objective}",));
    })
    .build()
}
