use crate::ability_initializer::AbilityInitializerSetup;
use crate::content::advances::warfare::draft_cost;
use crate::content::objectives::status_phase_objectives::home_position;
use crate::objective_card::{Objective, objective_is_ready};

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

pub(crate) fn city_founder() -> Objective {
    let name = "City Founder";
    Objective::builder(name, "You founded a city this at least 5 spaces away from your starting city position.")
        .add_simple_persistent_event_listener(
            |event| &mut event.found_city,
            0,
            |game, player, _, p| {
                if home_position(game, game.player(player)).distance(*p) >= 5 {
                    objective_is_ready(game.player_mut(player), name);
                }
            }

        )
        .build()
}
