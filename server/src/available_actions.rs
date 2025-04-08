use itertools::Either;
use crate::action::Action;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::{
    EventResponse, PersistentEventRequest, PersistentEventState,
};
use crate::game::{Game, GameState};
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
pub fn can_collect(game: &Game, player: usize, position: Position) -> Vec<PlayingActionType> {
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
pub fn can_increase_happiness(game: &Game, player: usize) -> Vec<PlayingActionType> {
    base_or_custom_available(
        game,
        player,
        PlayingActionType::IncreaseHappiness,
        CustomActionType::VotingIncreaseHappiness,
    )
}

#[must_use]
pub fn base_or_custom_available(
    game: &Game,
    player: usize,
    action: PlayingActionType,
    custom: CustomActionType,
) -> Vec<PlayingActionType> {
    // let mut actions: Vec<PlayingActionType> = vec![];
    // let option = action.is_available(game, player).map(|_| action).ok();
    // can_play_action(game, player, action)
    //     || 
    //     (game.state == GameState::Playing && 
    //         custom.is_available(game, player))
    // let action_type = custom.playing_action();
    // 
    // if custom.is_available(game, player) {
    //     actions.push(custom.clone());
    // }
    // actions
    
    vec![action, custom.playing_action()]
        .into_iter()
        .filter_map(|a| a.is_available(game, player).map(|_| a).ok())
        .collect()
}

#[must_use]
pub fn can_play_action(game: &Game, player: usize, action: &PlayingActionType) -> bool {
    action.is_available(game, player).is_ok()
}
