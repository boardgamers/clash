use crate::action::Action;
use crate::city::MoodState;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::{
    EventResponse, PersistentEventRequest, PersistentEventState,
};
use crate::game::Game;
use crate::playing_actions::PlayingActionType;
use crate::position::Position;

pub fn get_available_actions(game: &Game) -> Vec<Action> {
    if let Some(event) = game.events.last() {
        responses(game, event)
    } else {
        base_actions(game)
    }
}

fn base_actions(game: &Game) -> Vec<Action> {
    todo!()
}

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
pub fn available_collect_actions(
    game: &Game,
    player: usize,
    position: Position,
) -> Vec<PlayingActionType> {
    // todo should we pull the city from the arg list - since it's the same for all cities?
    if game.player(player).get_city(position).can_activate() {
        base_or_custom_available(
            game,
            player,
            PlayingActionType::Collect,
            CustomActionType::FreeEconomyCollect,
        )
    } else {
        vec![]
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
        CustomActionType::VotingIncreaseHappiness,
    )
}

#[must_use]
pub fn available_influence_actions(game: &Game, player: usize) -> Vec<PlayingActionType> {
    base_or_custom_available(
        game,
        player,
        PlayingActionType::InfluenceCultureAttempt,
        CustomActionType::ArtsInfluenceCultureAttempt,
    )
}

#[must_use]
pub fn base_or_custom_available(
    game: &Game,
    player: usize,
    action: PlayingActionType,
    custom: CustomActionType,
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
