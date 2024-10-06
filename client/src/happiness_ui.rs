use server::action::Action;
use server::city::City;
use server::game::Game;
use server::player::Player;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::ResourcePile;

use crate::client_state::{ActiveDialog, ShownPlayer, StateUpdate};
use crate::dialog_ui::active_dialog_window;
use crate::layout_ui::{cancel_pos, ok_pos};

#[derive(Clone)]
pub struct IncreaseHappiness {
    pub steps: Vec<(Position, u32)>,
    pub cost: ResourcePile,
}

impl IncreaseHappiness {
    pub fn new(steps: Vec<(Position, u32)>, cost: ResourcePile) -> IncreaseHappiness {
        IncreaseHappiness { steps, cost }
    }
}

pub fn increase_happiness_dialog(
    game: &Game,
    player: &ShownPlayer,
    pos: Position,
    h: &IncreaseHappiness,
) -> StateUpdate {
    if let Some(city) = player.get(game).get_city(pos) {
        StateUpdate::SetDialog(ActiveDialog::IncreaseHappiness(add_increase_happiness(
            player.get(game),
            city,
            pos,
            h,
        )))
    } else {
        StateUpdate::None
    }
}

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

pub fn increase_happiness_menu(h: &IncreaseHappiness, player: &ShownPlayer) -> StateUpdate {
    active_dialog_window(player, "Increase Happiness", |ui| {
        ui.label(None, &format!("Cost: {:?}", h.cost));
        if ui.button(cancel_pos(player), "Cancel") {
            return StateUpdate::Cancel;
        }
        if ui.button(ok_pos(player), "Confirm") {
            return StateUpdate::Execute(Action::Playing(PlayingAction::IncreaseHappiness {
                happiness_increases: h.steps.clone(),
            }));
        }
        StateUpdate::None
    })
}

pub fn start_increase_happiness(game: &Game, player: &ShownPlayer) -> StateUpdate {
    StateUpdate::OpenDialog(ActiveDialog::IncreaseHappiness(IncreaseHappiness::new(
        game.get_player(player.index)
            .cities
            .iter()
            .map(|c| (c.position, 0))
            .collect(),
        ResourcePile::empty(),
    )))
}
