use crate::action_buttons::base_or_custom_action;
use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::BaseOrCustomDialog;
use crate::payment_ui::{Payment, payment_dialog};
use crate::render_context::RenderContext;
use server::action::Action;
use server::city::{City, MoodState};
use server::game::Game;
use server::happiness::{available_happiness_actions, happiness_cost};
use server::player::{CostTrigger, Player};
use server::player_events::CostInfo;
use server::playing_actions::{IncreaseHappiness, PlayingAction, PlayingActionType};
use server::position::Position;

#[derive(Clone)]
pub struct IncreaseHappinessConfig {
    pub steps: Vec<(Position, u8)>,
    pub payment: Payment<String>,
    pub custom: BaseOrCustomDialog,
}

impl IncreaseHappinessConfig {
    pub fn new(p: &Player, custom: BaseOrCustomDialog) -> IncreaseHappinessConfig {
        let steps = p.cities.iter().map(|c| (c.position, 0)).collect();
        IncreaseHappinessConfig {
            steps,
            payment: Self::happiness_payment(p, 0, &custom)
                .expect("Happiness payment should be available"),
            custom,
        }
    }

    fn happiness_payment(
        p: &Player,
        new_steps: u8,
        custom: &BaseOrCustomDialog,
    ) -> Option<Payment<String>> {
        let c = happiness_cost(p, new_steps, CostTrigger::WithModifiers).cost;
        c.can_afford(&custom.action_type.remaining_resources(p))
            .then_some(Payment::new(
                &c,
                &custom.action_type.remaining_resources(p),
                "Increase happiness".to_string(),
                "Increase happiness",
                false,
            ))
    }
}

pub fn open_increase_happiness_dialog(
    rc: &RenderContext,
    actions: Vec<PlayingActionType>,
    init: impl Fn(IncreaseHappinessConfig) -> IncreaseHappinessConfig,
) -> StateUpdate {
    base_or_custom_action(rc, actions, "Increase happiness", |custom| {
        ActiveDialog::IncreaseHappiness(init(IncreaseHappinessConfig::new(rc.shown_player, custom)))
    })
}

pub fn increase_happiness_click(
    rc: &RenderContext,
    pos: Position,
    h: &IncreaseHappinessConfig,
) -> StateUpdate {
    if let Some(city) = rc.shown_player.try_get_city(pos) {
        add_increase_happiness(rc, city, h.clone()).map_or(
            StateUpdate::None,
            |increase_happiness| {
                StateUpdate::OpenDialog(ActiveDialog::IncreaseHappiness(increase_happiness))
            },
        )
    } else {
        StateUpdate::None
    }
}

pub fn add_increase_happiness(
    rc: &RenderContext,
    city: &City,
    mut increase_happiness: IncreaseHappinessConfig,
) -> Option<IncreaseHappinessConfig> {
    let new_steps: Vec<(Position, u8)> = increase_happiness
        .steps
        .iter()
        .map(|(p, steps)| {
            let old_steps = *steps;
            if *p == city.position {
                if let Some(r) = increase_happiness_steps(
                    rc,
                    city,
                    old_steps,
                    &increase_happiness.custom.action_type,
                ) {
                    return (*p, r);
                }
            }
            (*p, old_steps)
        })
        .collect();

    let step_sum = new_steps
        .iter()
        .map(|(p, steps)| rc.shown_player.get_city(*p).size() as u8 * steps)
        .sum::<u8>();

    IncreaseHappinessConfig::happiness_payment(
        rc.shown_player,
        step_sum,
        &increase_happiness.custom,
    )
    .map(|payment| {
        increase_happiness.payment = payment;
        increase_happiness.steps = new_steps;
        increase_happiness
    })
}

fn increase_happiness_steps(
    rc: &RenderContext,
    city: &City,
    old_steps: u8,
    action_type: &PlayingActionType,
) -> Option<u8> {
    if let Some(value) = increase_happiness_new_steps(rc, city, old_steps + 1, action_type) {
        return Some(value);
    }
    if let Some(value) = increase_happiness_new_steps(rc, city, 0, action_type) {
        return Some(value);
    }
    None
}

fn increase_happiness_new_steps(
    rc: &RenderContext,
    city: &City,
    new_steps: u8,
    action_type: &PlayingActionType,
) -> Option<u8> {
    increase_happiness_cost(rc.shown_player, city, new_steps, action_type).map(|_| new_steps)
}

#[must_use]
pub fn increase_happiness_cost(
    player: &Player,
    city: &City,
    steps: u8,
    action_type: &PlayingActionType,
) -> Option<CostInfo> {
    let total_cost = happiness_cost(
        player,
        steps * city.size() as u8,
        CostTrigger::WithModifiers,
    );
    let max_steps = 2 - city.mood_state.clone() as u8;
    (total_cost
        .cost
        .can_afford(&action_type.remaining_resources(player))
        && steps <= max_steps)
        .then_some(total_cost)
}

#[must_use]
pub fn available_happiness_actions_for_city(
    game: &Game,
    player: usize,
    position: Position,
) -> Vec<PlayingActionType> {
    let city = game.player(player).get_city(position);
    if city.can_activate() && city.mood_state != MoodState::Happy {
        available_happiness_actions(game, player)
    } else {
        vec![]
    }
}

pub fn increase_happiness_menu(rc: &RenderContext, h: &IncreaseHappinessConfig) -> StateUpdate {
    payment_dialog(
        rc,
        &h.payment,
        true,
        |payment| {
            ActiveDialog::IncreaseHappiness(IncreaseHappinessConfig {
                steps: h.steps.clone(),
                payment,
                custom: h.custom.clone(),
            })
        },
        |payment| {
            let include_happiness =
                IncreaseHappiness::new(h.steps.clone(), payment, h.custom.action_type.clone());
            StateUpdate::execute(Action::Playing(PlayingAction::IncreaseHappiness(
                include_happiness,
            )))
        },
    )
}
