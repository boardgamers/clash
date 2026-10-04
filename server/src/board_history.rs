//! Public board snapshots for visual playback; never simulation states.
use crate::action::{Action, try_execute_action};
use crate::card::{HandCardLocation, HandCardType};
use crate::city::CityData;
use crate::game::Game;
use crate::game_api::log_length;
use crate::log::{ActionLogAction, ActionLogEntry};
use crate::map::Terrain;
use crate::playing_actions::PlayingAction;
use crate::position::Position;
use crate::unit::UnitData;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BoardHistory {
    pub id: String,
    pub frames: Vec<BoardFrame>,
}
impl BoardHistory {
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct BoardFrame {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub combat: Option<BoardCombat>,
    pub cursor: usize,
    pub actor: Option<usize>,
    pub ended_turn: bool,
    pub title: String,
    pub age: u32,
    pub round: u32,
    pub tiles: Vec<(Position, Terrain)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<BoardEffect>,
    pub players: Vec<BoardPlayer>,
}
// Only public battlefield locations; never pending tactics or hidden event data.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct BoardCombat {
    pub round: u32,
    pub attacker: BoardCombatSide,
    pub defender: BoardCombatSide,
}
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct BoardCombatSide {
    pub player: usize,
    pub position: Position,
}
fn board_combat(game: &Game) -> Option<BoardCombat> {
    use crate::content::persistent_events::PersistentEventType;
    for event in game.events.iter().rev() {
        let stats = match &event.event_type {
            PersistentEventType::CombatStart(c) => &c.stats,
            PersistentEventType::CombatRoundStart(r) => &r.combat.stats,
            PersistentEventType::CombatRoundEnd(r) => &r.combat.stats,
            PersistentEventType::CombatEnd(_) => return None,
            _ => continue,
        };
        return Some(BoardCombat {
            round: stats.round,
            attacker: BoardCombatSide {
                player: stats.attacker.player,
                position: stats.attacker.position,
            },
            defender: BoardCombatSide {
                player: stats.defender.player,
                position: stats.defender.position,
            },
        });
    }
    None
}
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct BoardEffect {
    pub player: usize,
    pub kind: String,
    pub label: String,
}
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct BoardPlayer {
    pub id: usize,
    pub civilization: String,
    pub cities: Vec<CityData>,
    pub units: Vec<UnitData>,
}
fn snapshot(game: &Game, actor: Option<usize>, title: String, ended_turn: bool) -> BoardFrame {
    BoardFrame {
        combat: board_combat(game),
        effects: vec![],
        cursor: log_length(game),
        actor,
        title,
        ended_turn,
        age: game.age,
        round: game.round,
        tiles: game
            .map
            .tiles
            .iter()
            .map(|(p, t)| (*p, t.clone()))
            .sorted_by_key(|(p, _)| *p)
            .collect(),
        players: game
            .players
            .iter()
            .map(|p| BoardPlayer {
                id: p.index,
                civilization: p.civilization.name.clone(),
                cities: p.cities.iter().map(|c| c.cloned_data()).collect(),
                units: p
                    .units
                    .iter()
                    .filter(|u| !u.is_transported())
                    .map(|u| u.data(p))
                    .collect(),
            })
            .collect(),
    }
}
fn title(action: &Action) -> String {
    let name = match action {
        Action::Movement(_) => "Move".to_owned(),
        Action::Response(_) => "Resolve choice".to_owned(),
        Action::ChooseCivilization(_) => "Choose civilization".to_owned(),
        Action::Playing(a) => {
            let value = serde_json::to_value(a).expect("action serializes");
            value
                .as_str()
                .map(str::to_owned)
                .or_else(|| value.as_object()?.keys().next().cloned())
                .unwrap_or_default()
        }
        _ => "Action".to_owned(),
    };
    match name.as_str() {
        "Advance" => "Research",
        "Construct" => "Build",
        "EndTurn" => "End turn",
        "FoundCity" => "Found city",
        "IncreaseHappiness" => "Increase happiness",
        "ActionCard" => "Play action card",
        "WonderCard" => "Build wonder",
        "InfluenceCultureAttempt" => "Cultural influence",
        "Custom" => "Civilization ability",
        _ => &name,
    }
    .to_owned()
}

pub fn execute(game: Game, action: Action, player: usize) -> Result<Game, String> {
    let undo = matches!(action, Action::Undo);
    let mut game = execute_one(game, action, player)?;
    // Save automatic responses as ordinary actions. Replay then reproduces them
    // without needing past settings; undo never immediately replays a skipped choice.
    while !undo && game.context == crate::game::GameContext::Play && auto_keep_cities(&game) {
        let player = game.active_player();
        game = execute_one(
            game,
            Action::Response(
                crate::content::persistent_events::EventResponse::SelectPositions(vec![]),
            ),
            player,
        )?;
    }
    Ok(game)
}

