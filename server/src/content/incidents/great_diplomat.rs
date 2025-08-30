use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::ActionCard;
use crate::content::ability::Ability;
use crate::content::effects::PermanentEffect;
use crate::content::incidents::great_persons::{
    great_person_card,
    GreatPersonType,
};
use crate::game::Game;
use crate::incident::IncidentBuilder;
use crate::player_events::IncidentTarget;
use crate::utils::remove_element_by;
use serde::{Deserialize, Serialize};

pub(crate) const DIPLOMAT_ID: u8 = 57;

pub(crate) fn great_diplomat() -> ActionCard {
    great_person_card::<_>(
        DIPLOMAT_ID,
        GreatPersonType::Public,
        "Great Diplomat",
        "Choose another player. \
        You cannot attack each other, unless you pay 2 culture tokens. \
        Discard this card if either player attacks the other. \
        You may discard the card as a regular action.",
        // action for ending diplomatic relations
        |c| c.action().no_resources(),
        vec![],
        |_game, _player| true,
    )
    .add_simple_persistent_event_listener(
        |e| &mut e.play_action_card,
        0,
        |game, p, _| {
            p.log(game, "Ended diplomatic relations.");
            remove_element_by(&mut game.permanent_effects, |e| {
                matches!(e, PermanentEffect::DiplomaticRelations(_))
            });
        },
    )
    .build()
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct DiplomaticRelations {
    pub active_player: usize,
    pub passive_player: usize,
}

impl DiplomaticRelations {
    pub fn new(active_player: usize, passive_player: usize) -> Self {
        Self {
            active_player,
            passive_player,
        }
    }

    pub fn partner(&self, player: usize) -> Option<usize> {
        if player == self.passive_player {
            Some(self.active_player)
        } else if player == self.active_player {
            Some(self.passive_player)
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Negotiations {
    #[serde(flatten)]
    pub relations: DiplomaticRelations,
    pub remaining_turns: usize,
}

pub(crate) fn choose_diplomat_partner(b: IncidentBuilder) -> IncidentBuilder {
    b.add_incident_player_request(
        IncidentTarget::SelectedPlayer,
        "Select a player to be your diplomat partner",
        |_, _, _| true,
        1,
        |game, s, _| {
            s.log(
                game,
                &format!(
                    "Initiated diplomatic relations with {}",
                    game.player_name(s.choice),
                ),
            );
            game.permanent_effects
                .push(PermanentEffect::DiplomaticRelations(
                    DiplomaticRelations::new(s.player_index, s.choice),
                ));
        },
    )
}

pub(crate) fn use_diplomatic_relations() -> Ability {
    Ability::builder("Diplomatic Relations", "")
        .add_simple_persistent_event_listener(
            |e| &mut e.combat_start,
            2,
            |game, p, _| {
                if let Some(partner) = diplomatic_relations_partner(game, p.index) {
                    p.log(
                        game,
                        &format!(
                            "Diplomatic relations with {} ended with a surprise attack.",
                            game.player_name(partner),
                        ),
                    );
                    remove_element_by(&mut game.permanent_effects, |e| {
                        matches!(e, PermanentEffect::DiplomaticRelations(_))
                    });
                }
            },
        )
        .build()
}

pub(crate) fn diplomatic_relations_partner(game: &Game, p: usize) -> Option<usize> {
    game.permanent_effects.iter().find_map(|e| {
        if let PermanentEffect::DiplomaticRelations(d) = e {
            d.partner(p)
        } else {
            None
        }
    })
}
