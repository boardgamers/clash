use crate::ui_state::{can_play_action, IncreaseHappiness, State};
use macroquad::color::BLACK;
use macroquad::math::vec2;
use macroquad::prelude::draw_text;
use macroquad::ui::root_ui;
use server::city::City;
use server::game::{Action, Game};
use server::player::Player;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::ResourcePile;

pub fn increase_happiness_click(
    player: &Player,
    city: &City,
    pos: &Position,
    increase_happiness: &IncreaseHappiness,
) -> IncreaseHappiness {
    let mut total_cost = increase_happiness.cost.clone();
    let new_steps = increase_happiness
        .steps
        .iter()
        .map(|(p, steps)| {
            let old_steps = *steps;
            if p == pos {
                if let Some(r) = increase_happiness_steps(player, city, &total_cost, old_steps) {
                    total_cost = r.1;
                    return (p.clone(), r.0);
                };
            }
            (p.clone(), old_steps)
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
        if player.resources().can_afford(&new_total) {
            return Some((new_steps, new_total));
        }
    }
    None
}

pub fn show_increase_happiness(game: &mut Game, player_index: usize, state: &mut State) {
    let y = 480.;
    if can_play_action(game)
        && root_ui().button(vec2(600., y), "Increase Happiness")
        && state.increase_happiness.is_none()
    {
        state.clear();
        state.increase_happiness = Some(IncreaseHappiness::new(
            game.get_player(player_index)
                .cities
                .iter()
                .map(|c| (c.position.clone(), 0))
                .collect(),
            ResourcePile::empty(),
        ));
    }
    if let Some(increase_happiness) = &state.increase_happiness {
        if root_ui().button(vec2(750., y), "Cancel") {
            state.clear();
        } else if increase_happiness.cost != ResourcePile::empty()
            && root_ui().button(vec2(800., y), "Confirm")
        {
            game.execute_action(
                Action::PlayingAction(PlayingAction::IncreaseHappiness {
                    happiness_increases: increase_happiness.steps.clone(),
                }),
                player_index,
            );
            state.clear();
        } else {
            draw_text(
                &format!("Cost: {}", increase_happiness.cost),
                600.,
                520.,
                20.,
                BLACK,
            );
        }
    }
}
