use crate::action_buttons::base_or_custom_available;
use crate::client_state::ActiveDialog;
use crate::custom_phase_ui::{MultiSelection, SelectedStructureInfo};
use crate::dialog_ui::BaseOrCustomDialog;
use crate::render_context::RenderContext;
use itertools::Itertools;
use server::city::City;
use server::content::custom_actions::CustomActionType;
use server::content::custom_phase_actions::{MultiRequest, SelectedStructure, Structure};
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
                        .filter_map(|s| {
                            let info = influence_culture_boost_cost(game, player, s);
                            if info.blockers.is_empty() {
                                Some(SelectedStructureInfo::new(
                                    s.0,
                                    s.1.clone(),
                                    false,          //todo
                                    String::new(), //todo
                                    String::new(), //todo
                                ))
                            } else {
                                None
                            }
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
    let mut structures: Vec<SelectedStructure> = vec![(city.position, Structure::CityCenter)];
    for b in city.pieces.buildings(None) {
        structures.push((city.position, Structure::Building(b)));
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
