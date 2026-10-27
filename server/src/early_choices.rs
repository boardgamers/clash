//! Private, editable commitments. They never change resolution order or reserve a
//! shared card. Only an ordinary, currently legal response can consume a plan.
use crate::card::HandCard;
use crate::content::persistent_events::{EventResponse, PersistentEventRequest, PersistentEventType};
use crate::game::Game;
use crate::resource_pile::ResourcePile;
use crate::status_phase::StatusPhaseState;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedChoice {
    pub player: usize,
    pub key: String,
    pub response: EventResponse,
}

#[derive(Serialize)]
pub struct OptionChoice {
    pub label: String,
    pub description: String,
    pub response: EventResponse,
}

#[derive(Serialize)]
pub struct Offer {
    pub key: String,
    pub title: String,
    pub options: Vec<OptionChoice>,
    pub selected: Option<EventResponse>,
}

fn scope(game: &Game, index: usize) -> Option<String> {
    let name = match &game.events.get(index)?.event_type {
        PersistentEventType::StatusPhase(phase) => format!("status:{phase:?}"),
        PersistentEventType::Incident(i) => format!("person:{}:{}", i.incident_id, i.active_player),
        _ => return None,
    };
    Some(format!("{}:{}:{index}:{name}/", game.age, game.round))
}

fn option(label: impl Into<String>, response: EventResponse) -> OptionChoice {
    OptionChoice { label: label.into(), description: String::new(), response }
}

fn objective_key(game: &Game, name: &str) -> String {
    format!("{}:objective:{name}", game.age)
}

fn objective_offers(game: &Game, player: usize) -> Vec<Offer> {
    let p = game.player(player);
    let mut names: Vec<_> = p.objective_cards.iter().filter(|id| **id != 0).flat_map(|id|
        crate::objective_card::status_phase_completable(game, p, *id)).collect();
    names.sort(); names.dedup();
    names.into_iter().map(|name| {
        let key = objective_key(game, &name);
        let mut options = vec![option("Skip", EventResponse::SelectHandCards(vec![]))];
        options.extend(p.objective_cards.iter().filter(|id| **id != 0).filter(|id|
            crate::objective_card::status_phase_completable(game, p, **id).contains(&name))
            .map(|id| option(game.cache.get_objective_card(*id).name(),
                EventResponse::SelectHandCards(vec![HandCard::ObjectiveCard(*id)]))));
        let selected = game.early_choices.iter().find(|c| c.player == player && c.key == key).map(|c| c.response.clone());
        Offer {key, title: name, options, selected}
    }).collect()
}

pub fn prune_objectives(game: &mut Game) {
    let invalid: Vec<_> = game.early_choices.iter().enumerate().filter_map(|(index, choice)| {
        if !choice.key.contains(":objective:") { return None; }
        let valid = objective_offers(game, choice.player).iter().any(|o| o.key == choice.key
            && o.options.iter().any(|v| v.response == choice.response));
        (!valid).then_some(index)
    }).collect();
    for index in invalid.into_iter().rev() { game.early_choices.remove(index); }
}

