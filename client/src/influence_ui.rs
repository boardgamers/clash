use crate::client_state::ActiveDialog;
use crate::custom_phase_ui::{MultiSelection, SelectedStructureInfo, SelectedStructureStatus};
use crate::dialog_ui::BaseOrCustomDialog;
use server::content::persistent_events::MultiRequest;
use server::cultural_influence::available_influence_culture;
use server::game::Game;

pub fn new_cultural_influence_dialog(
    game: &Game,
    player: usize,
    d: BaseOrCustomDialog,
) -> ActiveDialog {
    let a = available_influence_culture(game, player, &d.action_type)
        .into_iter()
        .map(|(s, info)| {
            let (status, label, tooltip) = match info {
                Ok(i) => {
                    let boost = i.range_boost_cost.default.amount().to_string();
                    let mut tooltip = format!("Range boost cost: {boost}");
                    (
                        if i.prevent_boost {
                            tooltip += " - you cannot boost the dice roll";
                            SelectedStructureStatus::Warn
                        } else {
                            SelectedStructureStatus::Valid
                        },
                        Some(boost),
                        tooltip,
                    )
                }
                Err(e) => (SelectedStructureStatus::Invalid, None, e),
            };

            SelectedStructureInfo::new(
                s.position,
                s.structure,
                status,
                label.clone(),
                Some(tooltip),
            )
        })
        .collect();
    ActiveDialog::StructuresRequest(
        Some(d),
        MultiSelection::new(MultiRequest::new(
            a,
            0..=1,
            "Select structure to influence culture",
        )),
    )
}
