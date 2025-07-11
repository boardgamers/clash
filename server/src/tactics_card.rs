use crate::ability_initializer::{
    AbilityInitializerBuilder, AbilityInitializerSetup, AbilityListeners,
};
use crate::action_card;
use crate::action_card::ActionCard;
use crate::advance::AdvanceBuilder;
use crate::card::{HandCard, HandCardLocation};
use crate::combat::{Combat, CombatModifier, get_combat_strength, update_combat_strength};
use crate::combat_listeners::{CombatRoundEnd, CombatRoundStart, CombatStrength};
use crate::content::persistent_events::HandCardsRequest;
use crate::events::{EventOrigin, EventPlayer};
use crate::game::Game;
use crate::player_events::{PersistentEvent, PersistentEvents};
use action_card::discard_action_card;
use std::fmt::Display;
use std::sync::Arc;

#[derive(Clone, PartialEq, Eq, Copy)]
pub enum TacticsCardTarget {
    ActivePlayer,
    Opponent,
    AllPlayers,
}

impl TacticsCardTarget {
    #[must_use]
    pub(crate) fn is_active(
        self,
        player: usize,
        combat: &Combat,
        expect_card: u8,
        attacker_card: Option<&u8>,
    ) -> bool {
        let card_player_role = if attacker_card.is_some_and(|c| *c == expect_card) {
            CombatRole::Attacker
        } else {
            CombatRole::Defender
        };
        let card_player = combat.player(card_player_role);

        match &self {
            TacticsCardTarget::ActivePlayer => card_player == player,
            TacticsCardTarget::Opponent => card_player == combat.opponent(player),
            TacticsCardTarget::AllPlayers => true,
        }
    }
}

#[derive(Debug, Clone)]
pub enum FighterRequirement {
    Army,
    Fortress,
    Ship,
}

impl Display for FighterRequirement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FighterRequirement::Army => write!(f, "Army"),
            FighterRequirement::Fortress => write!(f, "Fortress"),
            FighterRequirement::Ship => write!(f, "Ship"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CombatLocation {
    City,
    Sea,
    Land,
}

impl Display for CombatLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CombatLocation::City => write!(f, "City"),
            CombatLocation::Sea => write!(f, "Sea"),
            CombatLocation::Land => write!(f, "Land"),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Copy)]
pub enum CombatRole {
    Attacker,
    Defender,
}

impl Display for CombatRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CombatRole::Attacker => write!(f, "Attacker"),
            CombatRole::Defender => write!(f, "Defender"),
        }
    }
}

impl CombatRole {
    #[must_use]
    pub fn is_attacker(self) -> bool {
        matches!(self, CombatRole::Attacker)
    }
}

type TacticsChecker = Arc<dyn Fn(usize, &Game, &Combat) -> bool + Sync + Send>;

#[derive(Clone)]
pub struct TacticsCard {
    pub id: u8,
    pub name: String,
    pub description: String,
    pub fighter_requirement: Vec<FighterRequirement>,
    pub role_requirement: Option<CombatRole>,
    pub location_requirement: Option<CombatLocation>,
    pub checker: Option<TacticsChecker>,
    pub listeners: AbilityListeners,
}

impl TacticsCard {
    #[must_use]
    pub fn builder(id: u8, name: &str, description: &str) -> TacticsCardBuilder {
        TacticsCardBuilder::new(id, name, description)
    }
}

pub struct TacticsCardBuilder {
    pub id: u8,
    pub name: String,
    description: String,
    pub target: TacticsCardTarget,
    pub fighter_requirement: Vec<FighterRequirement>,
    pub role_requirement: Option<CombatRole>,
    pub location_requirement: Option<CombatLocation>,
    pub checker: Option<TacticsChecker>,
    builder: AbilityInitializerBuilder,
}

