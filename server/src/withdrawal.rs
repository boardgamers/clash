//! Withdrawal is a platform policy, not a civilization conversion or a full-turn AI.
//! Only explicit dropped seats are automated. Optional commitments are declined;
//! mandatory choices use stable request order and the ordinary legality constraints.
use crate::action::Action;
use crate::content::persistent_events::{
    EventResponse, MultiRequest, PaymentRequest, PersistentEventRequest, PersistentEventType,
    is_selected_structures_valid,
};
use crate::game::{Game, GameState};
use crate::resource::ResourceType::*;
use crate::resource_pile::ResourcePile;
use itertools::Itertools;
use std::collections::HashSet;

const SEARCH_LIMIT: usize = 100_000;

pub(crate) const AMBIGUOUS_OWNER: &str =
    "Cannot prove the owner of the legacy withdrawn turn-start continuation";
pub(crate) const CASCADE_LIMIT: &str = "Automatic decision cascade exceeded 512 steps";
pub(crate) const NO_PROGRESS: &str = "Withdrawal response made no progress";

fn first_selection<T: Clone>(
    request: &MultiRequest<T>,
    valid: impl Fn(&[T]) -> bool,
) -> Result<Vec<T>, String> {
    let mut budget = SEARCH_LIMIT;
    for count in *request.needed.start()..=*request.needed.end() {
        for selected in request
            .choices
            .iter()
            .cloned()
            .combinations(usize::from(count))
        {
            if budget == 0 {
                return Err("Withdrawal selection search limit exceeded".into());
            }
            budget -= 1;
            if valid(&selected) {
                return Ok(selected);
            }
        }
    }
    Err("No legal withdrawal selection".into())
}

// Search whole payment groups, not greedily: a flexible first cost must not use
// the only resource that can satisfy a later fixed cost. Branch by reducing one
// component of a rejected recommendation. Any useful alternative must conserve
// at least one component; spending more of every component cannot help the suffix.
fn payments(
    requests: &[PaymentRequest],
    available: &ResourcePile,
) -> Result<Vec<ResourcePile>, String> {
    fn search(
        requests: &[PaymentRequest],
        available: &ResourcePile,
        budget: &mut usize,
    ) -> Option<Vec<ResourcePile>> {
        let Some((request, rest)) = requests.split_first() else {
            return Some(vec![]);
        };
        if request.optional {
            let mut suffix = search(rest, available, budget)?;
            suffix.insert(0, ResourcePile::empty());
            return Some(suffix);
        }
        let mut caps = vec![available.clone()];
        let mut visited = HashSet::new();
        while let Some(cap) = caps.pop() {
            if !visited.insert(cap.clone()) {
                continue;
            }
            if *budget == 0 {
                return None;
            }
            *budget -= 1;
            let Some(payment) = request.cost.first_valid_payment(&cap) else {
                continue;
            };
            let mut remaining = available.clone();
            remaining -= payment.clone();
            if let Some(mut suffix) = search(rest, &remaining, budget) {
                suffix.insert(0, payment);
                return Some(suffix);
            }
            for resource in [
                Captives,
                CultureTokens,
                MoodTokens,
                Gold,
                Ideas,
                Ore,
                Wood,
                Food,
            ] {
                let amount = payment.get(&resource);
                if amount > 0 {
                    let mut next = cap.clone();
                    *next.get_mut(&resource) = amount - 1;
                    caps.push(next);
                }
            }
        }
        None
    }
    let mut budget = SEARCH_LIMIT;
    search(requests, available, &mut budget)
        .ok_or_else(|| "No legal withdrawal payment within search budget".into())
}

