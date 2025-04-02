use crate::action_buttons::base_or_custom_available;
use crate::client_state::ActiveDialog;
use crate::custom_phase_ui::{MultiSelection, SelectedStructureInfo, SelectedStructureStatus};
use crate::dialog_ui::BaseOrCustomDialog;
use crate::render_context::RenderContext;
use itertools::Itertools;
use server::city::City;
use server::content::custom_actions::CustomActionType;
use server::content::persistent_events::{MultiRequest, SelectedStructure, Structure};
use server::cultural_influence::influence_culture_boost_cost;
use server::game::Game;
use server::playing_actions::PlayingActionType;

pub fn new_cultural_influence_dialog(
    game: &Game,
    player: usize,
    d: BaseOrCustomDialog,
) -> ActiveDialog {
    let a = game
        .players
        .iter()
        .flat_map(|p| {
            p.cities
                .iter()
                .flat_map(|city| {
                    structures(city)
                        .iter()
                        .map(|s| {
                            let info = influence_culture_boost_cost(game, player, s);
                            let mut tooltip = info.blockers.clone();
                            let status = if info.blockers.is_empty() {
                                if info.prevent_boost {
                                    tooltip.push("You cannot boost the dice roll".to_string());
                                    SelectedStructureStatus::Warn
                                } else {
                                    SelectedStructureStatus::Valid
                                }
                            } else {
                                SelectedStructureStatus::Invalid
                            };

                            let mut boost = info.range_boost_cost.default.amount();
                            if status == SelectedStructureStatus::Invalid {
                                boost = 0;
                            }
                            let label = (boost > 0).then_some(boost.to_string());
                            SelectedStructureInfo::new(
                                s.position,
                                s.structure.clone(),
                                status,
                                label.clone(),
                                (!tooltip.is_empty())
                                    .then_some(tooltip.join(", "))
                                    .or(label.map(|l| format!("Range boost cost: {l}"))),
                            )
                        })
                        .collect_vec()
                })
                .collect_vec()
        })
        .collect_vec();

    ActiveDialog::StructuresRequest(
        Some(d),
        MultiSelection::new(MultiRequest::new(
            a,
            0..=1,
            "Select structure to influence culture",
        )),
    )
}

fn structures(city: &City) -> Vec<SelectedStructure> {
    let mut structures: Vec<SelectedStructure> =
        vec![SelectedStructure::new(city.position, Structure::CityCenter)];
    for b in city.pieces.buildings(None) {
        structures.push(SelectedStructure::new(
            city.position,
            Structure::Building(b),
        ));
    }
    structures
}

pub fn can_play_influence_culture(rc: &RenderContext) -> bool {
    base_or_custom_available(
        rc,
        &PlayingActionType::InfluenceCultureAttempt,
        &CustomActionType::ArtsInfluenceCultureAttempt,
    )
}
