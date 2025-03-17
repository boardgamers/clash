use crate::ability_initializer::{
    AbilityInitializerBuilder, AbilityInitializerSetup, AbilityListeners,
};
use crate::card::draw_card_from_pile;
use crate::content::action_cards;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::tactics_card::TacticsCard;

pub struct CivilCard {
    pub name: String,
    pub description: String,
    pub listeners: AbilityListeners,
}

pub struct ActionCard {
    pub id: u8,
    pub civil_card: CivilCard,
    pub tactics_card: TacticsCard,
}

impl ActionCard {
    #[must_use]
    fn new(id: u8, civil_card: CivilCard, tactics_card: TacticsCard) -> Self {
        Self {
            id,
            civil_card,
            tactics_card,
        }
    }

    #[must_use]
    pub fn civil_card_builder(
        id: u8,
        name: &str,
        description: &str,
        tactics_card: TacticsCard,
    ) -> CivilCardBuilder {
        CivilCardBuilder::new(id, name, description, tactics_card)
    }

    #[must_use]
    pub fn description(&self) -> Vec<String> {
        // todo
        vec![format!(
            "{}\n\n{}",
            self.civil_card.description, self.tactics_card.description
        )]
    }
}

pub struct CivilCardBuilder {
    id: u8,
    name: String,
    description: String,
    tactics_card: TacticsCard,
    builder: AbilityInitializerBuilder,
}

impl CivilCardBuilder {
    fn new(id: u8, name: &str, description: &str, tactics_card: TacticsCard) -> Self {
        Self {
            id,
            name: name.to_string(),
            description: description.to_string(),
            tactics_card,
            builder: AbilityInitializerBuilder::new(),
        }
    }

    #[must_use]
    pub fn build(self) -> ActionCard {
        ActionCard::new(
            self.id,
            CivilCard {
                name: self.name,
                description: self.description,
                listeners: self.builder.build(),
            },
            self.tactics_card,
        )
    }
}

impl AbilityInitializerSetup for CivilCardBuilder {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder {
        &mut self.builder
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::ActionCard(self.id)
    }
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

fn draw_action_card_from_pile(game: &mut Game) -> Option<ActionCard> {
    draw_card_from_pile(
        game,
        "Action Card",
        false,
        |g| &mut g.action_cards_left,
        || action_cards::get_all().iter().map(|c| c.id).collect(),
        |p| p.action_cards.iter().map(|c| c.id).collect(),
    )
    .map(action_cards::get_action_card)
}

fn gain_action_card(game: &mut Game, player_index: usize, action_card: ActionCard) {
    game.players[player_index].action_cards.push(action_card);
}
