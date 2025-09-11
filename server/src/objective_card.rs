use crate::ability_initializer::{
    AbilityInitializerBuilder, AbilityInitializerSetup, AbilityListeners,
};
use crate::card::{HandCard, HandCardLocation, draw_card_from_pile, log_card_transfer};
use crate::content::ability::{Ability, AbilityBuilder};
use crate::content::effects::PermanentEffect;
use crate::content::incidents::great_persons::find_great_seer;
use crate::content::persistent_events::{HandCardsRequest, PersistentEventType};
use crate::events::{EventOrigin, EventPlayer};
use crate::game::Game;
use crate::player::Player;
use crate::utils::{remove_element, remove_element_by};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

type StatusPhaseCheck = Arc<dyn Fn(&Game, &Player) -> bool + Sync + Send>;

type StatusPhaseUpdate = Arc<dyn Fn(&mut Game, &EventPlayer) + Sync + Send>;

pub enum ObjectiveType {
    Instant,
    StatusPhase,
}

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

    // high priority means it is checked first
    #[must_use]
    pub fn priority(&self) -> i32 {
        // status_phase_update == we have to pay for this objective
        i32::from(self.status_phase_update.is_none())
    }

    #[must_use]
    pub fn get_type(&self) -> ObjectiveType {
        if self.status_phase_check.is_some() {
            ObjectiveType::StatusPhase
        } else {
            ObjectiveType::Instant
        }
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
    fn has_objective(&self, name: &str) -> bool {
        self.objectives.iter().any(|o| name == o.name)
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
    pub(crate) fn status_phase_check<F>(mut self, f: F) -> Self
    where
        F: Fn(&Game, &Player) -> bool + 'static + Sync + Send,
    {
        self.status_phase_check = Some(Arc::new(f));
        self
    }

    #[must_use]
    pub(crate) fn status_phase_update<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut Game, &EventPlayer) + 'static + Sync + Send,
    {
        self.status_phase_update = Some(Arc::new(f));
        self
    }

    #[must_use]
    pub(crate) fn contradicting_status_phase_objective(mut self, name: &str) -> Self {
        self.contradicting_status_phase_objective = Some(name.to_string());
        self
    }

    #[must_use]
    pub(crate) fn build(self) -> Objective {
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

    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct SelectObjectivesInfo {
    pub(crate) objective_opportunities: Vec<String>,
    pub(crate) shown_objective: Option<String>,
}

impl SelectObjectivesInfo {
    #[must_use]
    pub(crate) fn new(objective_opportunities: Vec<String>) -> Self {
        Self {
            objective_opportunities,
            shown_objective: None,
        }
    }

    pub(crate) fn strip_secret(&mut self) {
        self.shown_objective = None;
        self.objective_opportunities.clear();
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CompletedObjective {
    pub card: u8,
    pub name: String,
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
    opportunities.sort_by_cached_key(|name| {
        let o = game.cache.get_objective(name);
        (-o.priority(), o.name.clone())
    });
    opportunities.dedup(); // can't use 2 objectives with the same name at once
    on_objective_cards(game, player, SelectObjectivesInfo::new(opportunities));
}

pub(crate) fn on_objective_cards(game: &mut Game, player_index: usize, info: SelectObjectivesInfo) {
    let _ = game.trigger_persistent_event(
        &[player_index],
        |events| &mut events.select_objective_cards,
        info,
        PersistentEventType::SelectObjectives,
    );
}

pub(crate) fn select_objectives() -> Ability {
    let mut b = Ability::builder(
        "Select Objective Cards to Complete",
        "Select which Objective Cards to use \
        (because the requirements are now met)",
    );
    // probably less than this at most
    for i in 0..10 {
        b = add_select_objective_card_to_complete(b, i);
    }
    b.build()
}

fn add_select_objective_card_to_complete(b: AbilityBuilder, priority: i32) -> AbilityBuilder {
    b.add_hand_card_request(
        |e| &mut e.select_objective_cards,
        priority,
        |game, player, i| {
            if i.objective_opportunities.is_empty() {
                return None;
            }
            let objective = i.objective_opportunities.remove(0);
            i.shown_objective = Some(objective.clone());

            let cards = player
                .get(game)
                .objective_cards
                .iter()
                .filter_map(|&card_id| {
                    let card = game.cache.get_objective_card(card_id);
                    card.has_objective(&objective)
                        .then_some(HandCard::ObjectiveCard(card.id))
                })
                .collect_vec();

            Some(HandCardsRequest::new(
                cards.clone(),
                0..=1,
                &format!("You may select a card to complete objective: {objective}"),
            ))
        },
        |game, s, i| {
            let card_id = match s.choice.first() {
                None => {
                    s.log(game, "No card selected");
                    return;
                }
                Some(HandCard::ObjectiveCard(card_id)) => card_id,
                _ => {
                    panic!("Expected an objective card to be selected");
                }
            };

            if let Some(contradicting) = game
                .cache
                .get_objective_card(*card_id)
                .objectives
                .iter()
                .find_map(|o| o.contradicting_status_phase_objective.as_ref())
            {
                i.objective_opportunities.retain(|o| o != contradicting);
            }

            complete_objective_card(
                game,
                s.player_index,
                *card_id,
                i.shown_objective
                    .as_ref()
                    .expect("Expected current objective to be set"),
            );
        },
    )
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

pub(crate) fn complete_objective_card(game: &mut Game, player: usize, id: u8, objective: &str) {
    if let Some(s) = game
        .cache
        .get_objective_card(id)
        .objectives
        .iter()
        .find(|o| o.name == objective)
        .and_then(|o| o.status_phase_update.clone())
    {
        s(
            game,
            &EventPlayer::new(player, EventOrigin::Objective(objective.to_string())),
        );
    }

    discard_objective_card(
        game,
        player,
        id,
        &EventOrigin::Ability("Complete Objectives".to_string()),
        HandCardLocation::CompleteObjective(objective.to_string()),
    );
    let completed_objective = CompletedObjective {
        card: id,
        name: objective.to_string(),
    };
    game.player_mut(player)
        .completed_objectives
        .push(completed_objective.clone());
}

pub(crate) fn gain_objective_card_from_pile(game: &mut Game, player: &EventPlayer) {
    if let Some(c) = draw_objective_card_from_pile(game, player) {
        gain_objective_card(game, player.index, c);
    }
}

pub(crate) fn log_gain_objective_card(
    game: &mut Game,
    player: &EventPlayer,
    objective_card: u8,
    from: HandCardLocation,
) {
    log_card_transfer(
        game,
        &HandCard::ObjectiveCard(objective_card),
        from,
        HandCardLocation::Hand(player.index),
        &player.origin,
    );
}

pub(crate) fn draw_objective_card_from_pile(game: &mut Game, player: &EventPlayer) -> Option<u8> {
    draw_great_seer_card(game, player).or_else(|| {
        let card = draw_card_from_pile(
            game,
            player,
            "Objective Card",
            |g| &mut g.objective_cards_left,
            |g| g.cache.get_objective_cards().iter().map(|c| c.id).collect(),
            |p| {
                p.objective_cards
                    .clone()
                    .into_iter()
                    .chain(p.completed_objectives.iter().map(|o| o.card))
                    .collect_vec()
            },
        );
        if let Some(card) = card {
            log_gain_objective_card(game, player, card, HandCardLocation::DrawPile);
        }
        card
    })
}

fn draw_great_seer_card(game: &mut Game, player: &EventPlayer) -> Option<u8> {
    let mut remove_great_seer = false;
    let mut result = None;
    if let Some(great_seer) = find_great_seer(game)
        && let Some(o) = great_seer.assigned_objectives.iter().find_map(|o| {
            if o.player == player.index {
                Some(o.clone())
            } else {
                None
            }
        })
    {
        remove_element(&mut great_seer.assigned_objectives, &o)
            .unwrap_or_else(|| panic!("should be able to remove objective card {o:?}"));
        remove_great_seer = great_seer.assigned_objectives.is_empty();
        result = Some(o.objective_card);
        log_gain_objective_card(
            game,
            player,
            o.objective_card,
            HandCardLocation::GreatSeer(player.index),
        );
    }

    if remove_great_seer {
        remove_element_by(&mut game.permanent_effects, |e| {
            matches!(e, PermanentEffect::GreatSeer(_))
        })
        .unwrap_or_else(|| panic!("should be able to remove great seer"));
    }
    result
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

pub(crate) fn discard_objective_card(
    game: &mut Game,
    player: usize,
    card: u8,
    origin: &EventOrigin,
    to: HandCardLocation,
) {
    let card = remove_element_by(&mut game.player_mut(player).objective_cards, |&id| {
        id == card
    })
    .unwrap_or_else(|| panic!("should be able to discard objective card {card}"));
    deinit_objective_card(game, player, card);
    log_card_transfer(
        game,
        &HandCard::ObjectiveCard(card),
        HandCardLocation::Hand(player),
        to,
        origin,
    );
}
