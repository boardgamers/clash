use crate::ability_initializer::{
    AbilityInitializerBuilder, AbilityInitializerSetup, AbilityListeners,
};
use crate::card::{HandCard, draw_card_from_pile};
use crate::content::builtin::Builtin;
use crate::content::objective_cards;
use crate::content::objective_cards::get_objective_card;
use crate::content::persistent_events::{HandCardsRequest, PersistentEventType};
use crate::events::EventOrigin;
use crate::game::Game;
use crate::log::current_player_turn_log_mut;
use crate::player::Player;
use crate::utils::remove_element_by;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

type StatusPhaseCheck = Box<dyn Fn(&Game, &Player) -> bool>;

type StatusPhaseUpdate = Box<dyn Fn(&mut Game, usize)>;

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
        F: Fn(&Game, &Player) -> bool + 'static,
    {
        self.status_phase_check = Some(Box::new(f));
        self
    }

    #[must_use]
    pub fn status_phase_update<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.status_phase_update = Some(Box::new(f));
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
            let card = get_objective_card(card_id);
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
                match_objective_cards(&s.choice, &cards.objective_opportunities)
                    .expect("invalid card selection")
            {
                complete_objective_card(game, p, card, objective);
            }
        },
    )
    .build()
}

pub(crate) fn status_phase_completable(game: &Game, player: &Player, id: u8) -> Vec<String> {
    get_objective_card(id)
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
    let card = get_objective_card(id);
    if let Some(s) = card
        .objectives
        .iter()
        .find(|o| o.name == objective)
        .and_then(|o| o.status_phase_update.as_ref())
    {
        s(game, player);
    }

    discard_objective_card(game, player, id);
    game.player_mut(player)
        .completed_objectives
        .push(objective.clone());
    current_player_turn_log_mut(game)
        .items
        .last_mut()
        .expect("items empty")
        .completed_objectives
        .push(objective);
}

pub(crate) fn match_objective_cards(
    hand_cards: &[HandCard],
    opportunities: &[String],
) -> Result<Vec<(u8, String)>, String> {
    if hand_cards.is_empty() {
        // is checked by needed range
        return Ok(Vec::new());
    }

    let mut cards = vec![];

    for card in hand_cards {
        match card {
            HandCard::ObjectiveCard(id) => {
                cards.push(get_objective_card(*id));
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

    combinations(&cards, opportunities)
        .into_iter()
        .find(|v| {
            v.iter().zip(hand_cards).all(|((id, _), card)| match card {
                HandCard::ObjectiveCard(c) => c == id,
                _ => false,
            })
        })
        .ok_or("Invalid selection of objective cards".to_string())
}

fn combinations(cards: &[ObjectiveCard], opportunities: &[String]) -> Vec<Vec<(u8, String)>> {
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
    let rest_combinations = combinations(rest, opportunities);
    if rest_combinations.is_empty() {
        return vec![first];
    }

    rest_combinations
        .iter()
        .flat_map(|rest_objectives| {
            let vec1 = first
                .iter()
                .map(|first_objective| {
                    let name = &first_objective.1;
                    let mut r = rest_objectives.clone();
                    if !r.iter().any(|(_id, n)| name == n) {
                        r.insert(0, first_objective.clone());
                    }
                    r
                })
                .collect_vec();
            vec1
        })
        .collect_vec()
}

pub(crate) fn gain_objective_card_from_pile(game: &mut Game, player: usize) {
    if let Some(c) = draw_objective_card_from_pile(game) {
        game.add_info_log_item(&format!(
            "{} gained an objective card from the pile",
            game.player_name(player)
        ));
        gain_objective_card(game, player, &c);
    }
}

fn draw_objective_card_from_pile(game: &mut Game) -> Option<ObjectiveCard> {
    draw_card_from_pile(
        game,
        "Objective Card",
        false,
        |g| &mut g.objective_cards_left,
        || objective_cards::get_all().iter().map(|c| c.id).collect(),
        |p| p.objective_cards.clone(),
    )
    .map(get_objective_card)
}

pub(crate) fn gain_objective_card(
    game: &mut Game,
    player_index: usize,
    objective_card: &ObjectiveCard,
) {
    game.players[player_index]
        .objective_cards
        .push(objective_card.id);
}

pub(crate) fn discard_objective_card(game: &mut Game, player: usize, card: u8) {
    let card = remove_element_by(&mut game.player_mut(player).objective_cards, |&id| {
        id == card
    })
    .expect("should be able to discard objective card");
    for o in get_objective_card(card).objectives {
        o.listeners.deinit(game, player);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combinations() {
        let o1 = ObjectiveCard::new(
            0,
            Objective::builder("Objective 1", "Description 1").build(),
            Objective::builder("Objective 2", "Description 2").build(),
        );
        let o2 = ObjectiveCard::new(
            1,
            Objective::builder("Objective 1", "Description 3").build(),
            Objective::builder("Objective 4", "Description 4").build(),
        );
        let o3 = ObjectiveCard::new(
            2,
            Objective::builder("Objective 5", "Description 5").build(),
            Objective::builder("Objective 6", "Description 6").build(),
        );
        let cards = vec![o1, o2, o3];

        let opportunities = vec!["Objective 1".to_string(), "Objective 4".to_string()];

        let mut got = combinations(&cards, &opportunities);
        got.sort();
        assert_eq!(got, vec![
            vec![
                (0, "Objective 1".to_string()),
                (1, "Objective 4".to_string()),
            ],
            vec![(1, "Objective 1".to_string()),],
        ]);
    }
}
