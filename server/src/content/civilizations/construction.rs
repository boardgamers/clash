// Shared immediate construction purchase for leader abilities.
use crate::ability_initializer::AbilityInitializerSetup;
use crate::city_pieces::{BUILDINGS, Building};
use crate::construct::{Construct, can_construct, new_building_positions};
use crate::content::ability::AbilityBuilder;
use crate::content::persistent_events::{
    PaymentRequest, PositionRequest, SelectedStructure, StructuresRequest,
};
use crate::game::Game;
use crate::leader::leader_position;
use crate::player::{CostTrigger, Player};
use crate::player_events::CostInfo;
use crate::resource_pile::ResourcePile;
use crate::structure::Structure;

fn cost(game: &Game, p: &Player, target: &SelectedStructure) -> Option<CostInfo> {
    let city = p.try_get_city(target.position)?;
    match target.structure {
        Structure::Building(b) => {
            can_construct(city, b, p, game, CostTrigger::NoModifiers, &[]).ok()
        }
        Structure::Wonder(w) => {
            let mut cost = crate::wonder::wonder_cost(game, p, w);
            cost.ignore_action_cost = true;
            crate::wonder::can_construct_wonder(city, w, p, game, cost, CostTrigger::NoModifiers)
                .ok()
        }
        _ => None,
    }
}
pub(crate) fn choices(game: &Game, p: &Player, reserve: &ResourcePile) -> Vec<SelectedStructure> {
    if !p.resources.has_at_least(reserve) {
        return vec![];
    }
    let mut resources = p.resources.clone();
    resources -= reserve.clone();
    let pos = leader_position(p);
    let Some(city) = p.try_get_city(pos) else {
        return vec![];
    };
    BUILDINGS
        .into_iter()
        .map(Structure::Building)
        .chain(p.wonder_cards.iter().copied().map(Structure::Wonder))
        .map(|s| SelectedStructure::new(pos, s))
        .filter(|s| {
            if let Structure::Building(b) = s.structure {
                if new_building_positions(game, b, city).is_empty() {
                    return false;
                }
            }
            cost(game, p, s).is_some_and(|c| c.cost.first_valid_payment(&resources).is_some())
        })
        .collect()
}
pub(crate) fn purchase(b: AbilityBuilder) -> AbilityBuilder {
    b.add_structures_request(
        |e| &mut e.custom_action,
        3,
        |game, p, _| {
            Some(StructuresRequest::new(
                choices(game, p.get(game), &ResourcePile::empty()),
                1..=1,
                "Choose what to construct in the leader's city",
            ))
        },
        |game, s, a| {
            let target = s.choice[0].clone();
            a.purchase_cost = cost(game, s.player().get(game), &target);
            a.selected_structure = Some(target);
        },
    )
    .add_position_request(
        |e| &mut e.custom_action,
        2,
        |game, p, a| {
            let target = a.selected_structure.as_ref().unwrap();
            if target.structure != Structure::Building(Building::Port) {
                return None;
            }
            Some(PositionRequest::new(
                new_building_positions(game, Building::Port, p.get(game).get_city(target.position))
                    .into_iter()
                    .flatten()
                    .collect(),
                1..=1,
                "Place the Port",
            ))
        },
        |_, s, a| a.action.city = Some(s.choice[0]),
    )
    .add_payment_request_listener(
        |e| &mut e.custom_action,
        1,
        |_, _, a| {
            Some(vec![PaymentRequest::mandatory(
                a.purchase_cost.as_ref().unwrap().cost.clone(),
                "Construction cost",
            )])
        },
        |game, s, a| {
            let cost = a.purchase_cost.take().unwrap();
            cost.info.execute(game);
            let target = a.selected_structure.take().unwrap();
            match target.structure {
                Structure::Building(b) => crate::construct::do_construct(
                    game,
                    s.player_index,
                    &Construct::new(target.position, b, s.choice[0].clone())
                        .with_port_position(a.action.city),
                    cost.activate_city,
                    &s.origin,
                ),
                Structure::Wonder(w) => {
                    s.player().get_mut(game).wonder_cards.retain(|c| *c != w);
                    crate::wonder::construct_wonder(game, &s.player(), w, target.position);
                }
                _ => unreachable!(),
            }
        },
    )
}
