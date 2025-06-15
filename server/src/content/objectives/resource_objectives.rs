use crate::objective_card::Objective;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;

pub(crate) fn optimized_storage() -> Objective {
    Objective::builder(
        "Optimized Storage",
        "You have at least 3 food, 3 ore, and 3 wood.",
    )
    .status_phase_check(|_game, player| {
        let r = &player.resources;
        r.food >= 3 && r.ore >= 3 && r.wood >= 3
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
    .status_phase_check(move |_game, player| player.resources.has_at_least(&want))
    .status_phase_update(move |game, player| {
        player.lose_resources(game, pay.clone());
        player.log(game, &format!("Pay {pay} for {objective}",));
    })
    .build()
}
