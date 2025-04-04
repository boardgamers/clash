use crate::ability_initializer::{
    AbilityInitializerBuilder, AbilityInitializerSetup, AbilityListeners,
};
use crate::card::draw_card_from_pile;
use crate::content::action_cards;
use crate::content::action_cards::get_civil_card;
use crate::content::persistent_events::PersistentEventType;
use crate::content::tactics_cards::TacticsCardFactory;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::log::{ActionLogItem, current_player_turn_log, current_player_turn_log_mut};
use crate::player::Player;
use crate::playing_actions::ActionType;
use crate::position::Position;
use crate::tactics_card::TacticsCard;
use crate::utils::remove_element_by;
use action_cards::get_action_card;
use serde::{Deserialize, Serialize};
use std::slice::from_ref;

pub type CanPlayCard = Box<dyn Fn(&Game, &Player, &ActionCardInfo) -> bool>;

#[derive(PartialEq, Eq)]
pub enum CivilCardTarget {
    ActivePlayer,
    AllPlayers,
}

pub struct CivilCard {
    pub name: String,
    pub description: String,
    pub can_play: CanPlayCard,
    pub listeners: AbilityListeners,
    pub action_type: ActionType,
    pub requirement: Option<CivilCardRequirement>,
    pub(crate) target: CivilCardTarget,
}

pub struct ActionCard {
    pub id: u8,
    pub civil_card: CivilCard,
    pub tactics_card: Option<TacticsCard>,
}

impl ActionCard {
    #[must_use]
    fn new(id: u8, civil_card: CivilCard, tactics_card: Option<TacticsCard>) -> Self {
        Self {
            id,
            civil_card,
            tactics_card,
        }
    }

    #[must_use]
    pub fn builder<F>(
        id: u8,
        name: &str,
        description: &str,
        action_type: ActionType,
        can_play: F,
    ) -> ActionCardBuilder
    where
        F: Fn(&Game, &Player, &ActionCardInfo) -> bool + 'static,
    {
        ActionCardBuilder {
            id,
            name: name.to_string(),
            description: description.to_string(),
            can_play: Box::new(can_play),
            requirement: None,
            builder: AbilityInitializerBuilder::new(),
            tactics_card: None,
            action_type,
            target: CivilCardTarget::ActivePlayer,
        }
    }
}

pub struct ActionCardBuilder {
    id: u8,
    name: String,
    description: String,
    action_type: ActionType,
    can_play: CanPlayCard,
    requirement: Option<CivilCardRequirement>,
    tactics_card: Option<TacticsCard>,
    builder: AbilityInitializerBuilder,
    target: CivilCardTarget,
}

impl ActionCardBuilder {
    #[must_use]
    pub fn tactics_card(mut self, tactics_card: TacticsCardFactory) -> Self {
        self.tactics_card = Some(tactics_card(self.id));
        self
    }

    #[must_use]
    pub fn requirement(mut self, requirement: CivilCardRequirement) -> Self {
        self.requirement = Some(requirement);
        self
    }

    #[must_use]
    pub fn target(mut self, target: CivilCardTarget) -> Self {
        self.target = target;
        self
    }

    #[must_use]
    pub fn build(self) -> ActionCard {
        ActionCard::new(
            self.id,
            CivilCard {
                name: self.name,
                description: self.description,
                can_play: self.can_play,
                requirement: self.requirement,
                listeners: self.builder.build(),
                action_type: self.action_type,
                target: self.target,
            },
            self.tactics_card,
        )
    }
}

impl AbilityInitializerSetup for ActionCardBuilder {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder {
        &mut self.builder
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::CivilCard(self.id)
    }
}

pub(crate) fn play_action_card(game: &mut Game, player_index: usize, id: u8) {
    discard_action_card(game, player_index, id);
    let mut satisfying_action: Option<usize> = None;
    let card = get_civil_card(id);
    if let Some(requirement) = card.requirement {
        if let Some(action_log_index) = requirement.satisfying_action(game, id) {
            satisfying_action = Some(action_log_index);
            current_player_turn_log_mut(game).items[action_log_index]
                .civil_card_match
                .as_mut()
                .expect("civil card match")
                .played_cards
                .push(id);
        }
    }
    on_play_action_card(
        game,
        player_index,
        ActionCardInfo::new(
            id,
            satisfying_action,
            (card.target == CivilCardTarget::AllPlayers).then_some(player_index),
        ),
    );
}

