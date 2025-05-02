use crate::ability_initializer::{
    AbilityInitializerBuilder, AbilityInitializerSetup, AbilityListeners,
};
use crate::cache::Cache;
use crate::card::{HandCard, draw_card_from_pile};
use crate::content::builtin::Builtin;
use crate::content::persistent_events::{HandCardsRequest, PersistentEventType};
use crate::events::EventOrigin;
use crate::game::Game;
use crate::log::current_action_log_item;
use crate::player::Player;
use crate::utils::remove_element_by;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

type StatusPhaseCheck = Arc<dyn Fn(&Game, &Player) -> bool + Sync + Send>;

type StatusPhaseUpdate = Arc<dyn Fn(&mut Game, usize) + Sync + Send>;

#[derive(Clone)]
pub struct Objective {
    pub name: String,
    pub description: String,
    pub(crate) listeners: AbilityListeners,
    pub(crate) status_phase_check: Option<StatusPhaseCheck>,
    pub(crate) status_phase_update: Option<StatusPhaseUpdate>,
    pub(crate) contradicting_status_phase_objective: Option<String>,
}

impl Objective {
    #[must_use]
    pub fn builder(name: &str, description: &str) -> ObjectiveBuilder {
        ObjectiveBuilder::new(name, description)
    }
}
#[derive(Clone)]
pub struct ObjectiveCard {
    pub id: u8,
    pub objectives: [Objective; 2],
}

impl ObjectiveCard {
    #[must_use]
    pub fn new(id: u8, first: Objective, second: Objective) -> Self {
        Self {
            id,
            objectives: [first, second],
        }
    }

    #[must_use]
    fn has_objective(&self, got: &[String]) -> bool {
        self.objectives.iter().any(|o| got.contains(&o.name))
    }

    #[must_use]
    pub fn name(&self) -> String {
        format!("{}/{}", self.objectives[0].name, self.objectives[1].name)
    }
}

pub struct ObjectiveBuilder {
    name: String,
    description: String,
    status_phase_check: Option<StatusPhaseCheck>,
    status_phase_update: Option<StatusPhaseUpdate>,
    contradicting_status_phase_objective: Option<String>,
    builder: AbilityInitializerBuilder,
}

impl ObjectiveBuilder {
    #[must_use]
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            status_phase_check: None,
            status_phase_update: None,
            contradicting_status_phase_objective: None,
            builder: AbilityInitializerBuilder::new(),
        }
    }

    #[must_use]
    pub fn status_phase_check<F>(mut self, f: F) -> Self
    where
        F: Fn(&Game, &Player) -> bool + 'static + Sync + Send,
    {
        self.status_phase_check = Some(Arc::new(f));
        self
    }

    #[must_use]
    pub fn status_phase_update<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static + Sync + Send,
    {
        self.status_phase_update = Some(Arc::new(f));
        self
    }

    #[must_use]
    pub fn contradicting_status_phase_objective(mut self, name: &str) -> Self {
        self.contradicting_status_phase_objective = Some(name.to_string());
        self
    }

    #[must_use]
    pub fn build(self) -> Objective {
        Objective {
            name: self.name,
            description: self.description,
            listeners: self.builder.build(),
            status_phase_check: self.status_phase_check,
            status_phase_update: self.status_phase_update,
            contradicting_status_phase_objective: self.contradicting_status_phase_objective,
        }
    }
}

impl AbilityInitializerSetup for ObjectiveBuilder {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder {
        &mut self.builder
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::Objective(self.name.clone())
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct SelectObjectivesInfo {
    pub(crate) objective_opportunities: Vec<String>,
    pub(crate) cards: Vec<HandCard>,
}

impl SelectObjectivesInfo {
    #[must_use]
    pub(crate) fn new(objective_opportunities: Vec<String>, cards: Vec<HandCard>) -> Self {
        Self {
            objective_opportunities,
            cards,
        }
    }

