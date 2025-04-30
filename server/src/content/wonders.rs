use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::content::persistent_events::PaymentRequest;
use crate::map::Terrain::Fertile;
use crate::payment::{PaymentOptions, PaymentReason};
use crate::wonder::Wonder;
use crate::{resource_pile::ResourcePile, wonder::WonderInfo};
use std::collections::HashSet;

#[must_use]
pub fn get_all_uncached() -> Vec<WonderInfo> {
    vec![colosseum(), pyramids(), great_gardens()]
}

fn great_gardens() -> WonderInfo {
    WonderInfo::builder(
        Wonder::GreatGardens,
        "Great Gardens",
        "The city with this wonder may Collect any type of resource from \
            Grassland spaces including ideas and gold. \
            Enemies cannot enter the city if they have entered a Grassland space this turn.",
        PaymentOptions::fixed_resources(ResourcePile::new(5, 5, 2, 0, 0, 0, 5)),
        Advance::Irrigation,
    )
    .add_transient_event_listener(
        |events| &mut events.terrain_collect_options,
        1,
        |m, (), ()| {
            m.insert(
                Fertile,
                HashSet::from([
                    ResourcePile::food(1),
                    ResourcePile::wood(1),
                    ResourcePile::ore(1),
                    ResourcePile::ideas(1),
                    ResourcePile::gold(1),
                ]),
            );
        },
    )
    .build()
}

fn pyramids() -> WonderInfo {
    WonderInfo::builder(
        Wonder::Pyramids,
        "Pyramids",
        "Counts as 5.1 victory points (instead of 4). \
            All victory points are awarded to the player who built the wonder \
            (owning does not grant any points).",
        PaymentOptions::fixed_resources(ResourcePile::new(2, 3, 7, 0, 0, 0, 5)),
        Advance::Rituals,
    )
    .built_victory_points(5.1) // because it breaks the tie
    .owned_victory_points(0)
    .build()
}

fn colosseum() -> WonderInfo {
    WonderInfo::builder(
        Wonder::Colosseum,
        "Colosseum",
        "May pay culture tokens with mood tokens (or vice versa) - \
        except for the building wonders.\
        May increase the combat value in a land battle by 1 for 1 culture or mood token.",
        PaymentOptions::fixed_resources(ResourcePile::new(3, 4, 5, 0, 0, 0, 5)),
        Advance::Sports,
    )
    .add_payment_request_listener(
        |e| &mut e.combat_round_end,
        90,
        |game, player_index, e| {
            let player = &game.player(player_index);

            let cost = PaymentOptions::tokens(player, PaymentReason::WonderAbility, 1);

            if !player.can_afford(&cost) {
                return None;
            }
            
            let h = e.hits_mut(e.role(player_index));
            let mut with_increase = h.clone();
            with_increase.combat_value += 1;
            if h.hits() == with_increase.hits() {
                game.add_info_log_item(&format!(
                    "Combat value is already at maximum, cannot increase combat value for {}",
                    game.player_name(player_index)
                ));
                return None;
            }

            Some(vec![PaymentRequest::optional(
                cost,
                "Add 1 to the combat value?",
            )])
        },
        |game, s, e| {
            let pile = &s.choice[0];
            let name = &s.player_name;
            if pile.is_empty() {
                game.add_info_log_item(&format!("{name} declined to pay for the combat value",));
            } else {
                game.add_info_log_item(&format!(
                    "{name} paid {pile} to increase the combat value by 1, scoring an extra hit",
                ));
                e.hits_mut(e.role(s.player_index)).combat_value += 1;
                e.set_final_result();
            }
        },
    )
    .build()
}