pub(crate) fn on_play_action_card(game: &mut Game, player_index: usize, i: ActionCardInfo) {
    let players = match get_civil_card(i.id).target {
        CivilCardTarget::ActivePlayer => vec![player_index],
        CivilCardTarget::AllPlayers => game.human_players(player_index),
    };

    let _ = game.trigger_persistent_event_with_listener(
        &players,
        |e| &mut e.play_action_card,
        &get_civil_card(i.id).listeners,
        i,
        PersistentEventType::ActionCard,
        None,
        |_| {},
    );
}

pub(crate) fn gain_action_card_from_pile(game: &mut Game, player: usize) {
    if let Some(c) = draw_action_card_from_pile(game) {
        game.add_info_log_item(&format!(
            "{} gained an action card from the pile",
            game.player_name(player)
        ));
        gain_action_card(game, player, &c);
    }
}

fn draw_action_card_from_pile(game: &mut Game) -> Option<ActionCard> {
    draw_card_from_pile(
        game,
        "Action Card",
        false,
        |g| &mut g.action_cards_left,
        || action_cards::get_all().iter().map(|c| c.id).collect(),
        |p| p.action_cards.clone(),
    )
    .map(get_action_card)
}

pub(crate) fn gain_action_card(game: &mut Game, player_index: usize, action_card: &ActionCard) {
    game.players[player_index].action_cards.push(action_card.id);
}

pub(crate) fn discard_action_card(game: &mut Game, player: usize, card: u8) {
    remove_element_by(&mut game.player_mut(player).action_cards, |&id| id == card);
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ActionCardInfo {
    pub id: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_position: Option<Position>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub selected_positions: Vec<Position>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_player: Option<usize>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub satisfying_action: Option<usize>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_player: Option<usize>, // only set when all players are playing the action card
}

impl ActionCardInfo {
    #[must_use]
    pub fn new(id: u8, satisfying_action: Option<usize>, active_player: Option<usize>) -> Self {
        Self {
            id,
            selected_position: None,
            selected_positions: Vec::new(),
            selected_player: None,
            answer: None,
            satisfying_action,
            active_player,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum CivilCardOpportunity {
    CaptureCity,
    WinLandBattle,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct CivilCardMatch {
    pub opportunity: CivilCardOpportunity,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub played_cards: Vec<u8>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opponent: Option<usize>,
}

impl CivilCardMatch {
    #[must_use]
    pub fn new(opportunity: CivilCardOpportunity, opponent: Option<usize>) -> Self {
        Self {
            opportunity,
            played_cards: Vec::new(),
            opponent,
        }
    }

    pub(crate) fn store(self, game: &mut Game) {
        current_player_turn_log_mut(game)
            .items
            .last_mut()
            .expect("no action log")
            .civil_card_match = Some(self);
    }
}

pub struct CivilCardRequirement {
    pub opportunities: Vec<CivilCardOpportunity>, // by order of preference
    pub just_before: bool,
}

impl CivilCardRequirement {
    #[must_use]
    pub fn new(opportunities: Vec<CivilCardOpportunity>, just_before: bool) -> Self {
        Self {
            opportunities,
            just_before,
        }
    }

    #[must_use]
    pub fn satisfying_action(&self, game: &Game, action_card_id: u8) -> Option<usize> {
        let mut l: &[ActionLogItem] = &current_player_turn_log(game).items;
        if let Some(c) = game.current_action_log_index {
            l = &l[..c];
        }
        if self.just_before {
            l = if l.is_empty() {
                &[]
            } else {
                from_ref(&l[l.len() - 1])
            };
        }
        let sister_card = if action_card_id % 2 == 0 {
            action_card_id - 1
        } else {
            action_card_id + 1
        };
        self.opportunities.iter().find_map(|o| {
            l.iter().position(|a| {
                if let Some(civil_card_match) = &a.civil_card_match {
                    if civil_card_match.opportunity == *o
                        && !civil_card_match.played_cards.contains(&sister_card)
                    {
                        return true;
                    }
                }
                false
            })
        })
    }
}