impl TacticsCardBuilder {
    fn new(id: u8, name: &str, description: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            description: description.to_string(),
            fighter_requirement: vec![],
            target: TacticsCardTarget::ActivePlayer,
            role_requirement: None,
            location_requirement: None,
            checker: None,
            builder: AbilityInitializerBuilder::new(),
        }
    }

    pub(crate) fn target(mut self, target: TacticsCardTarget) -> Self {
        self.target = target;
        self
    }

    pub(crate) fn role_requirement(mut self, role_requirement: CombatRole) -> Self {
        self.role_requirement = Some(role_requirement);
        self
    }

    pub(crate) fn fighter_requirement(mut self, fighter_requirement: FighterRequirement) -> Self {
        self.fighter_requirement.push(fighter_requirement);
        self
    }

    pub(crate) fn location_requirement(mut self, location_requirement: CombatLocation) -> Self {
        self.location_requirement = Some(location_requirement);
        self
    }

    pub(crate) fn fighter_any_requirement(
        mut self,
        fighter_requirement: &[FighterRequirement],
    ) -> Self {
        self.fighter_requirement
            .extend_from_slice(fighter_requirement);
        self
    }

    pub(crate) fn checker(
        mut self,
        checker: impl Fn(usize, &Game, &Combat) -> bool + Clone + 'static + Sync + Send,
    ) -> Self {
        self.checker = Some(Arc::new(checker));
        self
    }

    pub(crate) fn add_reveal_listener(
        self,
        priority: i32,
        listener: impl Fn(usize, &mut Game, &Combat, &mut CombatStrength)
        + Clone
        + 'static
        + Sync
        + Send,
    ) -> Self {
        self.add_combat_strength_listener(
            |event| &mut event.combat_round_start_tactics,
            priority,
            listener,
        )
    }

    fn add_combat_strength_listener<E>(
        self,
        event: E,
        priority: i32,
        listener: impl Fn(usize, &mut Game, &Combat, &mut CombatStrength)
        + Clone
        + 'static
        + Sync
        + Send,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut PersistentEvent<CombatRoundStart>
            + 'static
            + Clone
            + Sync
            + Send,
    {
        let target = self.target;
        let id = self.id;
        self.add_simple_persistent_event_listener(event, priority, move |game, p, s| {
            if s.is_active(p.index, id, target) {
                update_combat_strength(game, p.index, s, {
                    let l = listener.clone();
                    move |game, combat, s, _role| l(p.index, game, combat, s)
                });
            }
        })
    }

    pub(crate) fn add_resolve_listener(
        self,
        priority: i32,
        listener: impl Fn(&EventPlayer, &mut Game, &mut CombatRoundEnd) + Clone + 'static + Sync + Send,
    ) -> Self {
        let target = self.target;
        let id = self.id;
        self.add_simple_persistent_event_listener(
            |event| &mut event.combat_round_end_tactics,
            priority,
            move |game, p, s| {
                if s.is_active(p.index, id, target) {
                    listener(p, game, s);
                }
            },
        )
    }

    #[must_use]
    pub fn build(self) -> TacticsCard {
        TacticsCard {
            id: self.id,
            name: self.name,
            description: self.description,
            fighter_requirement: self.fighter_requirement,
            role_requirement: self.role_requirement,
            location_requirement: self.location_requirement,
            checker: self.checker,
            listeners: self.builder.build(),
        }
    }
}

impl AbilityInitializerSetup for TacticsCardBuilder {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder {
        &mut self.builder
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::TacticsCard(self.id)
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}

pub(crate) fn play_tactics_card(b: AdvanceBuilder) -> AdvanceBuilder {
    b.add_hand_card_request(
        |e| &mut e.combat_round_start,
        0,
        |game, player, s| {
            let p = player.index;
            if get_combat_strength(p, s).deny_tactics_card {
                return None;
            }

            let cards = available_tactics_cards(game, p, &s.combat);
            if cards.is_empty() {
                return None;
            }

            let c = &s.combat;
            if p == c.defender()
                && c.first_round()
                && c.modifiers.contains(&CombatModifier::TrojanHorse)
            {
                update_combat_strength(game, p, s, |_game, _c, s, _role| {
                    s.roll_log
                        .push("Trojan Horse denied playing Tactics Cards".to_string());
                });

                return None;
            }

            Some(HandCardsRequest::new(cards, 0..=1, "Play Tactics Card"))
        },
        |game, s, r| {
            if s.choice.is_empty() {
                s.log(game, "Did not play a Tactics Card");
            } else {
                let player = s.player_index;
                let HandCard::ActionCard(card) = s.choice[0] else {
                    panic!("Expected ActionCard, got {:?}", s.choice[0]);
                };
                update_combat_strength(game, player, r, move |_game, _c, s, _role| {
                    s.tactics_card = Some(card);
                });
                discard_action_card(
                    game,
                    player,
                    card,
                    &s.origin,
                    HandCardLocation::PlayToDiscardFaceDown,
                );
            }
        },
    )
}

fn available_tactics_cards(game: &Game, player: usize, combat: &Combat) -> Vec<HandCard> {
    game.players[player]
        .action_cards
        .iter()
        .map(|id| game.cache.get_action_card(*id))
        .filter(|a| can_play_tactics_card(game, player, a, combat))
        .map(|a| HandCard::ActionCard(a.id))
        .collect()
}

fn can_play_tactics_card(game: &Game, player: usize, card: &ActionCard, combat: &Combat) -> bool {
    match &card.tactics_card {
        Some(card) => {
            let position_met = card
                .role_requirement
                .as_ref()
                .is_none_or(|&r| combat.role(player) == r);

            let fighter_met = card.fighter_requirement.is_empty()
                || card.fighter_requirement.iter().any(|r| match r {
                    FighterRequirement::Army => combat.is_land_battle(game),
                    FighterRequirement::Fortress => {
                        combat.defender_fortress(game) && combat.defender() == player
                    }
                    FighterRequirement::Ship => combat.is_sea_battle(game),
                });

            let location_met = card.location_requirement.as_ref().is_none_or(|l| match l {
                // city is also land!
                CombatLocation::City => combat.defender_city(game).is_some(),
                CombatLocation::Sea => combat.is_sea_battle(game),
                CombatLocation::Land => combat.is_land_battle(game),
            });

            let checker_met = card
                .checker
                .as_ref()
                .is_none_or(|c| c(player, game, combat));

            position_met && fighter_met && location_met && checker_met
        }
        _ => false,
    }
}
