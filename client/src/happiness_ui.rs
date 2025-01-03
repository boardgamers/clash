use server::action::Action;
use server::city::City;
use server::game::Game;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::ResourcePile;

use crate::client_state::{ActiveDialog, ShownPlayer, State, StateUpdate};
use crate::dialog_ui::{cancel_button, ok_button};

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

pub fn increase_happiness_click(
    game: &Game,
    player: &ShownPlayer,
    pos: Position,
    h: &IncreaseHappiness,
) -> StateUpdate {
    if let Some(city) = player.get(game).get_city(pos) {
        StateUpdate::SetDialog(ActiveDialog::IncreaseHappiness(add_increase_happiness(
            city, pos, h,
        )))
    } else {
        StateUpdate::None
    }
}

pub fn add_increase_happiness(
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
                if let Some(r) = increase_happiness_steps(city, &total_cost, old_steps) {
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
    city: &City,
    total_cost: &ResourcePile,
    old_steps: u32,
) -> Option<(u32, ResourcePile)> {
    if let Some(value) = increase_happiness_new_steps(city, total_cost, old_steps, old_steps + 1) {
        return Some(value);
    }
    if let Some(value) = increase_happiness_new_steps(city, total_cost, old_steps, 0) {
        return Some(value);
    }
    None
}

fn increase_happiness_new_steps(
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
        return Some((new_steps, new_total));
    }
    None
}

pub fn increase_happiness_menu(
    h: &IncreaseHappiness,
    player: &ShownPlayer,
    state: &State,
    game: &Game,
) -> StateUpdate {
    if ok_button(state, player.get(game).resources.can_afford(&h.cost)) {
        return StateUpdate::Execute(Action::Playing(PlayingAction::IncreaseHappiness {
            happiness_increases: h.steps.clone(),
        }));
    }
    if cancel_button(state) {
        return StateUpdate::Cancel;
    }
    StateUpdate::None
}

pub fn start_increase_happiness(game: &Game, player: &ShownPlayer) -> StateUpdate {
    StateUpdate::OpenDialog(ActiveDialog::IncreaseHappiness(IncreaseHappiness::new(
        player
            .get(game)
            .cities
            .iter()
            .map(|c| (c.position, 0))
            .collect(),
        ResourcePile::empty(),
    )))
}
