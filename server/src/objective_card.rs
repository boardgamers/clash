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
use crate::player::Player;
use crate::utils::remove_element_by;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

type StatusPhaseCheck = Box<dyn Fn(&Game, &Player) -> bool>;

pub struct Objective {
    pub name: String,
    pub description: String,
    pub(crate) listeners: AbilityListeners,
    status_phase_check: Option<StatusPhaseCheck>,
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
    pub fn new(id: u8, objectives: [Objective; 2]) -> Self {
        Self { id, objectives }
    }

    #[must_use]
    fn has_objective(&self, got: &[String]) -> bool {
        self.objectives.iter().any(|o| got.contains(&o.name))
    }
}

// todo is this needed or just store the name?
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub(crate) struct ObjectiveOpportunity {
    pub objective: String,
}

impl ObjectiveOpportunity {
    #[must_use]
    pub fn new(objective: String) -> Self {
        Self { objective }
    }
}

pub struct ObjectiveBuilder {
    name: String,
    description: String,
    status_phase_check: Option<StatusPhaseCheck>,
    builder: AbilityInitializerBuilder,
}

impl ObjectiveBuilder {
    #[must_use]
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            status_phase_check: None,
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
    pub fn build(self) -> Objective {
        Objective {
            name: self.name,
            description: self.description,
            listeners: self.builder.build(),
            status_phase_check: self.status_phase_check,
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
    pub(crate) objective_opportunities: Vec<ObjectiveOpportunity>,
    pub cards: Vec<HandCard>,
}

impl SelectObjectivesInfo {
    #[must_use]
    pub(crate) fn new(
        objective_opportunities: Vec<ObjectiveOpportunity>,
        cards: Vec<HandCard>,
    ) -> Self {
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
    player
        .objective_opportunities
        .push(ObjectiveOpportunity::new(name.to_string()));
}

pub(crate) fn present_objective_opportunities(game: &mut Game) {
    let Some(player) = game
        .players
        .iter()
        .find_map(|p| (!p.objective_opportunities.is_empty()).then_some(p.index))
    else {
        return;
    };

    let opportunities = std::mem::take(&mut game.player_mut(player).objective_opportunities);
    let got = opportunities
        .iter()
        .map(|o| o.objective.clone())
        .collect::<Vec<_>>();

    let cards = game
        .player(player)
        .objective_cards
        .iter()
        .filter_map(|&card_id| {
            let card = get_objective_card(card_id);
            card.has_objective(&got)
                .then_some(HandCard::ObjectiveCard(card.id))
        })
        .collect::<Vec<_>>();
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
        "Select which Objective and Action Cards to use \
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

fn complete_objective_card(game: &mut Game, player: usize, id: u8, objective: String) {
    game.add_info_log_item(&format!(
        "{} completed objective {objective}",
        game.player_name(player),
    ));
    discard_objective_card(game, player, id);
    game.player_mut(player).completed_objectives.push(objective);
}

pub(crate) fn match_objective_cards(
    cards: &[HandCard],
    opportunities: &[ObjectiveOpportunity],
) -> Result<Vec<(u8, String)>, String> {
    let mut res = vec![];

    for card in cards {
        match card {
            HandCard::ObjectiveCard(id) => {
                res.push(get_objective_card(*id));
            }
            _ => return Err(format!("Invalid hand card: {card:?}"))?,
        }
    }

    combinations(&res)
        .into_iter()
        .find(|v| {
            v.iter().all(|(id, _)| res.iter().any(|c| c.id == *id))
                && v.iter()
                    .all(|(_, o)| opportunities.iter().any(|oo| &oo.objective == o))
        })
        .ok_or("combination is invalid".to_string())
}

fn combinations(cards: &[ObjectiveCard]) -> Vec<Vec<(u8, String)>> {
    if cards.is_empty() {
        return vec![vec![]];
    }

    let first = cards[0]
        .objectives
        .iter()
        .map(|o| (cards[0].id, o.name.clone()))
        .collect_vec();
    combinations(&cards[1..])
        .iter()
        .flat_map(|v| {
            first
                .iter()
                .map(|o| {
                    let mut r = vec![o.clone()];
                    r.extend(v.clone());
                    r
                })
                .collect_vec()
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
            [
                Objective::builder("Objective 1", "Description 1").build(),
                Objective::builder("Objective 2", "Description 2").build(),
            ],
        );
        let o2 = ObjectiveCard::new(
            1,
            [
                Objective::builder("Objective 3", "Description 3").build(),
                Objective::builder("Objective 4", "Description 4").build(),
            ],
        );
        let o3 = ObjectiveCard::new(
            2,
            [
                Objective::builder("Objective 5", "Description 5").build(),
                Objective::builder("Objective 6", "Description 6").build(),
            ],
        );
        let cards = vec![o1, o2, o3];

        let mut got = combinations(&cards);
        got.sort();
        assert_eq!(
            got,
            vec![
                vec![
                    (0, "Objective 1".to_string()),
                    (1, "Objective 3".to_string()),
                    (2, "Objective 5".to_string()),
                ],
                vec![
                    (0, "Objective 1".to_string()),
                    (1, "Objective 3".to_string()),
                    (2, "Objective 6".to_string()),
                ],
                vec![
                    (0, "Objective 1".to_string()),
                    (1, "Objective 4".to_string()),
                    (2, "Objective 5".to_string()),
                ],
                vec![
                    (0, "Objective 1".to_string()),
                    (1, "Objective 4".to_string()),
                    (2, "Objective 6".to_string()),
                ],
                vec![
                    (0, "Objective 2".to_string()),
                    (1, "Objective 3".to_string()),
                    (2, "Objective 5".to_string()),
                ],
                vec![
                    (0, "Objective 2".to_string()),
                    (1, "Objective 3".to_string()),
                    (2, "Objective 6".to_string()),
                ],
                vec![
                    (0, "Objective 2".to_string()),
                    (1, "Objective 4".to_string()),
                    (2, "Objective 5".to_string()),
                ],
                vec![
                    (0, "Objective 2".to_string()),
                    (1, "Objective 4".to_string()),
                    (2, "Objective 6".to_string()),
                ],
            ]
        );
    }
}