pub(crate) fn response(game: &Game) -> Result<EventResponse, String> {
    let event = game.events.last().ok_or("No withdrawal event")?;
    let handler = event
        .player
        .handler
        .as_ref()
        .ok_or("Withdrawal event has no handler")?;
    let player = game.player(event.player.index);
    let missing = || "Withdrawal request has no legal choice".to_string();
    Ok(match &handler.request {
        PersistentEventRequest::Payment(requests) => {
            EventResponse::Payment(payments(requests, &player.resources)?)
        }
        PersistentEventRequest::ResourceReward(request) => {
            let options = &request.reward.payment_options;
            let reward = options.default.clone();
            if !options.is_valid_payment(&reward) {
                return Err("Invalid default withdrawal reward".into());
            }
            EventResponse::ResourceReward(reward)
        }
        PersistentEventRequest::SelectAdvance(r) => {
            EventResponse::SelectAdvance(*r.choices.first().ok_or_else(missing)?)
        }
        PersistentEventRequest::SelectPlayer(r) => {
            EventResponse::SelectPlayer(*r.choices.first().ok_or_else(missing)?)
        }
        PersistentEventRequest::SelectUnitType(r) => {
            EventResponse::SelectUnitType(*r.choices.first().ok_or_else(missing)?)
        }
        PersistentEventRequest::SelectPositions(r) => {
            EventResponse::SelectPositions(first_selection(r, |_| true)?)
        }
        PersistentEventRequest::SelectCaptives(r) => {
            EventResponse::SelectCaptives(first_selection(r, |_| true)?)
        }
        PersistentEventRequest::SelectUnits(r) => {
            EventResponse::SelectUnits(first_selection(&r.request, |s| {
                crate::unit::validate_units_selection(s, game, game.player(r.player)).is_ok()
            })?)
        }
        PersistentEventRequest::SelectHandCards(r) => {
            EventResponse::SelectHandCards(first_selection(r, |s| {
                crate::card::validate_card_selection(s, game).is_ok()
            })?)
        }
        PersistentEventRequest::SelectStructures(r) => {
            let mut selected = first_selection(r, |s| {
                let mut sorted = s.to_vec();
                sorted.sort();
                is_selected_structures_valid(game, &sorted)
            })?;
            selected.sort();
            EventResponse::SelectStructures(selected)
        }
        PersistentEventRequest::BoolRequest(_) => EventResponse::Bool(false),
        PersistentEventRequest::ChangeGovernment => {
            let count = crate::status_phase::government_advances(player, game)
                .len()
                .saturating_sub(1);
            let government = game
                .cache
                .get_governments()
                .iter()
                .find(|g| {
                    player.can_advance_ignore_contradicting(g.advances[0].advance, game)
                        && g.advances.len() > count
                })
                .ok_or_else(missing)?;
            EventResponse::ChangeGovernmentType(crate::status_phase::ChangeGovernment::new(
                government.name.clone(),
                government
                    .advances
                    .iter()
                    .skip(1)
                    .take(count)
                    .map(|a| a.advance)
                    .collect(),
            ))
        }
        PersistentEventRequest::ExploreResolution => {
            let PersistentEventType::ExploreResolution(explore) = &event.event_type else {
                return Err("Invalid withdrawal exploration event".into());
            };
            EventResponse::ExploreResolution(explore.block.position.rotation)
        }
    })
}

pub(crate) fn discard_plans(game: &mut Game) {
    game.early_choices
        .retain(|choice| !game.dropped_players.contains(&choice.player));
    if game
        .action_plan
        .as_ref()
        .is_some_and(|plan| game.dropped_players.contains(&plan.player))
    {
        game.action_plan = None;
    }
}

/// One legal step only. The shared live boundary records and drains these steps.
/// Never skip an unresolved event, including events owned by a living opponent.
pub(crate) fn next_action(game: &Game) -> Result<Option<Action>, String> {
    if matches!(game.state, GameState::Finished)
        || game.civilization_draft.is_some()
        || !game.dropped_players.contains(&game.active_player())
    {
        return Ok(None);
    }
    if !game.events.is_empty() {
        return response(game).map(|response| Some(Action::Response(response)));
    }
    Ok(match game.state {
        GameState::Playing => Some(Action::Playing(
            crate::playing_actions::PlayingAction::EndTurn,
        )),
        GameState::Movement(_) => Some(Action::Movement(crate::movement::MovementAction::Stop)),
        GameState::ChooseCivilization => Some(Action::ChooseCivilization(
            crate::game_setup::civilization_choices(game, game.active_player())
                .first()
                .ok_or("No civilization available for withdrawn seat")?
                .name
                .clone(),
        )),
        GameState::Finished => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventOrigin;
    use crate::payment::{PaymentConversion, PaymentOptions};

    #[test]
    fn payment_groups_backtrack_and_decline_optional_without_rng() {
        let origin = EventOrigin::Ability("withdrawal test".into());
        let flexible = PaymentOptions::new(
            ResourcePile::food(1),
            vec![PaymentConversion::unlimited(
                ResourcePile::food(1),
                ResourcePile::wood(1),
            )],
            vec![],
            origin.clone(),
        );
        let fixed = PaymentOptions::new(ResourcePile::food(1), vec![], vec![], origin);
        let requests = vec![
            PaymentRequest::optional(fixed.clone(), "decline"),
            PaymentRequest::mandatory(flexible, "flexible"),
            PaymentRequest::mandatory(fixed, "fixed"),
        ];
        let available = ResourcePile::food(1) + ResourcePile::wood(1);
        assert_eq!(
            payments(&requests, &available).unwrap(),
            vec![
                ResourcePile::empty(),
                ResourcePile::wood(1),
                ResourcePile::food(1)
            ]
        );
        assert!(payments(&requests, &ResourcePile::food(1)).is_err());
    }

    #[test]
    fn pathological_selection_exhausts_budget_explicitly() {
        let request = MultiRequest::new((0..24).collect::<Vec<_>>(), 12..=12, "mandatory");
        assert_eq!(
            first_selection(&request, |_| false).unwrap_err(),
            "Withdrawal selection search limit exceeded"
        );
        let empty = MultiRequest::<u8>::new(vec![], 1..=1, "mandatory");
        assert_eq!(
            first_selection(&empty, |_| true).unwrap_err(),
            "No legal withdrawal selection"
        );
    }

    #[test]
    fn selection_prefers_minimum_but_searches_past_invalid_prefix() {
        let request = MultiRequest::new(vec![1, 2, 3], 2..=3, "mandatory");
        assert_eq!(
            first_selection(&request, |s| !s.contains(&1)).unwrap(),
            vec![2, 3]
        );
        let optional = MultiRequest::new(vec![1, 2, 3], 0..=3, "optional");
        assert!(first_selection(&optional, |_| true).unwrap().is_empty());
    }
}
