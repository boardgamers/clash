//! Short public outcomes for the platform's last-move chip, independent of viewer state.
use crate::action::Action;
use crate::card::HandCardLocation;
use crate::game::Game;
use crate::log::{ActionLogAction, ActionLogBalance, ActionLogEntry};
use crate::playing_actions::PlayingAction;
use crate::structure::Structure;

pub(crate) fn summary(game: &Game, entry: &ActionLogAction) -> String {
    let mut details = Vec::new();
    for item in &entry.items {
        let text = match &item.entry {
            ActionLogEntry::Move(m) => Some(format!(
                "Moved {} from {} to {}",
                m.units.to_string(Some(game)),
                m.start,
                m.destination
            )),
            ActionLogEntry::Advance(a) => Some(format!(
                "{} {}",
                if item.entry.balance() == Some(&ActionLogBalance::Gain) {
                    "Researched"
                } else {
                    "Lost"
                },
                a.advance.name(game)
            )),
            ActionLogEntry::Structure(s) => {
                let name = match &s.structure {
                    Structure::CityCenter => "city".to_owned(),
                    Structure::Building(b) => b.name().to_owned(),
                    Structure::Wonder(w) => w.name(),
                };
                Some(format!(
                    "{} {name} at {}",
                    if s.balance == ActionLogBalance::Gain {
                        match (&entry.action, &s.structure) {
                            (
                                Action::Playing(PlayingAction::FoundCity { .. }),
                                Structure::CityCenter,
                            ) => "Founded",
                            (
                                Action::Playing(
                                    PlayingAction::Construct(_) | PlayingAction::WonderCard(_),
                                ),
                                _,
                            ) => "Built",
                            _ => "Gained",
                        }
                    } else {
                        "Lost"
                    },
                    s.position
                ))
            }
            ActionLogEntry::Units {
                balance: ActionLogBalance::Loss | ActionLogBalance::Pay,
                ..
            } if matches!(
                entry.action,
                Action::Playing(PlayingAction::FoundCity { .. })
            ) =>
            {
                None
            }
            ActionLogEntry::Units {
                units,
                balance,
                position,
            } => Some(format!(
                "{} {}{}",
                if *balance == ActionLogBalance::Gain {
                    "Gained"
                } else {
                    "Lost"
                },
                units.to_string(Some(game)),
                position.map_or(String::new(), |p| format!(" at {p}"))
            )),
            ActionLogEntry::Resources { resources, balance } if !resources.is_empty() => {
                Some(format!(
                    "{} {resources}",
                    match balance {
                        ActionLogBalance::Gain => "Gained",
                        ActionLogBalance::Pay => "Paid",
                        ActionLogBalance::Loss => "Lost",
                    }
                ))
            }
            ActionLogEntry::MoodChange { city, mood } => Some(format!(
                "{city}: {}",
                match mood {
                    crate::city::MoodState::Happy => "happy",
                    crate::city::MoodState::Neutral => "neutral",
                    crate::city::MoodState::Angry => "angry",
                }
            )),
            // The game-list summary is public even when the caller requests a private slice.
            // Never include identities of drawn, peeked, discarded or face-down cards.
            ActionLogEntry::HandCard { card, to, .. } => match to {
                HandCardLocation::Hand(_) | HandCardLocation::RevealedHand(_) => {
                    Some(format!("Drew {}", card.card_type()))
                }
                HandCardLocation::CompleteObjective(name) => {
                    Some(format!("Completed objective {name}"))
                }
                _ => None,
            },
            ActionLogEntry::Explore { .. } => Some("Explored a region".into()),
            ActionLogEntry::InfluenceCultureAttempt(i) => Some(format!(
                "Cultural influence from {} to {}",
                i.starting_city_position, i.position
            )),
            ActionLogEntry::CombatRoll(r) => Some(format!(
                "Rolled {} combat value, {} hits",
                r.combat_value, r.hits
            )),
            _ => None,
        };
        if let Some(text) = text {
            let text = if item.player == entry.player {
                text
            } else {
                format!(
                    "{}: {text}",
                    game.players
                        .get(item.player)
                        .map_or("Player", |p| p.civilization.name.as_str())
                )
            };
            if !details.contains(&text) {
                details.push(text);
            }
        }
    }
    let outcome = if details.is_empty() {
        fallback(game, entry)
    } else {
        details.join(" · ")
    };
    let civilization = game
        .players
        .get(entry.player)
        .map_or("Player", |player| player.civilization.name.as_str());
    let outcome = format!("{civilization}: {outcome}");
    // Keep the platform chip compact without cutting a Unicode character in half.
    if outcome.chars().count() > 240 {
        format!("{}…", outcome.chars().take(239).collect::<String>())
    } else {
        outcome
    }
}

fn fallback(game: &Game, entry: &ActionLogAction) -> String {
    match &entry.action {
        Action::Playing(action) => match action {
            PlayingAction::EndTurn => "Ended turn".into(),
            PlayingAction::Advance(_) => "Research".into(),
            PlayingAction::FoundCity { .. } => "Found a city".into(),
            PlayingAction::Construct(_) => "Construct a building".into(),
            PlayingAction::Collect(_) => "Collect resources".into(),
            PlayingAction::Recruit(_) => "Recruit units".into(),
            PlayingAction::IncreaseHappiness(_) => "Increase happiness".into(),
            PlayingAction::InfluenceCultureAttempt(_) => "Cultural influence".into(),
            PlayingAction::ActionCard(_) => "Played an action card".into(),
            PlayingAction::WonderCard(_) => "Construct a wonder".into(),
            PlayingAction::Custom(_) => entry
                .origin
                .as_ref()
                .map_or_else(|| "Used an ability".into(), |origin| origin.name(game)),
        },
        Action::Movement(_) => "Moved units".into(),
        Action::ChooseCivilization(name) => format!("Chose {name}"),
        Action::StartTurn => "Started turn".into(),
        Action::Response(_) => "Completed a decision".into(),
        Action::Undo => "Undid a move".into(),
        Action::Redo => "Redid a move".into(),
    }
}
