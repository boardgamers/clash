use crate::action::Action;
use crate::city::MoodState;
use crate::content::advances;
use crate::content::custom_actions::{CustomAction, CustomActionInfo, CustomActionType};
use crate::content::persistent_events::{
    EventResponse, PersistentEventRequest, PersistentEventState, SelectedStructure,
};
use crate::game::Game;
use crate::playing_actions::{Collect, IncreaseHappiness, PlayingAction, PlayingActionType};
use crate::position::Position;
use itertools::{Either, Itertools};

///
/// Returns a list of available actions for the current player.
///
/// Some simplifications are made to help AI implementations:
/// - custom actions are preferred over basic actions if available.
/// - always pay default payment
/// - collect and select as much as possible (which is not always the best choice,
///   e.g. selecting to sacrifice a unit for an incident)
/// - move actions are not returned at all - this required special handling
#[must_use]
pub fn get_available_actions(game: &Game) -> Vec<Action> {
    if let Some(event) = game.events.last() {
        responses(game, event)
    } else {
        let actions = base_actions(game);
        if actions.is_empty() {
            return vec![Action::Playing(PlayingAction::EndTurn)];
        }
        actions
    }
}

#[must_use]
fn base_actions(game: &Game) -> Vec<Action> {
    let p = game.player(game.current_player_index);

    let mut actions: Vec<Action> = vec![];

    // Advance
    for a in advances::get_all() {
        if p.can_advance(&a) {
            actions.push(Action::Playing(PlayingAction::Advance {
                advance: a.name.clone(),
                payment: p.advance_cost(&a, None).cost.default,
            }));
        }
    }

    // FoundCity
    for u in &p.units {
        if u.can_found_city(game) {
            actions.push(Action::Playing(PlayingAction::FoundCity { settler: u.id }));
        }
    }

    // Construct,
    // Collect,
    // Recruit,

    // MoveUnits -> special handling

    // IncreaseHappiness
    let happiness = available_happiness_actions(game, p.index);
    if !happiness.is_empty() {
        let action = happiness_action(
            &prefer_custom_action(happiness),
            calculate_increase_happiness(),
        );
        actions.push(action);
    }

    // InfluenceCultureAttempt,
    // available_influence_actions(game, p.index)
    
    // ActionCard(u8),
    // WonderCard(String),
    // Custom(CustomActionInfo),

    actions
}

fn calculate_increase_happiness() -> IncreaseHappiness {
    todo!()
}

fn prefer_custom_action(actions: Vec<PlayingActionType>) -> PlayingActionType {
    let (action, custom) = base_and_custom_action(actions);
    action.unwrap_or_else(|| {
        PlayingActionType::Custom(custom.expect("custom action should be present").info())
    })
}

#[must_use]
pub fn base_and_custom_action(
    actions: Vec<PlayingActionType>,
) -> (Option<PlayingActionType>, Option<CustomActionType>) {
    let (mut custom, mut action): (Vec<_>, Vec<_>) = actions.into_iter().partition_map(|a| {
        if let PlayingActionType::Custom(c) = a {
            Either::Left(c.custom_action_type.clone())
        } else {
            Either::Right(a.clone())
        }
    });
    (action.pop(), custom.pop())
}

#[must_use]
fn responses(game: &Game, event: &PersistentEventState) -> Vec<Action> {
    let request = &event.player.handler.as_ref().expect("handler").request;
    match request {
        PersistentEventRequest::Payment(p) => {
            // todo how to model payment options?
            vec![]
        }
        PersistentEventRequest::ResourceReward(_) => {
            // todo
            vec![]
        }
        PersistentEventRequest::SelectAdvance(a) => a
            .choices
            .iter()
            .map(|c| Action::Response(EventResponse::SelectAdvance(c.clone())))
            .collect(),
        PersistentEventRequest::SelectPlayer(p) => p
            .choices
            .iter()
            .map(|c| Action::Response(EventResponse::SelectPlayer(c.clone())))
            .collect(),
        PersistentEventRequest::SelectPositions(_) => {
            // todo
            vec![]
        }
        PersistentEventRequest::SelectUnitType(t) => t
            .choices
            .iter()
            .map(|c| Action::Response(EventResponse::SelectUnitType(c.clone())))
            .collect(),
        PersistentEventRequest::SelectUnits(t) => {
            // all combinations of units
            // todo
            vec![]
        }
        PersistentEventRequest::SelectStructures(_) => {
            // todo call validate?
            vec![]
        }
        PersistentEventRequest::SelectHandCards(_) => {
            // todo call validate_card_selection
            vec![]
        }
        PersistentEventRequest::BoolRequest(_) => vec![
            Action::Response(EventResponse::Bool(false)),
            Action::Response(EventResponse::Bool(true)),
        ],
        PersistentEventRequest::ChangeGovernment(c) => {
            // todo need to select all combinations of advances
            vec![]
        }
        PersistentEventRequest::ExploreResolution => {
            vec![
                Action::Response(EventResponse::ExploreResolution(0)),
                Action::Response(EventResponse::ExploreResolution(3)),
            ]
        }
    }
}