fn auto_keep_cities(game: &Game) -> bool {
    use crate::content::persistent_events::{PersistentEventRequest, PersistentEventType};
    let Some(event) = game.events.last() else {
        return false;
    };
    matches!(
        event.event_type,
        PersistentEventType::StatusPhase(crate::status_phase::StatusPhaseState::RazeSize1City)
    ) && game.player(game.active_player()).settings.skip_raze_city == Some(true)
        && event.player.handler.as_ref().is_some_and(|handler| {
            matches!(&handler.origin, crate::events::EventOrigin::Ability(name) if name == "Raze city")
                && matches!(
                    &handler.request,
                    PersistentEventRequest::SelectPositions(request) if request.needed == (0..=1)
                )
        })
}

fn execute_one(mut game: Game, action: Action, player: usize) -> Result<Game, String> {
    let choosing_civilization = matches!(game.state, crate::game::GameState::ChooseCivilization);
    // Kept outside the undo patch: undo trims playback; it never creates new history.
    let mut history = std::mem::take(&mut game.board_history);
    let before_cursor = log_length(&game);
    let before = snapshot(&game, None, "Earlier position".to_owned(), false);
    if history.id.is_empty() {
        history.id = format!(
            "{:x}",
            Sha256::digest(format!("clash-board-v1:{}", game.seed))
        );
    }
    history.frames.retain(|frame| frame.cursor <= before.cursor);
    if !before.tiles.is_empty()
        && history
            .frames
            .last()
            .is_none_or(|f| f.cursor != before.cursor)
    {
        history.frames.push(before);
    }
    let undo = matches!(action, Action::Undo);
    let recorded = if matches!(action, Action::Redo) {
        crate::log::current_turn_log(&game)
            .actions
            .get(game.log_index)
            .map_or(&action, |a| &a.action)
    } else {
        &action
    };
    let ended_turn = matches!(recorded, Action::Playing(PlayingAction::EndTurn));
    let label = if let Action::Playing(PlayingAction::Custom(custom)) = recorded {
        game.player(player)
            .special_actions
            .get(&crate::content::custom_actions::SpecialAction::Custom(
                custom.action,
            ))
            .map(|info| info.event_origin.name(&game))
            .unwrap_or_else(|| title(recorded))
    } else if matches!(recorded, Action::Response(_)) {
        game.events
            .last()
            .and_then(|event| event.player.handler.as_ref())
            .filter(|handler| {
                matches!(
                    handler.origin,
                    crate::events::EventOrigin::Advance(_)
                        | crate::events::EventOrigin::SpecialAdvance(_)
                        | crate::events::EventOrigin::LeaderAbility(_)
                        | crate::events::EventOrigin::Ability(_)
                )
            })
            .map(|handler| handler.origin.name(&game))
            .unwrap_or_else(|| title(recorded))
    } else {
        title(recorded)
    };
    let mut next = try_execute_action(game, action, player)?;
    // Simultaneous setup belongs to everyone, regardless of who locked in last.
    let mut after = if choosing_civilization {
        snapshot(&next, None, "Game setup".to_owned(), false)
    } else {
        snapshot(&next, Some(player), label, ended_turn)
    };
    if !undo {
        after.effects = effects(&next, before_cursor);
    }
    history.frames.retain(|f| f.cursor <= after.cursor);
    if !undo
        && !after.tiles.is_empty()
        && history
            .frames
            .last()
            .is_none_or(|f| f.cursor != after.cursor)
    {
        history.frames.push(after);
    }
    // Keep several turns while bounding save and download size on long games.
    if history.frames.len() > 192 {
        history.frames.drain(..history.frames.len() - 192);
    }
    next.board_history = history;
    Ok(next)
}

fn effects(game: &Game, from: usize) -> Vec<BoardEffect> {
    let turns: Vec<_> = game
        .log
        .iter()
        .flat_map(|a| a.rounds.iter().flat_map(|r| &r.turns))
        .collect();
    let actions: Vec<&ActionLogAction> = turns
        .iter()
        .enumerate()
        .flat_map(|(i, t)| {
            t.actions.iter().take(if i + 1 == turns.len() {
                game.log_index
            } else {
                t.actions.len()
            })
        })
        .collect();
    actions
        .into_iter()
        .skip(from)
        .flat_map(|a| &a.items)
        .filter_map(|item| {
            let ActionLogEntry::HandCard { card, from, to } = &item.entry else {
                return None;
            };
            if let HandCardLocation::CompleteObjective(name) = to {
                return Some(BoardEffect {
                    player: item.player,
                    kind: "completed".into(),
                    label: name.clone(),
                });
            }
            if !matches!(
                from,
                HandCardLocation::DrawPile
                    | HandCardLocation::DrawPilePeeked(_)
                    | HandCardLocation::Incident
            ) {
                return None;
            }
            let (HandCardLocation::Hand(player) | HandCardLocation::RevealedHand(player)) = to
            else {
                return None;
            };
            let kind = match card.card_type() {
                HandCardType::Action | HandCardType::Public => "action",
                HandCardType::Objective => "objective",
                HandCardType::Wonder => "wonder",
            };
            Some(BoardEffect {
                player: *player,
                kind: kind.into(),
                label: format!("Drew {}", card.card_type()),
            })
        })
        .collect()
}
