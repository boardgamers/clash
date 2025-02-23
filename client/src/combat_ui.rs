use crate::client_state::StateUpdate;
use crate::dialog_ui::{cancel_button_with_tooltip, ok_button, OkTooltip};
use crate::render_context::RenderContext;
use server::action::{Action, CombatAction, PlayActionCard};

pub fn retreat_dialog(rc: &RenderContext) -> StateUpdate {
    if ok_button(rc, OkTooltip::Valid("Retreat".to_string())) {
        return retreat(true);
    }
    if cancel_button_with_tooltip(rc, "Decline") {
        return retreat(false);
    }
    StateUpdate::None
}

fn retreat(retreat: bool) -> StateUpdate {
    StateUpdate::Execute(Action::Combat(CombatAction::Retreat(retreat)))
}
// 
// #[derive(Clone)]
// pub struct RemoveCasualtiesSelection {
//     pub player: usize,
//     pub position: Position,
//     pub needed: u8,
//     pub needed_carried: u8,
//     pub selectable: Vec<u32>,
//     pub units: Vec<u32>,
// }
// 
// impl RemoveCasualtiesSelection {
//     pub fn new(
//         player: usize,
//         position: Position,
//         needed: u8,
//         needed_carried: u8,
//         selectable: Vec<u32>,
//     ) -> Self {
//         RemoveCasualtiesSelection {
//             player,
//             position,
//             needed,
//             needed_carried,
//             units: Vec::new(),
//             selectable,
//         }
//     }
// 
//     #[must_use]
//     pub fn total_needed(&self) -> u8 {
//         self.needed + self.needed_carried
//     }
// }
// 
// impl UnitSelection for RemoveCasualtiesSelection {
//     fn selected_units_mut(&mut self) -> &mut Vec<u32> {
//         &mut self.units
//     }
// 
//     fn can_select(&self, _game: &Game, unit: &Unit) -> bool {
//         self.selectable.contains(&unit.id)
//     }
// }
// 
// impl ConfirmSelection for RemoveCasualtiesSelection {
//     fn cancel_name(&self) -> Option<&str> {
//         None
//     }
// 
//     fn confirm(&self, game: &Game) -> OkTooltip {
//         let units = self
//             .units
//             .iter()
//             .map(|id| game.get_player(self.player).get_unit(*id).unwrap());
//         let carried = units.filter(|u| u.carrier_id.is_some()).count() as u8;
// 
//         if carried == self.needed_carried && self.units.len() as u8 == self.total_needed() {
//             OkTooltip::Valid("Remove casualties".to_string())
//         } else {
//             OkTooltip::Invalid(format!(
//                 "Need to select {} units",
//                 self.total_needed() - self.units.len() as u8
//             ))
//         }
//     }
// }
// 
// pub fn remove_casualties_dialog(
//     rc: &RenderContext,
//     sel: &RemoveCasualtiesSelection,
// ) -> StateUpdate {
//     unit_ui::unit_selection_dialog::<RemoveCasualtiesSelection>(
//         rc,
//         sel,
//         |new: RemoveCasualtiesSelection| {
//             StateUpdate::Execute(Action::Combat(CombatAction::RemoveCasualties(
//                 new.units.clone(),
//             )))
//         },
//     )
// }

pub fn play_action_card_dialog(rc: &RenderContext) -> StateUpdate {
    if cancel_button_with_tooltip(rc, "Play no action card") {
        return StateUpdate::Execute(Action::Combat(CombatAction::PlayActionCard(
            PlayActionCard::None,
        )));
    }
    StateUpdate::None
}
// 
// pub fn remove_casualties_active_dialog(game: &Game, r: &CombatRoundResult, player: usize) -> ActiveDialog {
//     let c = get_combat(game);
// 
//     let (position, casualties, selectable) = if player == c.attacker {
//         (
//             c.attacker_position,
//             r.attacker_hits,
//             active_attackers(game, c.attacker, &c.attackers, c.defender_position)
//                 .clone()
//                 .into_iter()
//                 .chain(c.attackers.iter().flat_map(|a| {
//                     let units = carried_units(*a, game.get_player(r.player));
//                     units
//                 }))
//                 .collect(),
//         )
//     } else if player == c.defender {
//         (
//             c.defender_position,
//             r.defender_hits,
//             c.active_defenders(game, c.defender, c.defender_position),
//         )
//     } else {
//         panic!("player should be either defender or attacker")
//     };
// 
//     ActiveDialog::RemoveCasualties(RemoveCasualtiesSelection::new(
//         player,
//         position,
//         casualties,
//         c.carried_units_casualties(game, player, casualties),
//         selectable,
//     ))
// }
