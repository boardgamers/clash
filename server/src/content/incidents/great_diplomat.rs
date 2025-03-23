use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::ActionCard;
use crate::content::builtin::Builtin;
use crate::content::incidents::great_persons::{
    great_person_action_card, GREAT_PERSON_DESCRIPTION,
};
use crate::game::Game;
use crate::incident::{IncidentBuilder, PermanentIncidentEffect};
use crate::player_events::IncidentTarget;
use crate::playing_actions::ActionType;
use crate::utils::remove_element_by;
use serde::{Deserialize, Serialize};

pub(crate) const DIPLOMAT_ID: u8 = 57;

pub(crate) fn great_diplomat() -> ActionCard {
    great_person_action_card::<_, String>(
        DIPLOMAT_ID,
        "Great Diplomat",
        &format!(
            "{GREAT_PERSON_DESCRIPTION} Choose another player. \
                You cannot attack each other, unless you pay 2 culture tokens. \
                Discard this card if either player attacks the other. \
                You may discard the card as a regular action.",
        ),
        ActionType::regular(),
        &[],
        |_game, _player| true,
    )
    .add_simple_persistent_event_listener(
        |e| &mut e.on_play_action_card,
        0,
        |game, _player_index, player_name, _| {
            game.add_info_log_item(&format!("{player_name} ended diplomatic relations.",));
            remove_element_by(&mut game.permanent_incident_effects, |e| {
                matches!(e, PermanentIncidentEffect::DiplomaticRelations(_))
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

pub(crate) fn choose_diplomat_partner(b: IncidentBuilder) -> IncidentBuilder {
    b.add_incident_player_request(
        IncidentTarget::AllPlayers,
        "Select a player to be your diplomat partner",
        |_, _, _| true,
        1,
        |game, s, _| {
            game.add_info_log_item(&format!(
                "{} initiated diplomatic relations with {}",
                s.player_name,
                game.player_name(s.choice),
            ));
            game.permanent_incident_effects
                .push(PermanentIncidentEffect::DiplomaticRelations(
                    DiplomaticRelations {
                        active_player: s.player_index,
                        passive_player: s.choice,
                    },
                ));
        },
    )
}

pub(crate) fn use_diplomatic_relations() -> Builtin {
    Builtin::builder("Diplomatic Relations", "")
        .add_simple_persistent_event_listener(
            |e| &mut e.on_combat_start,
            2,
            |game, player_index, player_name, _| {
                let partner = diplomatic_relations_partner(game, player_index);
                if let Some(partner) = partner {
                    game.add_info_log_item(&format!(
                        "{} paid 2 culture tokens to end diplomatic relations with {} using a surprise attack.",
                        player_name,
                        game.player_name(partner),
                    ));
                    remove_element_by(&mut game.permanent_incident_effects, |e| {
                        matches!(e, PermanentIncidentEffect::DiplomaticRelations(_))
                    });
                }
            },
        )
        .build()
}

pub(crate) fn diplomatic_relations_partner(game: &Game, p: usize) -> Option<usize> {
    game.permanent_incident_effects.iter().find_map(|e| {
        if let PermanentIncidentEffect::DiplomaticRelations(d) = e {
            d.partner(p)
        } else {
            None
        }
    })
}
