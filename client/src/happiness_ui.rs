use macroquad::math::vec2;
use macroquad::ui::root_ui;

use server::action::Action;
use server::city::City;
use server::game::Game;
use server::player::Player;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::ResourcePile;

use crate::dialog_ui::active_dialog_window;
use crate::ui_state::{can_play_action, ActiveDialog, IncreaseHappiness, StateUpdate};

pub fn add_increase_happiness(
    player: &Player,
    city: &City,
    pos: Position,
    increase_happiness: &IncreaseHappiness,
) -> IncreaseHappiness {
    let mut total_cost = increase_happiness.cost.clone();
    let new_steps = increase_happiness
        .steps
        .iter()
        .map(|(p, steps)| {
            let old_steps = *steps;
            if *p == pos {
                if let Some(r) = increase_happiness_steps(player, city, &total_cost, old_steps) {
                    total_cost = r.1;
                    return (*p, r.0);
                };
            }
            (*p, old_steps)
        })
        .collect();

    IncreaseHappiness::new(new_steps, total_cost)
}

fn increase_happiness_steps(
    player: &Player,
    city: &City,
    total_cost: &ResourcePile,
    old_steps: u32,
) -> Option<(u32, ResourcePile)> {
    if let Some(value) =
        increase_happiness_new_steps(player, city, total_cost, old_steps, old_steps + 1)
    {
        return Some(value);
    }
    if let Some(value) = increase_happiness_new_steps(player, city, total_cost, old_steps, 0) {
        return Some(value);
    }
    None
}

fn increase_happiness_new_steps(
    player: &Player,
    city: &City,
    total_cost: &ResourcePile,
    old_steps: u32,
    new_steps: u32,
) -> Option<(u32, ResourcePile)> {
    if let Some(new_cost) = city.increase_happiness_cost(new_steps) {
        let mut new_total = total_cost.clone();
        if old_steps > 0 {
            new_total -= city
                .increase_happiness_cost(old_steps)
                .expect("invalid steps");
        }
        new_total += new_cost;
        if player.resources.can_afford(&new_total) {
            return Some((new_steps, new_total));
        }
    }
    None
}

pub fn increase_happiness_menu(h: &IncreaseHappiness) -> StateUpdate {
    active_dialog_window("Increase Happiness", |ui| {
        ui.label(None, &format!("Cost: {:?}", h.cost));
        if ui.button(None, "Cancel") {
            return StateUpdate::Cancel;
        }
        if ui.button(None, "Confirm") {
            return StateUpdate::Execute(Action::Playing(PlayingAction::IncreaseHappiness {
                happiness_increases: h.steps.clone(),
            }));
        }
        StateUpdate::None
    })
}

pub fn show_increase_happiness(game: &Game, player_index: usize) -> StateUpdate {
    if can_play_action(game) && root_ui().button(vec2(1200., 60.), "Increase Happiness") {
        return StateUpdate::SetDialog(ActiveDialog::IncreaseHappiness(IncreaseHappiness::new(
            game.get_player(player_index)
                .cities
                .iter()
                .map(|c| (c.position, 0))
                .collect(),
            ResourcePile::empty(),
        )));
    }

    StateUpdate::None
}