/// Expose only the current step, never future cards, random results, or another
/// player's hand. Earlier players may still change the available options.
pub fn offers(game: &Game, player: usize) -> Vec<Offer> {
    if player >= game.players.len() || !game.player(player).is_human()
        || game.dropped_players.contains(&player) || game.civilization_draft.is_some() || matches!(game.state, crate::game::GameState::Finished) {
        return vec![];
    }
    if game.events.is_empty() { return objective_offers(game, player); }
    let Some(index) = game.events.iter().rposition(|e| matches!(e.event_type,
        PersistentEventType::StatusPhase(_) | PersistentEventType::Incident(_))) else { return vec![]; };
    let event = &game.events[index];
    let first = if matches!(event.event_type, PersistentEventType::StatusPhase(StatusPhaseState::ChangeGovernmentType(_))) {
        let players: Vec<_> = game.human_players_sorted(game.starting_player_index).iter().map(|i| game.player(*i)).collect();
        if crate::status_phase::player_that_chooses_next_first_player(&players) == player {
            let key = format!("{}:first", game.age);
            vec![Offer { selected: game.early_choices.iter().find(|c| c.player == player && c.key == key).map(|c| c.response.clone()), key,
                title: "Choose first player".into(), options: game.human_players_sorted(game.starting_player_index).iter().map(|i| option(game.player(*i).civilization.name.clone(), EventResponse::SelectPlayer(*i))).collect() }]
        } else { vec![] }
    } else { vec![] };
    if event.players_used.contains(&player) { return first; }
    // Nested research/event reactions must finish before early status choices are
    // offered. Objective selection is the only supported nested request.
    if game.events[index+1..].iter().any(|e| !matches!(e.event_type, PersistentEventType::SelectObjectives(_))) {
        return vec![];
    }
    let prefix = scope(game, index).expect("supported event");
    let p = game.player(player);
    let mut result: Vec<Offer> = first;
    let mut add = |suffix: &str, title: String, options: Vec<OptionChoice>| {
        if options.is_empty() { return; }
        let key = format!("{prefix}{suffix}");
        let selected = game.early_choices.iter().find(|c| c.player == player && c.key == key).map(|c| c.response.clone());
        result.push(Offer { key, title, options, selected });
    };
    match &event.event_type {
        PersistentEventType::StatusPhase(StatusPhaseState::FreeAdvance) => {
            let mut options: Vec<_> = game.cache.get_advances().values()
                .filter(|a| p.can_advance_free(a.advance, game))
                .map(|a| OptionChoice { label: a.name.clone(), description: a.description.clone(), response: EventResponse::SelectAdvance(a.advance) }).collect();
            options.sort_by(|a,b| a.label.cmp(&b.label));
            add("advance", "Free Advance".into(), options);
        }
        PersistentEventType::StatusPhase(StatusPhaseState::RazeSize1City) => {
            let mut options: Vec<_> = p.cities.iter().filter(|c| c.size() == 1)
                .map(|c| option(c.position.to_string(), EventResponse::SelectPositions(vec![c.position]))).collect();
            if !options.is_empty() {
                options.insert(0, option("Keep all cities", EventResponse::SelectPositions(vec![])));
                add("raze", "Raze city".into(), options);
            }
        }
        PersistentEventType::StatusPhase(StatusPhaseState::CompleteObjectives) => {
            return objective_offers(game, player);
        }
        PersistentEventType::Incident(i) if !i.consumed && !i.replaced_by_sacrifice && i.selected_player.is_none()
            && game.cache.get_incident(i.incident_id).action_card.is_some() => {
            // Do not ask before event cancellation, placement or other base
            // effects. Once an offer is visible, later players may precommit.
            if !event.player.handler.as_ref().is_some_and(|h| matches!(&h.request,
                PersistentEventRequest::Payment(rs) if rs.iter().any(|r| r.name == "Pay to gain the Action Card"))) {
                return vec![];
            }
            let cost = if player == i.active_player { 1 } else { 2 };
            let mut options = vec![option("Pass", EventResponse::Payment(vec![ResourcePile::empty()]))];
            // No inferred gold/captive conversion: a queued purchase spends
            // precisely the displayed culture payment, or asks again.
            if p.resources.culture_tokens >= cost {
                options.push(option(format!("Take if offered · {cost} culture"), EventResponse::Payment(vec![ResourcePile::culture_tokens(cost)])));
            }
            add("offer", game.cache.get_incident(i.incident_id).name.clone(), options);
        }
        _ => {}
    }
    result
}

pub fn can_save(game: &Game, player: usize, key: &str, response: Option<&EventResponse>) -> bool {
    (player != game.active_player() || (game.events.is_empty() && key.contains(":objective:"))) && offers(game, player).iter().any(|offer| offer.key == key
        && response.is_none_or(|r| offer.options.iter().any(|o| &o.response == r)))
}

