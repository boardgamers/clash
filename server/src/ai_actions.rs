use crate::action::Action;
use crate::available_actions::{
    available_collect_actions, available_happiness_actions, available_influence_actions,
    base_and_custom_action, collect_action, happiness_action, influence_action,
};
use crate::city::City;
use crate::collect::{
    CollectInfo, PositionCollection, add_collect, get_total_collection,
    possible_resource_collections,
};
use crate::content::advances;
use crate::content::persistent_events::{
    EventResponse, PersistentEventRequest, PersistentEventState, SelectedStructure,
};
use crate::game::Game;
use crate::player::Player;
use crate::playing_actions::{Collect, IncreaseHappiness, PlayingAction, PlayingActionType};
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;

///
/// Returns a list of available actions for the current player.
///
/// Some simplifications are made to help AI implementations:
/// - custom actions are preferred over basic actions if available.
/// - always pay default payment
/// - collect and select as much as possible (which is not always the best choice,
///   e.g. selecting to sacrifice a unit for an incident)
/// - move actions are not returned at all - this required special handling
/// - never activate a city when it decreases happiness
///
#[must_use]
pub fn get_available_actions(game: &Game) -> Vec<Action> {
    if let Some(event) = game.events.last() {
        responses(event)
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
    let collect = available_collect_actions(game, p.index);
    if !collect.is_empty() {
        let action_type = prefer_custom_action(collect);

        for c in collections(game, p) {
            actions.push(collect_action(&action_type, c));
        }
    }

    // Recruit,

    // MoveUnits -> special handling

    // IncreaseHappiness
    let happiness = available_happiness_actions(game, p.index);
    if !happiness.is_empty() {
        let action_type = prefer_custom_action(happiness);

        for h in calculate_increase_happiness() {
            actions.push(happiness_action(&action_type, h));
        }
    }

    // InfluenceCultureAttempt,
    let influence = available_influence_actions(game, p.index);
    if !influence.is_empty() {
        let action_type = prefer_custom_action(influence);

        for i in calculate_influence() {
            actions.push(influence_action(&action_type, i));
        }
    }

    // available_influence_actions(game, p.index)

    // ActionCard(u8),
    // WonderCard(String),
    // Custom(CustomActionInfo),

    actions
}

fn calculate_increase_happiness() -> Vec<IncreaseHappiness> {
    //todo
    vec![]
}

fn prefer_custom_action(actions: Vec<PlayingActionType>) -> PlayingActionType {
    let (action, custom) = base_and_custom_action(actions);
    action.unwrap_or_else(|| {
        PlayingActionType::Custom(custom.expect("custom action should be present").info())
    })
}

#[allow(clippy::match_same_arms)]
#[must_use]
fn responses(event: &PersistentEventState) -> Vec<Action> {
    let request = &event.player.handler.as_ref().expect("handler").request;
    match request {
        PersistentEventRequest::Payment(_p) => {
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
            .map(|c| Action::Response(EventResponse::SelectPlayer(*c)))
            .collect(),
        PersistentEventRequest::SelectPositions(_) => {
            // todo
            vec![]
        }
        PersistentEventRequest::SelectUnitType(t) => t
            .choices
            .iter()
            .map(|c| Action::Response(EventResponse::SelectUnitType(*c)))
            .collect(),
        PersistentEventRequest::SelectUnits(_t) => {
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
        PersistentEventRequest::ChangeGovernment(_c) => {
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
pub fn collections(game: &Game, player: &Player) -> Vec<Collect> {
    non_activated_cities(player)
        .iter()
        .map(|city| city_collection(game, player, city))
        .collect()
}

#[must_use]
pub fn city_collection(game: &Game, player: &Player, city: &City) -> Collect {
    let mut c: Vec<PositionCollection> = vec![];

    loop {
        let info = possible_resource_collections(game, city.position, player.index, &c, &c);

        let Some((pos, pile)) = pick_resource(&info, &c) else {
            break;
        };
        let new = add_collect(&info, pos, &pile, &c);

        if get_total_collection(game, player.index, city.position, &new).is_err() {
            break;
        }

        c = new;
    }

    Collect {
        city_position: city.position,
        collections: c,
    }
}

fn pick_resource(
    info: &CollectInfo,
    collected: &[PositionCollection],
) -> Option<(Position, ResourcePile)> {
    //todo take what can be stored and is most valuable and has least in store
    info.choices
        .iter()
        // todo check max_per_tile -> then we can collect multiple
        .find(|(pos, _)| !collected.iter().any(|c| c.position == **pos))
        .map(|(pos, c)| {
            let pile = ResourceType::all()
                .iter()
                .find_map(|r| c.iter().find(|p| p.get(r) > 0))
                .expect("no resource type");

            (*pos, pile.clone())
        })
}

fn calculate_influence() -> Vec<SelectedStructure> {
    //todo
    vec![]
}

fn non_activated_cities(player: &Player) -> Vec<&City> {
    player
        .cities
        .iter()
        .filter(|city| !city.is_activated())
        .collect()
}