#[must_use]
pub fn available_collect_actions_for_city(
    game: &Game,
    player: usize,
    position: Position,
) -> Vec<PlayingActionType> {
    if game.player(player).get_city(position).can_activate() {
        available_collect_actions(game, player)
    } else {
        vec![]
    }
}

#[must_use]
pub fn available_collect_actions(game: &Game, player: usize) -> Vec<PlayingActionType> {
    base_or_custom_available(
        game,
        player,
        PlayingActionType::Collect,
        &CustomActionType::FreeEconomyCollect,
    )
}

#[must_use]
pub fn collect_action(action: &PlayingActionType, collect: Collect) -> Action {
    match action {
        PlayingActionType::Collect => Action::Playing(PlayingAction::Collect(collect)),
        PlayingActionType::Custom(c)
            if c.custom_action_type == CustomActionType::FreeEconomyCollect =>
        {
            Action::Playing(PlayingAction::Custom(CustomAction::FreeEconomyCollect(
                collect,
            )))
        }
        _ => panic!("illegal type {action:?}"),
    }
}

#[must_use]
pub fn available_happiness_actions_for_city(
    game: &Game,
    player: usize,
    position: Position,
) -> Vec<PlayingActionType> {
    let city = game.player(player).get_city(position);
    if city.can_activate() && city.mood_state != MoodState::Happy {
        available_happiness_actions(game, player)
    } else {
        vec![]
    }
}

#[must_use]
pub fn available_happiness_actions(game: &Game, player: usize) -> Vec<PlayingActionType> {
    base_or_custom_available(
        game,
        player,
        PlayingActionType::IncreaseHappiness,
        &CustomActionType::VotingIncreaseHappiness,
    )
}

#[must_use]
pub fn happiness_action(
    action: &PlayingActionType,
    include_happiness: IncreaseHappiness,
) -> Action {
    match action {
        PlayingActionType::IncreaseHappiness => {
            Action::Playing(PlayingAction::IncreaseHappiness(include_happiness))
        }
        PlayingActionType::Custom(c)
            if c.custom_action_type == CustomActionType::VotingIncreaseHappiness =>
        {
            Action::Playing(PlayingAction::Custom(
                CustomAction::VotingIncreaseHappiness(include_happiness),
            ))
        }
        _ => panic!("illegal type {action:?}"),
    }
}

#[must_use]
pub fn available_influence_actions(game: &Game, player: usize) -> Vec<PlayingActionType> {
    base_or_custom_available(
        game,
        player,
        PlayingActionType::InfluenceCultureAttempt,
        &CustomActionType::ArtsInfluenceCultureAttempt,
    )
}

#[must_use]
pub fn influence_action(action: &PlayingActionType, target: SelectedStructure) -> Action {
    match action {
        PlayingActionType::InfluenceCultureAttempt => {
            Action::Playing(PlayingAction::InfluenceCultureAttempt(target))
        }
        PlayingActionType::Custom(c)
            if c.custom_action_type == CustomActionType::ArtsInfluenceCultureAttempt =>
        {
            Action::Playing(PlayingAction::Custom(
                CustomAction::ArtsInfluenceCultureAttempt(target),
            ))
        }
        _ => panic!("illegal type {action:?}"),
    }
}

#[must_use]
pub fn base_or_custom_available(
    game: &Game,
    player: usize,
    action: PlayingActionType,
    custom: &CustomActionType,
) -> Vec<PlayingActionType> {
    vec![action, custom.playing_action()]
        .into_iter()
        .filter_map(|a| a.is_available(game, player).map(|_| a).ok())
        .collect()
}

#[must_use]
pub fn can_play_action(game: &Game, player: usize, action: &PlayingActionType) -> bool {
    action.is_available(game, player).is_ok()
}
