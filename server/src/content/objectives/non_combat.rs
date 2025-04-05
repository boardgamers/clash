use crate::ability_initializer::AbilityInitializerSetup;
use crate::content::advances::warfare::draft_cost;
use crate::objective_card::{Objective, objective_is_ready};

pub(crate) fn non_combat_objectives() -> Vec<Objective> {
    vec![draft()]
}

pub(crate) fn draft() -> Objective {
    let name = "Draft";
    Objective::builder(name, "You've recruited twice using Draft this turn.")
        .add_simple_persistent_event_listener(
            |event| &mut event.recruit,
            2,
            |game, player, _, r| {
                let p = game.player_mut(player);
                // Draft is just a cost conversion
                let used_draft = r.units.infantry > 0
                    && r.payment.mood_tokens >= draft_cost(p)
                    && p.has_advance("Draft");
                if used_draft {
                    if p.event_info.contains_key("Used Draft") {
                        objective_is_ready(p, name);
                    } else {
                        p.event_info
                            .insert("Used Draft".to_string(), "true".to_string());
                    }
                }
            },
        )
        .build()
}
