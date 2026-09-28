use crate::action::Action;
use crate::advance::AdvanceAction;
use crate::card::HandCard;
use crate::city::MoodState;
use crate::collect::{
    Collect, PositionCollection, collect_event_origin, get_total_collection,
    possible_resource_collections,
};
use crate::consts::OBJECTIVE_VICTORY_POINTS;
use crate::content::persistent_events::{
    EventResponse, PersistentEventRequest, PersistentEventType,
};
use crate::game::{Game, GameState};
use crate::objective_card::ObjectiveType;
use crate::player::CostTrigger;
use crate::playing_actions::{PlayingAction, PlayingActionType};
use crate::position::Position;
use crate::victory_points::victory_points_parts;
use crate::wonder::Wonder;
use serde_json::{Value, json};
mod actions;
pub use actions::recruit_preview;

pub fn view(game: &Game, seat: Option<usize>) -> Value {
    let seat = seat.filter(|i| *i < game.players.len() && game.player(*i).is_human());
    let active = game.active_player();
    let playing = game.state == GameState::Playing && game.events.is_empty();
    let objective_phase = game.events.last().is_some_and(|event| {
        matches!(event.event_type, PersistentEventType::SelectObjectives(_))
            && matches!(
                game.current_event_handler().map(|h| &h.request),
                Some(PersistentEventRequest::SelectHandCards(_))
            )
    });
    let moving = matches!(game.state, GameState::Movement(_)) && game.events.is_empty();
    let choice = seat.and_then(|seat| choice_decision(game, seat));
    let exploration = seat.and_then(|seat| exploration_decision(game, seat));
    let supported_phase = playing || objective_phase || moving || choice.is_some() || exploration.is_some();
    let can_play = seat == Some(active) && playing;
    let players = game.players.iter().filter(|p| p.is_human()).map(|p| json!({
        "index": p.index, "name": game.player_name(p.index), "civilization": p.civilization.name,
        "score": p.victory_points(game),
        "scoreParts": victory_points_parts(p, game).map(|(name, points)| json!({"name": name, "points": points})),
        "cities": p.cities.iter().map(|c| json!({"position": c.position, "size": c.size(), "capacity": c.mood_modified_size(p), "mood": c.mood_state, "activations": c.activations})).collect::<Vec<_>>()
    })).collect::<Vec<_>>();
    let Some(seat) = seat else {
        return json!({"activePlayer": active, "canPlay": false, "supportedPhase": supported_phase, "players": players, "cities": [], "advances": [], "objectiveCards": [], "wonderCards": [], "objectiveDecision": null, "cityActions": [], "settlers": [], "stopMovement": null, "canUndo": false, "canEndTurn": false});
    };
    let p = game.player(seat);
    let wonder_cards = p.wonder_cards.iter().filter(|wonder| **wonder != Wonder::Hidden).map(|wonder| {
        let info = wonder.info(game);
        json!({"id": wonder, "name": info.name(), "description": info.description,
            "cost": info.cost.default_payment(), "requiredAdvance": info.required_advance.name(game),
            "requiredAdvanceOwned": p.has_advance(info.required_advance),
            "builtPoints": info.built_victory_points, "ownedPoints": info.owned_victory_points})
    }).collect::<Vec<_>>();
    // Hidden cards use ID zero in the player-filtered state; never resolve them.
    let objective_cards = p
        .objective_cards
        .iter()
        .filter(|id| **id != 0)
        .map(|id| {
            let card = game.cache.get_objective_card(*id);
            json!({"id": card.id, "objectives": card.objectives.iter().map(|objective| json!({
            "name": objective.name,
            "description": objective.description,
            "timing": match objective.get_type() {
                ObjectiveType::Instant => "Instant",
                ObjectiveType::StatusPhase => "Status phase",
            }
        })).collect::<Vec<_>>()})
        })
        .collect::<Vec<_>>();
    let collect_type = PlayingActionType::Collect;
    let collect_reason = collect_type.is_available(game, seat).err();
    let origin = collect_event_origin(&collect_type, p);
    let cities = p.cities.iter().map(|city| {
        let mut after_activation = crate::city::City::from_data(city.cloned_data(), seat);
        if city.is_activated() {
            after_activation.mood_state = match city.mood_state { MoodState::Happy => MoodState::Neutral, _ => MoodState::Angry };
        }
        let info = possible_resource_collections(game, city.position, seat, &origin, CostTrigger::NoModifiers);
        let mut choices = info.choices.iter().flat_map(|(position, piles)| piles.iter().map(move |pile| json!({"position":position,"pile":pile}))).collect::<Vec<_>>();
        choices.sort_by_key(Value::to_string);
        let reason = if !can_play { Some("Wait for your turn".to_string()) } else if !city.can_activate() { Some("This city has already been activated while angry".to_string()) } else { collect_reason.clone() };
        json!({"position":city.position,"size":city.size(),"capacity":info.max_selection,"maxPerTile":info.max_per_tile,"maxRange2":info.max_range2_tiles,"mood":city.mood_state,"activations":city.activations,"reason":reason,"choices":choices,
            "canActivate":city.can_activate(), "activationMood":after_activation.mood_state,"activationCapacity":after_activation.mood_modified_size(p)})
    }).collect::<Vec<_>>();
    let mut advances = game.cache.get_advances().iter().map(|(advance, info)| {
        let cost = p.advance_cost(*advance, game, CostTrigger::NoModifiers).cost;
        let payment = cost.first_valid_payment(&p.resources);
        let owned = p.has_advance(*advance);
        let reason = if owned { Some("Already researched".to_string()) }
        else if !can_play { Some("Wait for your turn".to_string()) }
        else if let Err(reason) = PlayingActionType::Advance.is_available(game, seat) { Some(reason) }
        else if let Some(required) = info.required.filter(|a| !p.has_advance(*a)) { Some(format!("Requires {}", required.name(game))) }
        else if !p.can_advance_free(*advance, game) { Some("Conflicts with your current advances".to_string()) }
        else if payment.is_none() { Some("Not enough resources".to_string()) } else { None };
        let action = if reason.is_none() { payment.clone().map(|payment| Action::Playing(PlayingAction::Advance(AdvanceAction::new(*advance, payment)))) } else { None };
        let group = game.cache.get_advance_groups().iter().enumerate().find_map(|(group_index, group)| {
            group.advances.iter().position(|a| a.advance == *advance).map(|index| (group.name.clone(), group_index * 10 + index))
        });
        json!({"id":advance,"name":info.name,"description":info.description,"owned":owned,"reason":reason,"payment":payment.unwrap_or_else(|| cost.default_payment()),"action":action,
            "group":group.as_ref().map(|g| &g.0),"order":group.as_ref().map(|g| g.1),"required":info.required,
            "bonus":info.bonus.as_ref().map(|bonus| bonus.resources()),"unlocks":info.unlocked_building.map(|building| building.to_string())})
    }).collect::<Vec<_>>();
    advances.sort_by_key(|a| a["name"].as_str().unwrap_or_default().to_string());
    json!({"activePlayer":active,"canPlay":can_play,"supportedPhase":supported_phase,"players":players,"cities":cities,"advances":advances,"objectiveCards":objective_cards,"objectiveDecision":objective_decision(game, seat),
        "choiceDecision":choice, "explorationDecision":exploration, "wonderCards":wonder_cards,
        "cityActions":actions::cities(game, seat, can_play), "settlers":actions::settlers(game, seat, (can_play && PlayingActionType::MoveUnits.is_available(game,seat).is_ok()) || (moving && seat == active)),
        "stopMovement":if moving && seat == active {Some(Action::Movement(crate::movement::MovementAction::Stop))} else {None},
        "canUndo":seat == active && game.can_undo(),"canEndTurn":can_play && PlayingActionType::EndTurn.is_available(game, seat).is_ok()})
}

