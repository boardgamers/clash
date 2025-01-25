use crate::action_buttons::{base_or_custom_action, base_or_custom_available};
use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::{
    cancel_button, ok_button, BaseOrCustomAction, BaseOrCustomDialog, OkTooltip,
};
use crate::render_context::RenderContext;
use crate::resource_ui::show_resource_pile;
use server::action::Action;
use server::city::City;
use server::content::custom_actions::{CustomAction, CustomActionType};
use server::player::Player;
use server::playing_actions::{IncreaseHappiness, PlayingAction, PlayingActionType};
use server::position::Position;
use server::resource::ResourceType;
use server::resource_pile::ResourcePile;

#[derive(Clone)]
pub struct IncreaseHappinessConfig {
    pub steps: Vec<(Position, u32)>,
    pub cost: ResourcePile,
    pub custom: BaseOrCustomDialog,
}

impl IncreaseHappinessConfig {
    pub fn new(p: &Player, custom: BaseOrCustomDialog) -> IncreaseHappinessConfig {
        let steps = p.cities.iter().map(|c| (c.position, 0)).collect();
        IncreaseHappinessConfig {
            steps,
            cost: ResourcePile::empty(),
            custom,
        }
    }
}

pub fn can_play_increase_happiness(rc: &RenderContext) -> bool {
    base_or_custom_available(
        rc,
        PlayingActionType::IncreaseHappiness,
        &CustomActionType::VotingIncreaseHappiness,
    )
}

pub fn open_increase_happiness_dialog(
    rc: &RenderContext,
    init: impl Fn(IncreaseHappinessConfig) -> IncreaseHappinessConfig,
) -> StateUpdate {
    base_or_custom_action(
        rc,
        PlayingActionType::IncreaseHappiness,
        "Increase happiness",
        &[("Voting", CustomActionType::VotingIncreaseHappiness)],
        |custom| {
            ActiveDialog::IncreaseHappiness(init(IncreaseHappinessConfig::new(
                rc.shown_player,
                custom,
            )))
        },
    )
}

pub fn increase_happiness_click(
    rc: &RenderContext,
    pos: Position,
    h: &IncreaseHappinessConfig,
) -> StateUpdate {
    if let Some(city) = rc.shown_player.get_city(pos) {
        StateUpdate::OpenDialog(ActiveDialog::IncreaseHappiness(add_increase_happiness(
            city, h,
        )))
    } else {
        StateUpdate::None
    }
}

pub fn add_increase_happiness(
    city: &City,
    increase_happiness: &IncreaseHappinessConfig,
) -> IncreaseHappinessConfig {
    let mut total_cost = increase_happiness.cost.clone();
    let new_steps = increase_happiness
        .steps
        .iter()
        .map(|(p, steps)| {
            let old_steps = *steps;
            if *p == city.position {
                if let Some(r) = increase_happiness_steps(city, &total_cost, old_steps) {
                    total_cost = r.1;
                    return (*p, r.0);
                };
            }
            (*p, old_steps)
        })
        .collect();

    IncreaseHappinessConfig {
        steps: new_steps,
        cost: total_cost,
        custom: increase_happiness.custom.clone(),
    }
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

pub fn increase_happiness_menu(rc: &RenderContext, h: &IncreaseHappinessConfig) -> StateUpdate {
    show_resource_pile(rc, &h.cost, &[ResourceType::MoodTokens]);

    let tooltip = if rc.shown_player.can_afford_resources(&h.cost) {
        OkTooltip::Valid("Increase happiness".to_string())
    } else {
        OkTooltip::Invalid("Not enough resources".to_string())
    };
    if ok_button(rc, tooltip) {
        let i = IncreaseHappiness {
            happiness_increases: h.steps.clone(),
        };
        let action = match &h.custom.custom {
            BaseOrCustomAction::Base => PlayingAction::IncreaseHappiness(i),
            BaseOrCustomAction::Custom { .. } => {
                PlayingAction::Custom(CustomAction::VotingIncreaseHappiness(i))
            }
        };
        return StateUpdate::Execute(Action::Playing(action));
    }
    if cancel_button(rc) {
        return StateUpdate::Cancel;
    }
    StateUpdate::None
}
