use crate::content::effects::{CollectEffect, ConstructEffect, PermanentEffect};
use crate::game::Game;
use serde_json::{Value, json};

pub(super) fn describe(game: &Game) -> Vec<Value> {
    game.permanent_effects.iter().map(|effect| {
        let (name, players): (&str, Vec<usize>) = match effect {
            PermanentEffect::Pestilence => ("Pestilence", vec![]),
            PermanentEffect::TrojanHorse => ("Trojan Horse", vec![]),
            PermanentEffect::SolarEclipse => ("Solar Eclipse", vec![]),
            PermanentEffect::Construct(ConstructEffect::CityDevelopment) => ("City Development", vec![game.active_player()]),
            PermanentEffect::Construct(ConstructEffect::GreatEngineer) => ("Great Engineer", vec![game.active_player()]),
            PermanentEffect::Collect(CollectEffect::ProductionFocus) => ("Production Focus", vec![game.active_player()]),
            PermanentEffect::Collect(CollectEffect::MassProduction) => ("Mass Production", vec![game.active_player()]),
            PermanentEffect::CulturalTakeover => ("Cultural Takeover", vec![game.active_player()]),
            PermanentEffect::DiplomaticRelations(r) => ("Great Diplomat", vec![r.active_player, r.passive_player]),
            PermanentEffect::Negotiations(n) => ("Negotiations", vec![n.relations.active_player, n.relations.passive_player]),
            PermanentEffect::RevolutionLoseAction(p) => ("Civil War", vec![*p]),
            PermanentEffect::AssassinationLoseAction(p) => ("Assassination", vec![*p]),
            PermanentEffect::Anarchy(a) => ("Anarchy", vec![a.player]),
            PermanentEffect::GreatSeer(s) => ("Great Seer", s.assigned_objectives.iter().map(|o| o.player).collect()),
            PermanentEffect::PublicWonderCard(_) => ("Public wonder", vec![]),
        };
        let source = match effect {
            PermanentEffect::Pestilence => event_source(game, 1),
            PermanentEffect::TrojanHorse => event_source(game, 42),
            PermanentEffect::SolarEclipse => event_source(game, 41),
            PermanentEffect::DiplomaticRelations(_) => event_source(game, 57),
            PermanentEffect::RevolutionLoseAction(_) => event_source(game, 38),
            PermanentEffect::Anarchy(_) => event_source(game, 44),
            PermanentEffect::PublicWonderCard(_) => event_source(game, 40),
            _ => action_source(game, name, players.first().copied()),
        };
        let incident = match effect {
            PermanentEffect::Pestilence => Some(1),
            PermanentEffect::TrojanHorse => Some(42),
            PermanentEffect::SolarEclipse => Some(41),
            PermanentEffect::Construct(ConstructEffect::GreatEngineer) => Some(26),
            _ => None,
        };
        // Base effects and initial event decisions have already resolved.
        let mut rules = incident.map_or_else(|| effect.description(game), |id| game.cache.get_incident(id).description(game).into_iter().rev().take(1).collect());
        if rules.first().is_some_and(|line| line == name) { rules.remove(0); }
        match effect {
            PermanentEffect::Pestilence => rules = vec!["You cannot construct buildings or wonders until you research Sanitation.".into()],
            PermanentEffect::Construct(ConstructEffect::GreatEngineer) => rules = vec!["Construct a building in one of your cities at its normal resource cost, without spending an additional action or activating the city.".into()],
            PermanentEffect::Collect(CollectEffect::MassProduction) => rules = vec![game.cache.get_civil_card(29).description.clone()],
            _ => {}
        }
        if let PermanentEffect::PublicWonderCard(wonder) = effect {
            let info = wonder.info(game);
            rules.extend([
                info.description.clone(),
                format!("Requires {}. Base construction cost: {}.", info.required_advance.name(game), info.cost.default_payment()),
                format!("{} VP for building · {} VP for owning", info.built_victory_points, info.owned_victory_points),
            ]);
        }
        // Objective assignments remain private even when this view is built from a full game.
        if let PermanentEffect::GreatSeer(_) = effect {
            rules = vec!["The next time each listed player draws an objective from the pile, they receive their designated card instead. The assigned cards remain private.".into()];
        }
        if matches!(effect, PermanentEffect::DiplomaticRelations(_)) {
            rules.push("The agreement ends when either player attacks the other. The card's owner may also end it as a regular action.".into());
        }
        let scope = if players.is_empty() { "All players".to_string() }
            else { players.iter().map(|p| game.player_name(*p)).collect::<Vec<_>>().join(" · ") };
        json!({"name": name, "scope": scope, "players": players, "rules": rules,"source":source})
    }).collect()
}

