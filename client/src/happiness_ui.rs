use crate::action_buttons::base_or_custom_action;
use crate::client_state::{ActiveDialog, NO_UPDATE, RenderResult, StateUpdate};
use crate::dialog_ui::BaseOrCustomDialog;
use crate::payment_ui::{Payment, payment_dialog};
use crate::render_context::RenderContext;
use itertools::Itertools;
use server::action::Action;
use server::city::{City, MoodState};
use server::events::check_event_origin;
use server::game::Game;
use server::happiness::{
    IncreaseHappiness, available_happiness_actions, happiness_city_restriction, happiness_cost,
};
use server::player::CostTrigger;
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::position::Position;

#[derive(Clone, Debug)]
pub(crate) struct IncreaseHappinessConfig {
    pub steps: Vec<(Position, u8)>,
    pub payment: Payment<String>,
    pub custom: BaseOrCustomDialog,
    pub city_restriction: Option<Position>,
}

impl IncreaseHappinessConfig {
    pub fn new(rc: &RenderContext, custom: BaseOrCustomDialog) -> IncreaseHappinessConfig {
        let p = rc.shown_player;
        let steps = p.cities.iter().map(|c| (c.position, 0)).collect();
        let city_restriction = happiness_city_restriction(rc.shown_player, &custom.action_type);
        IncreaseHappinessConfig {
            steps,
            payment: Self::happiness_payment(rc, 0, &custom)
                .expect("Happiness payment should be available"),
            custom,
            city_restriction,
        }
    }

    fn happiness_payment(
        rc: &RenderContext,
        new_steps: u8,
        custom: &BaseOrCustomDialog,
    ) -> Option<Payment<String>> {
        let p = rc.shown_player;
        let c = happiness_cost(
            p.index,
            new_steps,
            CostTrigger::WithModifiers,
            &custom.action_type,
            rc.game,
            &check_event_origin(),
        )
        .cost;
        p.can_afford(&c).then_some(rc.new_payment(
            &c,
            "Increase happiness".to_string(),
            "Increase happiness",
            false,
        ))
    }
}

pub fn open_increase_happiness_dialog(
    rc: &RenderContext,
    actions: &[PlayingActionType],
    init: impl Fn(IncreaseHappinessConfig) -> IncreaseHappinessConfig,
) -> RenderResult {
    base_or_custom_action(rc, actions, "Increase happiness", |custom| {
        ActiveDialog::IncreaseHappiness(init(IncreaseHappinessConfig::new(rc, custom)))
    })
}

pub fn increase_happiness_click(
    rc: &RenderContext,
    pos: Position,
    h: &IncreaseHappinessConfig,
) -> RenderResult {
    if let Some(city) = rc.shown_player.try_get_city(pos) {
        add_increase_happiness(rc, city, h.clone()).map_or(NO_UPDATE, |increase_happiness| {
            StateUpdate::open_dialog(ActiveDialog::IncreaseHappiness(increase_happiness))
        })
    } else {
        NO_UPDATE
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

    IncreaseHappinessConfig::happiness_payment(rc, step_sum, &increase_happiness.custom).map(
        |payment| {
            increase_happiness.payment = payment;
            increase_happiness.steps = new_steps;
            increase_happiness
        },
    )
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
    can_afford_increase_happiness(rc, city, new_steps, action_type).then_some(new_steps)
}

#[must_use]
pub fn can_afford_increase_happiness(
    rc: &RenderContext,
    city: &City,
    steps: u8,
    action_type: &PlayingActionType,
) -> bool {
    let max_steps = 2 - city.mood_state.clone() as u8;
    if steps > max_steps {
        return false;
    }

    rc.shown_player.can_afford(
        &happiness_cost(
            rc.shown_player.index,
            steps * city.size() as u8,
            CostTrigger::WithModifiers,
            action_type,
            rc.game,
            &check_event_origin(),
        )
        .cost,
    )
}

#[must_use]
pub fn available_happiness_actions_for_city(
    game: &Game,
    player: usize,
    position: Position,
) -> Vec<PlayingActionType> {
    let p = game.player(player);
    let city = p.get_city(position);
    if city.can_activate() && city.mood_state != MoodState::Happy {
        available_happiness_actions(game, player)
            .into_iter()
            .filter(|action_type| {
                happiness_city_restriction(p, action_type).is_none_or(|r| r == position)
            })
            .collect_vec()
    } else {
        vec![]
    }
}

pub fn increase_happiness_menu(rc: &RenderContext, h: &IncreaseHappinessConfig) -> RenderResult {
    payment_dialog(
        rc,
        &h.payment,
        true,
        |payment| {
            ActiveDialog::IncreaseHappiness(IncreaseHappinessConfig {
                steps: h.steps.clone(),
                payment,
                custom: h.custom.clone(),
                city_restriction: h.city_restriction,
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