fn exploration_decision(game: &Game, seat: usize) -> Option<Value> {
    let event = game.events.last()?;
    if event.player.index != seat {
        return None;
    }
    let PersistentEventType::ExploreResolution(state) = &event.event_type else {
        return None;
    };
    let handler = game.current_event_handler()?;
    if handler.response.is_some()
        || !matches!(handler.request, PersistentEventRequest::ExploreResolution)
    {
        return None;
    }
    // The engine has already revealed this region and applied forced-placement rules.
    // Only the original and opposite orientations are valid for this request.
    let base = state.block.position.rotation;
    let choices = [base, (base + 3) % 6].map(|rotation| {
        json!({"rotation":rotation,"tiles":state.block.block.tiles(&state.block.position,rotation),
            "action":Action::Response(EventResponse::ExploreResolution(rotation))})
    });
    Some(json!({"start":state.start,"destination":state.destination,"choices":choices}))
}

fn choice_decision(game: &Game, seat: usize) -> Option<Value> {
    use crate::resource::ResourceType;
    use crate::resource_pile::ResourcePile;
    let event = game.events.last()?;
    if event.player.index != seat {
        return None;
    }
    let handler = game.current_event_handler()?;
    if handler.response.is_some() {
        return None;
    }
    match &handler.request {
        PersistentEventRequest::ResourceReward(request)
            if request.reward.payment_options.default.amount() == 1 =>
        {
            let choices = ResourceType::all().into_iter().map(|r| ResourcePile::of(r,1))
                .filter(|pile|request.reward.payment_options.is_valid_payment(pile))
                .map(|pile|json!({"name":pile.to_string(),"pile":pile,"action":Action::Response(EventResponse::ResourceReward(pile.clone()))})).collect::<Vec<_>>();
            Some(json!({"name":request.name,"choices":choices}))
        }
        PersistentEventRequest::BoolRequest(name) => Some(json!({"name":name,"choices":[
            {"name":"Yes","action":Action::Response(EventResponse::Bool(true))},
            {"name":"No","action":Action::Response(EventResponse::Bool(false))}]})),
        _ => None,
    }
}