    pub(crate) fn strip_secret(&mut self) {
        self.cards.clear();
        self.objective_opportunities.clear();
    }
}

pub(crate) fn objective_is_ready(player: &mut Player, name: &str) {
    let o = &mut player.objective_opportunities;
    o.push(name.to_string());
    o.sort();
    o.dedup(); // can't fulfill 2 objectives with the same name at once
}

pub(crate) fn present_instant_objective_cards(game: &mut Game) {
    let Some(player) = game
        .players
        .iter()
        .find_map(|p| (!p.objective_opportunities.is_empty()).then_some(p.index))
    else {
        return;
    };

    let opportunities = std::mem::take(&mut game.player_mut(player).objective_opportunities);

    present_objective_cards(game, player, opportunities);
}

pub(crate) fn present_objective_cards(
    game: &mut Game,
    player: usize,
    mut opportunities: Vec<String>,
) {
    opportunities.sort();
    opportunities.dedup(); // can't use 2 objectives with the same name at once

    let cards = game
        .player(player)
        .objective_cards
        .iter()
        .filter_map(|&card_id| {
            let card = game.cache.get_objective_card(card_id);
            card.has_objective(&opportunities)
                .then_some(HandCard::ObjectiveCard(card.id))
        })
        .collect_vec();
    on_objective_cards(
        game,
        player,
        SelectObjectivesInfo::new(opportunities, cards),
    );
}

pub(crate) fn on_objective_cards(game: &mut Game, player_index: usize, info: SelectObjectivesInfo) {
    let _ = game.trigger_persistent_event(
        &[player_index],
        |events| &mut events.select_objective_cards,
        info,
        PersistentEventType::SelectObjectives,
    );
}

pub(crate) fn select_objectives() -> Builtin {
    Builtin::builder(
        "Select Objective Cards to Complete",
        "Select which Objective Cards to use \
        (because the requirements are now met)",
    )
    .add_hand_card_request(
        |e| &mut e.select_objective_cards,
        0,
        |_game, _player_index, i| {
            let cards = &i.cards;
            Some(HandCardsRequest::new(
                cards.clone(),
                0..=cards.len() as u8,
                "Select cards to complete",
            ))
        },
        |game, s, cards| {
            let p = s.player_index;
            for (card, objective) in
                match_objective_cards(&s.choice, &cards.objective_opportunities, game)
                    .expect("invalid card selection")
            {
                complete_objective_card(game, p, card, objective);
            }
        },
    )
    .build()
}

pub(crate) fn status_phase_completable(game: &Game, player: &Player, id: u8) -> Vec<String> {
    game.cache
        .get_objective_card(id)
        .objectives
        .iter()
        .filter_map(|objective| {
            objective
                .status_phase_check
                .as_ref()
                .is_some_and(|s| s(game, player))
                .then_some(objective.name.clone())
        })
        .collect()
}

pub(crate) fn complete_objective_card(game: &mut Game, player: usize, id: u8, objective: String) {
    game.add_info_log_item(&format!(
        "{} completed objective {objective}",
        game.player_name(player),
    ));
    let card = game.cache.get_objective_card(id);
    if let Some(s) = card
        .objectives
        .iter()
        .find(|o| o.name == objective)
        .and_then(|o| o.status_phase_update.clone())
    {
        s(game, player);
    }

    discard_objective_card(game, player, id);
    game.player_mut(player)
        .completed_objectives
        .push(objective.clone());
    let o = &mut current_action_log_item(game).completed_objectives;
    o.push(objective);
    o.dedup(); // in redo we add it again
}

pub(crate) fn match_objective_cards(
    hand_cards: &[HandCard],
    opportunities: &[String],
    game: &Game,
) -> Result<Vec<(u8, String)>, String> {
    if hand_cards.is_empty() {
        // is checked by needed range
        return Ok(Vec::new());
    }

    let mut cards = vec![];

    for card in hand_cards {
        match card {
            HandCard::ObjectiveCard(id) => {
                cards.push(game.cache.get_objective_card(*id));
            }
            _ => return Err(format!("Invalid hand card: {card:?}"))?,
        }
    }

    for c in &cards {
        for o in &c.objectives {
            if let Some(contradict) = &o.contradicting_status_phase_objective {
                if cards
                    .iter()
                    .any(|c2| c2.objectives.iter().any(|o2| &o2.name == contradict))
                {
                    return Err(format!(
                        "Cannot select {} and {} at the same time",
                        c.objectives[0].name, contradict
                    ));
                }
            }
        }
    }

    combinations(&cards, opportunities, &game.cache)
        .into_iter()
        .find(|v| {
            v.iter().zip(hand_cards).all(|((id, _), card)| match card {
                HandCard::ObjectiveCard(c) => c == id,
                _ => false,
            })
        })
        .ok_or("Invalid selection of objective cards".to_string())
}

fn combinations(
    cards: &[&ObjectiveCard],
    opportunities: &[String],
    cache: &Cache,
) -> Vec<Vec<(u8, String)>> {
    let Some((first, rest)) = cards.split_first() else {
        return vec![];
    };

    let first = first
        .objectives
        .iter()
        .filter_map(|o| {
            opportunities
                .contains(&o.name)
                .then_some((cards[0].id, o.name.clone()))
        })
        .collect_vec();
    let rest_combinations = combinations(rest, opportunities, cache);
    if rest_combinations.is_empty() {
        return filter_duplicated_objectives(vec![first], cache);
    }

    let result = rest_combinations
        .iter()
        .flat_map(|rest_objectives| {
            first
                .iter()
                .map(|first_objective| {
                    let name = &first_objective.1;
                    let mut r = rest_objectives.clone();
                    if !r.iter().any(|(_id, n)| name == n) {
                        r.insert(0, first_objective.clone());
                    }
                    r
                })
                .collect_vec()
        })
        .collect_vec();
    filter_duplicated_objectives(result, cache)
}

fn filter_duplicated_objectives(
    o: Vec<Vec<(u8, String)>>,
    cache: &Cache,
) -> Vec<Vec<(u8, String)>> {
    o.into_iter()
        .map(|mut v| {
            for (id, group) in &v.clone().iter().chunk_by(|(id, _name)| *id) {
                if group.into_iter().count() == 1 {
                    continue;
                }
                let card = cache.get_objective_card(id);
                let remove = card
                    .objectives
                    .iter()
                    .find_map(
                        |o| o.status_phase_update.is_some().then_some(o.name.clone()), // update means we have to pay
                    )
                    .unwrap_or(card.objectives[0].name.clone());

                remove_element_by(&mut v, |(id2, name)| id == *id2 && name == &remove)
                    .unwrap_or_else(|| {
                        panic!("should be able to remove objective card {id} with name {remove}")
                    });
            }

            v
        })
        .collect_vec()
}

pub(crate) fn gain_objective_card_from_pile(game: &mut Game, player: usize) {
    if let Some(c) = draw_and_log_objective_card_from_pile(game, player) {
        gain_objective_card(game, player, c);
    }
}

pub(crate) fn draw_and_log_objective_card_from_pile(game: &mut Game, player: usize) -> Option<u8> {
    let card = draw_objective_card_from_pile(game);
    if card.is_some() {
        game.add_info_log_item(&format!(
            "{} gained an objective card from the pile",
            game.player_name(player)
        ));
    }
    card
}

fn draw_objective_card_from_pile(game: &mut Game) -> Option<u8> {
    draw_card_from_pile(
        game,
        "Objective Card",
        |g| &mut g.objective_cards_left,
        |g| g.cache.get_objective_cards().iter().map(|c| c.id).collect(),
        |p| p.objective_cards.clone(),
    )
}

pub(crate) fn gain_objective_card(game: &mut Game, player_index: usize, objective_card: u8) {
    init_objective_card(game, player_index, objective_card);
    game.players[player_index]
        .objective_cards
        .push(objective_card);
}

pub(crate) fn init_objective_card(game: &mut Game, player_index: usize, id: u8) {
    for o in &game.cache.get_objective_card(id).objectives.clone() {
        o.listeners.init(game, player_index);
    }
}

pub(crate) fn deinit_objective_card(game: &mut Game, player: usize, card: u8) {
    for o in &game.cache.get_objective_card(card).objectives.clone() {
        o.listeners.deinit(game, player);
    }
}

pub(crate) fn discard_objective_card(game: &mut Game, player: usize, card: u8) {
    let card = remove_element_by(&mut game.player_mut(player).objective_cards, |&id| {
        id == card
    })
    .unwrap_or_else(|| panic!("should be able to discard objective card {card}"));
    deinit_objective_card(game, player, card);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::Cache;

    #[test]
    fn test_combinations_single_card() {
        // avoid ore supplies - because that costs ore to complete
        let cache = Cache::new();
        let o1 = cache.get_objective_card(12);
        let cards = vec![o1];

        let opportunities = vec!["Ore Supplies".to_string(), "Large Army".to_string()];

        let mut got = combinations(&cards, &opportunities, &cache);
        got.sort();
        assert_eq!(got, vec![vec![(12, "Large Army".to_string())]]);
    }

    #[test]
    fn test_combinations() {
        // ObjectiveCard::new(23, seafarers(), aggressor()),
        // ObjectiveCard::new(25, government(), aggressor()),

        let cache = Cache::new();
        let o1 = cache.get_objective_card(23);
        let o2 = cache.get_objective_card(25);

        let cards = vec![o1, o2];

        let opportunities = vec!["Seafarers".to_string(), "Aggressor".to_string()];

        let mut got = combinations(&cards, &opportunities, &cache);
        got.sort();
        assert_eq!(
            got,
            vec![
                vec![(23, "Seafarers".to_string()), (25, "Aggressor".to_string()),],
                vec![(25, "Aggressor".to_string())],
            ]
        );
    }
}
