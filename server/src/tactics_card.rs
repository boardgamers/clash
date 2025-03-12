use crate::ability_initializer::{
    AbilityInitializerBuilder, AbilityInitializerSetup, AbilityListeners,
};
use crate::action_card::ActionCard;
use crate::advance::AdvanceBuilder;
use crate::card::HandCard;
use crate::combat::{update_combat_strength, Combat, CombatModifier};
use crate::combat_listeners::{CombatRoundType, CombatStrength};
use crate::content::action_cards;
use crate::content::custom_phase_actions::HandCardsRequest;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::utils::remove_element_by;

#[derive(Clone, PartialEq, Eq, Copy)]
pub enum TacticsCardTarget {
    ActivePlayer,
    AllPlayers,
}

impl TacticsCardTarget {
    #[must_use]
    pub(crate) fn is_active(
        self,
        player: usize,
        combat: &Combat,
        round_type: &CombatRoundType,
    ) -> bool {
        match &self {
            TacticsCardTarget::ActivePlayer => match round_type {
                CombatRoundType::TacticsCardAttacker => combat.attacker == player,
                CombatRoundType::TacticsCardDefender => combat.defender == player,
                _ => panic!("TacticsCardTarget::ActivePlayer is not valid"),
            },
            TacticsCardTarget::AllPlayers => true,
        }
    }
}

pub enum FighterRequirement {
    Army,
    Fortress,
    Ship,
}

#[derive(Clone, PartialEq, Eq, Copy)]
pub enum CombatRole {
    Attacker,
    Defender,
}

impl CombatRole {
    #[must_use]
    pub fn is_attacker(self) -> bool {
        matches!(self, CombatRole::Attacker)
    }
}

pub struct TacticsCard {
    pub name: String,
    pub description: String,
    pub card_target: TacticsCardTarget,
    pub fighter_requirement: FighterRequirement,
    pub role_requirement: Option<CombatRole>,
    pub listeners: AbilityListeners,
}

impl TacticsCard {
    #[must_use]
    pub fn builder(
        name: &str,
        description: &str,
        tactics_card_target: TacticsCardTarget,
        fighter_requirement: FighterRequirement,
    ) -> TacticsCardBuilder {
        TacticsCardBuilder::new(name, description, tactics_card_target, fighter_requirement)
    }
}

pub struct TacticsCardBuilder {
    pub name: String,
    description: String,
    pub tactics_card_target: TacticsCardTarget,
    pub fighter_requirement: FighterRequirement,
    pub position_requirement: Option<CombatRole>,
    builder: AbilityInitializerBuilder,
}

impl TacticsCardBuilder {
    fn new(
        name: &str,
        description: &str,
        tactics_card_target: TacticsCardTarget,
        fighter_requirement: FighterRequirement,
    ) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            fighter_requirement,
            tactics_card_target,
            position_requirement: None,
            builder: AbilityInitializerBuilder::new(),
        }
    }

    // pub(crate) fn set_position_requirement(
    //     &mut self,
    //     position_requirement: PositionRequirement,
    // ) -> &mut Self {
    //     self.position_requirement = Some(position_requirement);
    //     self
    // }

    pub(crate) fn add_reveal_listener(
        self,
        priority: i32,
        listener: impl Fn(usize, &mut Game, &Combat, &mut CombatStrength) + Clone + 'static,
    ) -> Self {
        let target = self.tactics_card_target;
        self.add_simple_current_event_listener(
            |event| &mut event.on_combat_round_start_tactics,
            priority,
            move |game, player, _, s| {
                if target.is_active(player, &s.combat, &s.round_type) {
                    update_combat_strength(game, {
                        let l = listener.clone();
                        move |game, combat, s, _attacker| l(player, game, combat, s)
                    });
                }
            },
        )
    }

    #[must_use]
    pub fn build(self) -> TacticsCard {
        TacticsCard {
            name: self.name,
            description: self.description,
            card_target: self.tactics_card_target,
            fighter_requirement: self.fighter_requirement,
            role_requirement: self.position_requirement,
            listeners: self.builder.build(),
        }
    }
}

impl AbilityInitializerSetup for TacticsCardBuilder {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder {
        &mut self.builder
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::TacticsCard(self.name.clone())
    }
}

pub(crate) fn play_tactics_card(b: AdvanceBuilder) -> AdvanceBuilder {
    b.add_hand_card_request(
        |e| &mut e.on_combat_round_start,
        0,
        |game, player, s| {
            let cards = available_tactics_cards(game, player, &s.combat);
            if cards.is_empty() {
                return None;
            }

            let c = &s.combat;
            if player == c.defender
                && c.round == 1
                && c.modifiers.contains(&CombatModifier::TrojanHorse)
            {
                update_combat_strength(game, |_game, _c, s, _attacker| {
                    s.roll_log
                        .push("Trojan Horse denied playing Tactics Cards".to_string());
                });

                return None;
            }

            Some(HandCardsRequest::new(cards, 0..=1, "Play Tactics Card"))
        },
        |game, sel| {
            let name = &sel.player_name;
            if sel.choice.is_empty() {
                game.add_info_log_item(&format!("{name} did not play a Tactics Card"));
            } else {
                let player = sel.player_index;
                game.add_info_log_item(&format!("{name} played a Tactics Card"));
                let HandCard::ActionCard(card) = sel.choice[0] else {
                    panic!("Expected ActionCard, got {:?}", sel.choice[0]);
                };
                update_combat_strength(game, move |_game, _c, s, _attacker| {
                    s.tactics_card = Some(
                        action_cards::get_action_card(card)
                            .tactics_card
                            .name
                            .clone(),
                    );
                });
                remove_element_by(&mut game.get_player_mut(player).action_cards, |c| {
                    c.id == card
                });
            }
        },
    )
}

fn available_tactics_cards(game: &Game, player: usize, combat: &Combat) -> Vec<HandCard> {
    game.players[player]
        .action_cards
        .iter()
        .filter(|a| can_play_tactics_card(game, player, a, combat))
        .map(|a| HandCard::ActionCard(a.id))
        .collect()
}

fn can_play_tactics_card(game: &Game, player: usize, card: &ActionCard, combat: &Combat) -> bool {
    let position_met = card
        .tactics_card
        .role_requirement
        .as_ref()
        .is_none_or(|&r| combat.role(player) == r);

    let fighter_met = match card.tactics_card.fighter_requirement {
        FighterRequirement::Army => !combat.is_sea_battle(game),
        FighterRequirement::Fortress => combat.defender_fortress(game) && combat.defender == player,
        FighterRequirement::Ship => combat.is_sea_battle(game),
    };

    position_met && fighter_met
}