fn action_source(game: &Game, name: &str, player: Option<usize>) -> Value {
    let turns: Vec<_> = game
        .log
        .iter()
        .flat_map(|age| &age.rounds)
        .flat_map(|round| &round.turns)
        .collect();
    let id = turns.iter().enumerate().rev().find_map(|(index, turn)| {
        let end = if index + 1 == turns.len() {
            game.log_index.min(turn.actions.len())
        } else {
            turn.actions.len()
        };
        turn.actions[..end].iter().rev().find_map(|entry| {
            if player.is_some_and(|p| p != entry.player) {
                return None;
            }
            if let crate::action::Action::Playing(
                crate::playing_actions::PlayingAction::ActionCard(id),
            ) = entry.action
            {
                game.cache
                    .get_action_cards()
                    .iter()
                    .chain(
                        game.cache
                            .get_incidents()
                            .iter()
                            .filter_map(|i| i.action_card.as_ref()),
                    )
                    .find(|card| card.id == id && card.civil_card.name == name)
                    .map(|card| card.id)
            } else {
                None
            }
        })
    });
    json!({"kind":"Action card","name":name,"id":id})
}

fn event_source(game: &Game, id: u8) -> Value {
    json!({"kind":"Event", "name":game.cache.get_incident(id).name,"id":id})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::incidents::great_diplomat::DiplomaticRelations;
    #[test]
    fn shared_effects_show_scope_and_rules() {
        let mut game = crate::game_api::init(2, "effects".into(), Default::default());
        game.permanent_effects = vec![
            PermanentEffect::TrojanHorse,
            PermanentEffect::DiplomaticRelations(DiplomaticRelations::new(0, 1)),
        ];
        let effects = describe(&game);
        assert_eq!(effects[0]["scope"], "All players");
        assert_eq!(
            effects[0]["source"],
            json!({"kind":"Event","name":"Trojan Horse","id":42})
        );
        assert_eq!(effects[1]["source"]["id"], 57);
        assert!(effects[0]["rules"].to_string().contains("another player's"));
        assert_eq!(effects[1]["players"], json!([0, 1]));
        assert!(effects[1]["rules"].to_string().contains("2 culture tokens"));
        game.permanent_effects = vec![PermanentEffect::GreatSeer(
            crate::content::effects::GreatSeerEffect {
                player: 0,
                assigned_objectives: vec![crate::content::effects::GreatSeerObjective {
                    player: 1,
                    objective_card: 34,
                }],
            },
        )];
        let seer = describe(&game);
        assert_eq!(seer[0]["players"], json!([1]));
        assert!(!seer[0]["rules"].to_string().contains("Mercantile"));
        assert!(!seer[0]["rules"].to_string().contains("Traders"));
        game.permanent_effects = vec![
            PermanentEffect::Collect(CollectEffect::MassProduction),
            PermanentEffect::PublicWonderCard(crate::wonder::Wonder::Colosseum),
        ];
        let more = describe(&game);
        assert_eq!(more[0]["source"]["kind"], "Action card");
        assert_eq!(more[0]["source"]["name"], "Mass Production");
        assert_eq!(more[1]["source"]["id"], 40);
        let player = game.active_player();
        let entry = crate::log::ActionLogAction::new(
            crate::action::Action::Playing(crate::playing_actions::PlayingAction::ActionCard(30)),
            player,
            None,
            0,
        );
        crate::log::current_turn_log_mut(&mut game)
            .actions
            .push(entry);
        game.log_index = crate::log::current_turn_log(&game).actions.len();
        assert_eq!(describe(&game)[0]["source"]["id"], 30);
        let redo = crate::log::ActionLogAction::new(
            crate::action::Action::Playing(crate::playing_actions::PlayingAction::ActionCard(29)),
            player,
            None,
            0,
        );
        crate::log::current_turn_log_mut(&mut game)
            .actions
            .push(redo);
        assert_eq!(
            describe(&game)[0]["source"]["id"],
            30,
            "ignore undone card plays"
        );
        game.permanent_effects.clear();
        assert!(describe(&game).is_empty());
    }
}
