use crate::ability_initializer::{
    AbilityInitializerBuilder, AbilityInitializerSetup, AbilityListeners,
};
use crate::advance::Advance;
use crate::card::draw_card_from_pile;
use crate::combat_listeners::CombatResult;
use crate::content::action_cards;
use crate::content::action_cards::get_civil_card;
use crate::content::persistent_events::PersistentEventType;
use crate::content::tactics_cards::TacticsCardFactory;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::log::{current_player_turn_log, current_player_turn_log_mut};
use crate::player::Player;
use crate::playing_actions::ActionCost;
use crate::position::Position;
use crate::tactics_card::TacticsCard;
use crate::utils::remove_element_by;
use action_cards::get_action_card;
use serde::{Deserialize, Serialize};

pub type CanPlayCard = Box<dyn Fn(&Game, &Player, &ActionCardInfo) -> bool + Sync + Send>;

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
    pub action_type: ActionCost,
    pub requirement_land_battle_won: bool,
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
            can_play: Box::new(can_play),
            requirement_land_battle_won: false,
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
    action_type: ActionCost,
    can_play: CanPlayCard,
    requirement_land_battle_won: bool,
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
    pub fn requirement_land_battle_won(mut self) -> Self {
        self.requirement_land_battle_won = true;
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
                requirement_land_battle_won: self.requirement_land_battle_won,
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
    if card.requirement_land_battle_won {
        if let Some(action_log_index) = land_battle_won_action(game, player_index, id) {
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
        gain_action_card(game, player, c);
    }
}

fn draw_action_card_from_pile(game: &mut Game) -> Option<&'static ActionCard> {
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
    remove_element_by(&mut game.player_mut(player).action_cards, |&id| id == card)
        .unwrap_or_else(|| panic!("action card not found {card}"));
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
pub fn land_battle_won_action(game: &Game, player: usize, action_card_id: u8) -> Option<usize> {
    let sister_card = if action_card_id % 2 == 0 {
        action_card_id - 1
    } else {
        action_card_id + 1
    };

    current_player_turn_log(game).items.iter().position(|a| {
        if let Some(stats) = &a.combat_stats {
            if stats.result == Some(CombatResult::AttackerWins)
                && stats.battleground.is_land()
                && stats.attacker.player == player
                && !stats.claimed_action_cards.contains(&sister_card)
            {
                return true;
            }
        }
        false
    })
}