fn objective_decision(game: &Game, seat: usize) -> Option<Value> {
    let event = game.events.last()?;
    if seat != event.player.index {
        return None;
    }
    let PersistentEventType::SelectObjectives(info) = &event.event_type else {
        return None;
    };
    let name = info.shown_objective.as_ref()?;
    let handler = game.current_event_handler()?;
    let PersistentEventRequest::SelectHandCards(request) = &handler.request else {
        return None;
    };
    if handler.response.is_some() || !request.needed.contains(&1) {
        return None;
    }
    let cards = request.choices.iter().filter_map(|card| {
        let HandCard::ObjectiveCard(id) = card else { return None; };
        if *id == 0 || !game.player(seat).objective_cards.contains(id) { return None; }
        let info = game.cache.get_objective_card(*id);
        Some(json!({"id":id,"name":info.name(),"action":Action::Response(EventResponse::SelectHandCards(vec![card.clone()]))}))
    }).collect::<Vec<_>>();
    let skip = request
        .needed
        .contains(&0)
        .then(|| Action::Response(EventResponse::SelectHandCards(vec![])));
    Some(
        json!({"name":name,"description":game.cache.get_objective(name).description,"points":OBJECTIVE_VICTORY_POINTS,"cards":cards,"skip":skip}),
    )
}

pub fn collect_preview(
    game: &Game,
    seat: usize,
    city: Position,
    selections: Vec<PositionCollection>,
) -> Result<Value, String> {
    if seat >= game.players.len() || !game.player(seat).is_human() || seat != game.active_player() {
        return Err("Wait for your turn".to_string());
    }
    PlayingActionType::Collect.is_available(game, seat)?;
    let p = game.player(seat);
    let city_data = p.try_get_city(city).ok_or("Choose one of your cities")?;
    if !city_data.can_activate() {
        return Err("This city cannot be activated again this turn".to_string());
    }
    if selections.iter().any(|c| c.times == 0) {
        return Err("Choose at least one resource".to_string());
    }
    let origin = collect_event_origin(&PlayingActionType::Collect, p);
    let total = get_total_collection(
        game,
        seat,
        &origin,
        city,
        &selections,
        CostTrigger::NoModifiers,
    )?
    .total;
    let mut after = p.resources.clone() + total.clone();
    let waste = after.apply_resource_limit(&p.resource_limit);
    let action = Action::Playing(PlayingAction::Collect(Collect::new(
        city,
        selections,
        PlayingActionType::Collect,
    )));
    Ok(
        json!({"action":action,"total":total,"waste":waste,"after":after,"moodWillDecrease":city_data.is_activated()}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{game::GameOptions, game_api, resource_pile::ResourcePile};
    fn game() -> Game {
        game_api::init(2, "web-view-test".into(), GameOptions::default())
    }
    #[test]
    fn spectators_and_waiting_players_have_no_actions() {
        let game = game();
        assert_eq!(view(&game, None)["canPlay"], false);
        assert_eq!(view(&game, Some(99))["cities"], json!([]));
        let waiting = view(&game, Some(1 - game.active_player()));
        assert_eq!(waiting["canPlay"], false);
        assert!(
            waiting["advances"]
                .as_array()
                .unwrap()
                .iter()
                .all(|a| a["action"].is_null())
        );
    }
    #[test]
    fn collect_preview_uses_rules_and_does_not_mutate_state() {
        let game = game();
        let before = serde_json::to_string(&game.cloned_data()).unwrap();
        let seat = game.active_player();
        let city = game.player(seat).cities[0].position;
        let preview = collect_preview(
            &game,
            seat,
            city,
            vec![PositionCollection::new(city, ResourcePile::food(1))],
        )
        .unwrap();
        let action: Action = serde_json::from_value(preview["action"].clone()).unwrap();
        assert!(crate::action::try_execute_action(game.clone(), action, seat).is_ok());
        assert_eq!(before, serde_json::to_string(&game.cloned_data()).unwrap());
        assert!(collect_preview(&game, 1 - seat, city, vec![]).is_err());
        assert!(
            collect_preview(
                &game,
                seat,
                city,
                vec![PositionCollection::new(city, ResourcePile::gold(7))]
            )
            .is_err()
        );
    }
}
