use crate::city_ui::building_name;
use crate::dialog_ui::active_dialog_window;
use crate::ui_state::{CityMenu, StateUpdate};
use macroquad::ui::Ui;
use server::action::Action;
use server::city_pieces::Building;
use server::game::{CulturalInfluenceResolution, Game};
use server::playing_actions::PlayingAction;
use server::position::Position;

pub fn add_influence_button(
    game: &Game,
    menu: &CityMenu,
    ui: &mut Ui,
    closest_city_pos: Position,
    building: &Building,
    building_name: &str,
) -> StateUpdate {
    if !menu.get_city(game).pieces.can_add_building(building) {
        let start_position = if menu.is_city_owner() {
            menu.city_position
        } else {
            closest_city_pos
        };
        if let Some(cost) = game.influence_culture_boost_cost(
            menu.player_index,
            start_position,
            menu.city_owner_index,
            menu.city_position,
            building,
        ) {
            if ui.button(
                None,
                format!("Attempt Influence {building_name} for {cost}"),
            ) {
                return StateUpdate::Execute(Action::Playing(
                    PlayingAction::InfluenceCultureAttempt {
                        starting_city_position: start_position,
                        target_player_index: menu.city_owner_index,
                        target_city_position: menu.city_position,
                        city_piece: building.clone(),
                    },
                ));
            }
        }
    }
    StateUpdate::None
}

pub fn closest_city(game: &Game, menu: &CityMenu) -> Position {
    menu.get_player(game)
        .cities
        .iter()
        .min_by_key(|c| c.position.distance(menu.city_position))
        .unwrap()
        .position
}

pub fn cultural_influence_resolution_dialog(c: &CulturalInfluenceResolution) -> StateUpdate {
    active_dialog_window("Cultural Influence Resolution", |ui| {
        if ui.button(
            None,
            format!(
                "Pay {} culture tokens to influence {}",
                c.roll_boost_cost,
                building_name(&c.city_piece)
            ),
        ) {
            StateUpdate::Execute(Action::CulturalInfluenceResolution(true))
        } else if ui.button(None, "Decline") {
            StateUpdate::Execute(Action::CulturalInfluenceResolution(false))
        } else {
            StateUpdate::None
        }
    })
}
