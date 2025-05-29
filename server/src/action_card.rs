use crate::ability_initializer::{
    AbilityInitializerBuilder, AbilityInitializerSetup, AbilityListeners,
};
use crate::advance::Advance;
use crate::card::{discard_card, draw_card_from_pile};
use crate::combat_stats::CombatStats;
use crate::content::persistent_events::{
    PersistentEventType, TriggerPersistentEventParams, trigger_persistent_event_with_listener,
};
use crate::content::tactics_cards::TacticsCardFactory;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::log::{current_player_turn_log, current_player_turn_log_mut};
use crate::player::Player;
use crate::playing_actions::ActionCost;
use crate::position::Position;
use crate::tactics_card::TacticsCard;
use crate::utils::remove_element_by;
use crate::wonder::Wonder;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub type CanPlayCard = Arc<dyn Fn(&Game, &Player, &ActionCardInfo) -> bool + Sync + Send>;
pub type CombatRequirement = Arc<dyn Fn(&CombatStats, &Player) -> bool + Sync + Send>;

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum CivilCardTarget {
    ActivePlayer,
    AllPlayers,
}

#[derive(Clone)]
pub struct CivilCard {
    pub name: String,
    pub description: String,
    pub can_play: CanPlayCard,
    pub listeners: AbilityListeners,
    pub action_type: ActionCost,
    pub combat_requirement: Option<CombatRequirement>,
    pub(crate) target: CivilCardTarget,
}

#[derive(Clone)]
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
        action_type: ActionCost,
        can_play: F,
    ) -> ActionCardBuilder
    where
        F: Fn(&Game, &Player, &ActionCardInfo) -> bool + 'static + Sync + Send,
    {
        ActionCardBuilder {
            id,
            name: name.to_string(),
            description: description.to_string(),
            can_play: Arc::new(can_play),
            combat_requirement: None,
            builder: AbilityInitializerBuilder::new(),
            tactics_card: None,
            action_type,
            target: CivilCardTarget::ActivePlayer,
        }
    }

    #[must_use]
    pub fn name(&self) -> String {
        format!(
            "{}/{}",
            self.civil_card.name,
            self.tactics_card
                .as_ref()
                .map_or("-".to_string(), |c| c.name.clone())
        )
    }
}

pub struct ActionCardBuilder {
    id: u8,
    name: String,
    description: String,
    action_type: ActionCost,
    can_play: CanPlayCard,
    combat_requirement: Option<CombatRequirement>,
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
    pub fn combat_requirement(mut self, requirement: CombatRequirement) -> Self {
        self.combat_requirement = Some(requirement);
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
                combat_requirement: self.combat_requirement,
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
    let card = game.cache.get_civil_card(id);
    let civil_card_target = card.target;
    if let Some(r) = &card.combat_requirement {
        if let Some(action_log_index) = combat_requirement_met(game, player_index, id, r) {
            satisfying_action = Some(action_log_index);
            current_player_turn_log_mut(game).items[action_log_index]
                .combat_stats
                .as_mut()
                .expect("combat stats")
                .claimed_action_cards
                .push(id);
        }
    }
    on_play_action_card(
        game,
        player_index,
        ActionCardInfo::new(
            id,
            satisfying_action,
            (civil_card_target == CivilCardTarget::AllPlayers).then_some(player_index),
        ),
    );
}

pub(crate) fn on_play_action_card(game: &mut Game, player_index: usize, i: ActionCardInfo) {
    let players = match &game.cache.get_civil_card(i.id).target {
        CivilCardTarget::ActivePlayer => vec![player_index],
        CivilCardTarget::AllPlayers => game.human_players(player_index),
    };

    let _ = trigger_persistent_event_with_listener(
        game,
        &players,
        |e| &mut e.play_action_card,
        &game.cache.get_civil_card(i.id).listeners.clone(),
        i,
        PersistentEventType::ActionCard,
        TriggerPersistentEventParams::default(),
    );
}

pub(crate) fn gain_action_card_from_pile(game: &mut Game, player: usize) {
    if game
        .player(player)
        .wonders_owned
        .contains(Wonder::GreatMausoleum)
    {
        game.player_mut(player).great_mausoleum_action_cards += 1;
    } else {
        do_gain_action_card_from_pile(game, player);
    }
}

pub(crate) fn do_gain_action_card_from_pile(game: &mut Game, player: usize) {
    if let Some(c) = draw_action_card_from_pile(game) {
        gain_action_card(game, player, c);
        game.add_info_log_item(&format!(
            "{} gained an action card from the pile",
            game.player_name(player)
        ));
    }
}

fn draw_action_card_from_pile(game: &mut Game) -> Option<u8> {
    draw_card_from_pile(
        game,
        "Action Card",
        |g| &mut g.action_cards_left,
        |g| g.cache.get_action_cards().iter().map(|c| c.id).collect(),
        |p| p.action_cards.clone(),
    )
}

pub(crate) fn gain_action_card(game: &mut Game, player_index: usize, action_card: u8) {
    game.players[player_index].action_cards.push(action_card);
}

pub(crate) fn discard_action_card(game: &mut Game, player: usize, card: u8) {
    let card = remove_element_by(&mut game.player_mut(player).action_cards, |&id| id == card)
        .unwrap_or_else(|| panic!("action card not found {card}"));
    discard_card(|g| &mut g.action_cards_discarded, card, player, game);
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ActionCardInfo {
    pub id: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_position: Option<Position>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_advance: Option<Advance>,
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
            selected_advance: None,
            answer: None,
            satisfying_action,
            active_player,
        }
    }
}

#[must_use]
pub fn combat_requirement_met(
    game: &Game,
    player: usize,
    action_card_id: u8,
    requirement: &CombatRequirement,
) -> Option<usize> {
    let sister_card = if action_card_id % 2 == 0 {
        action_card_id - 1
    } else {
        action_card_id + 1
    };

    current_player_turn_log(game).items.iter().position(|a| {
        if let Some(stats) = &a.combat_stats {
            if requirement(stats, game.player(player))
                && !stats.claimed_action_cards.contains(&sister_card)
            {
                return true;
            }
        }
        false
    })
}

pub(crate) fn can_play_civil_card(game: &Game, p: &Player, id: u8) -> Result<(), String> {
    if !p.action_cards.contains(&id) {
        return Err("Action card not available".to_string());
    }

    let civil_card = game.cache.get_civil_card(id);
    let mut satisfying_action: Option<usize> = None;
    if let Some(r) = &civil_card.combat_requirement {
        if let Some(action_log_index) = combat_requirement_met(game, p.index, id, r) {
            satisfying_action = Some(action_log_index);
        } else {
            return Err("Requirement not met".to_string());
        }
    }
    if !(civil_card.can_play)(game, p, &ActionCardInfo::new(id, satisfying_action, None)) {
        return Err("Cannot play action card".to_string());
    }
    Ok(())
}