pub fn save(game: &mut Game, player: usize, key: &str, response: Option<EventResponse>) -> Result<(), String> {
    if !can_save(game, player, key, response.as_ref()) { return Err("This early choice is no longer available".into()); }
    game.early_choices.retain(|c| c.player != player || (c.key != key && !matches!((&c.response, &response),
        (EventResponse::SelectHandCards(a), Some(EventResponse::SelectHandCards(b))) if !a.is_empty() && a == b && key.contains(":objective:"))));
    if let Some(response) = response { game.early_choices.push(SavedChoice { player, key: key.into(), response }); }
    // Metadata-only platform update; not a turn, journal entry or undo boundary.
    game.early_choice_update = true;
    Ok(())
}

pub fn expire_current_event(game: &mut Game) {
    if game.events.last().is_some_and(|e| matches!(e.event_type, PersistentEventType::StatusPhase(StatusPhaseState::CompleteObjectives))) {
        let prefix = format!("{}:objective:", game.age);
        game.early_choices.retain(|c| !c.key.starts_with(&prefix));
    }
    if game.events.last().is_some_and(|e| matches!(e.event_type, PersistentEventType::StatusPhase(StatusPhaseState::DetermineFirstPlayer(_)))) {
        game.early_choices.retain(|c| c.key != format!("{}:first", game.age));
    }
    if let Some(prefix) = scope(game, game.events.len().saturating_sub(1)) {
        game.early_choices.retain(|c| !c.key.starts_with(&prefix));
    }
}

/// Validate against the *real* request, including costs and prerequisites after
/// intervening events. Invalid plans are removed and the normal prompt remains.
pub fn take_ready(game: &mut Game) -> Option<EventResponse> {
    let player = game.active_player();
    let event = game.events.last()?;
    let handler = event.player.handler.as_ref()?;
    let (index, suffix) = match &event.event_type {
        PersistentEventType::StatusPhase(StatusPhaseState::DetermineFirstPlayer(_)) => (game.events.len()-1, "first".into()),
        PersistentEventType::StatusPhase(StatusPhaseState::FreeAdvance) => (game.events.len()-1, "advance".into()),
        PersistentEventType::StatusPhase(StatusPhaseState::RazeSize1City) => (game.events.len()-1, "raze".into()),
        PersistentEventType::SelectObjectives(i) => {
            let index = game.events.iter().rposition(|e| matches!(e.event_type, PersistentEventType::StatusPhase(StatusPhaseState::CompleteObjectives)))?;
            (index, format!("objective:{}", i.shown_objective.as_ref()?))
        }
        PersistentEventType::Incident(_) if matches!(&handler.request, PersistentEventRequest::Payment(rs)
            if rs.iter().any(|r| r.name == "Pay to gain the Action Card")) => (game.events.len()-1, "offer".into()),
        _ => return None,
    };
    let key = if suffix == "first" { format!("{}:first", game.age) } else if let Some(name) = suffix.strip_prefix("objective:") {
        objective_key(game, name)
    } else { format!("{}{suffix}", scope(game, index)?) };
    let at = game.early_choices.iter().position(|c| c.player == player && c.key == key)?;
    let response = &game.early_choices[at].response;
    let valid = match (&handler.request, response) {
        (PersistentEventRequest::SelectPlayer(r), EventResponse::SelectPlayer(p)) => r.choices.contains(p),
        (PersistentEventRequest::SelectAdvance(r), EventResponse::SelectAdvance(a)) => r.choices.contains(a),
        (PersistentEventRequest::SelectPositions(r), EventResponse::SelectPositions(v)) => r.needed.contains(&(v.len() as u8)) && v.iter().all(|p| r.choices.contains(p)),
        (PersistentEventRequest::SelectHandCards(r), EventResponse::SelectHandCards(v)) => r.needed.contains(&(v.len() as u8)) && v.iter().all(|c| r.choices.contains(c)),
        (PersistentEventRequest::Payment(rs), EventResponse::Payment(v)) => rs.len() == 1 && v.len() == 1
            && game.player(player).resources.has_at_least(&v[0])
            && ((rs[0].optional && v[0].is_empty()) || rs[0].cost.is_valid_payment(&v[0])),
        _ => false,
    };
    let choice = game.early_choices.remove(at);
    valid.then_some(choice.response)
}
