use crate::client_state::{ActiveDialog, StateUpdate};
use crate::custom_phase_ui::MultiSelection;
use crate::dialog_ui::ok_button;
use crate::layout_ui::{bottom_centered_text, left_mouse_button_pressed_in_rect};
use crate::render_context::RenderContext;
use crate::select_ui::HighlightType;
use crate::tooltip::show_tooltip_for_rect;
use macroquad::color::BLACK;
use macroquad::math::{vec2, Rect};
use macroquad::prelude::{draw_rectangle, draw_rectangle_lines, Color, GREEN, RED, YELLOW};
use server::action::Action;
use server::card::{hand_cards, HandCard, HandCardType};
use server::content::action_cards::{get_action_card, get_civil_card};
use server::content::custom_phase_actions::EventResponse;
use server::content::wonders::get_wonder;
use server::player::Player;
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::tactics_card::CombatRole;
use server::wonder::Wonder;

pub struct HandCardObject {
    id: HandCard,
    name: String,
    description: Vec<String>,
    color: Color,
}

impl HandCardObject {
    pub fn new(id: HandCard, color: Color, name: String, description: Vec<String>) -> Self {
        Self {
            id,
            name,
            description,
            color,
        }
    }
}

#[must_use]
pub fn wonder_cards(player: &Player) -> Vec<Wonder> {
    player.wonder_cards.iter().map(|n| get_wonder(n)).collect()
}

const ACTION_CARD_COLOR: Color = RED;
const WONDER_CARD_COLOR: Color = YELLOW;

pub(crate) fn show_cards(rc: &RenderContext) -> StateUpdate {
    let p = rc.shown_player;
    let cards = hand_cards(p, &HandCardType::get_all());
    let size = vec2(180., 30.);

    let selection = if let ActiveDialog::HandCardsRequest(r) = &rc.state.active_dialog {
        Some(r)
    } else {
        None
    };

    for pass in 0..2 {
        let mut y = (cards.len() as f32 * -size.y) / 2.;
        for card in &cards {
            let screen = rc.state.screen_size;
            let pos = vec2(screen.x, screen.y / 2.0) + vec2(-size.x, y);

            let c = get_card_object(card);

            if pass == 0 {
                draw_rectangle(pos.x, pos.y, size.x, size.y, c.color);
                let (thickness, border) = highlight(rc, &c, selection);
                draw_rectangle_lines(pos.x, pos.y, size.x, size.y, thickness, border);

                rc.state.draw_text(&c.name, pos.x + 10., pos.y + 22.);
            } else {
                let rect = Rect::new(pos.x, pos.y, size.x, size.y);

                // tooltip should be shown on top of everything
                show_tooltip_for_rect(rc, &c.description, rect);

                if left_mouse_button_pressed_in_rect(rect, rc) {
                    if let Some(s) = selection {
                        return StateUpdate::OpenDialog(ActiveDialog::HandCardsRequest(
                            s.clone().toggle(c.id),
                        ));
                    }
                    if can_play_card(rc, card) {
                        return play_card(card);
                    }
                }
            }

            y += size.y;
        }
    }
    StateUpdate::None
}

fn can_play_card(rc: &RenderContext, card: &HandCard) -> bool {
    if rc.can_control_shown_player() {
        if let HandCard::ActionCard(id) = card {
            return rc.can_play_action(&PlayingActionType::ActionCard(*id));
        }
    }
    false
}

fn play_card(card: &HandCard) -> StateUpdate {
    match card {
        HandCard::ActionCard(a) => StateUpdate::execute_with_confirm(
            vec![format!("Play Action Card: {}", get_civil_card(*a).name)],
            Action::Playing(PlayingAction::ActionCard(*a)),
        ),
        HandCard::Wonder(_) => panic!("wonders are played in the construct menu"),
    }
}

fn highlight(
    rc: &RenderContext,
    c: &HandCardObject,
    selection: Option<&MultiSelection<HandCard>>,
) -> (f32, Color) {
    if let Some(s) = selection {
        if s.selected.contains(&c.id) {
            return (8.0, GREEN);
        }
        if s.request.choices.contains(&c.id) {
            return (8.0, HighlightType::Choices.color());
        }
    } else if can_play_card(rc, &c.id) {
        return (8.0, HighlightType::Choices.color());
    }
    (2.0, BLACK)
}

fn get_card_object(card: &HandCard) -> HandCardObject {
    match card {
        HandCard::ActionCard(a) if *a == 0 => HandCardObject::new(
            card.clone(),
            ACTION_CARD_COLOR,
            "Action Card".to_string(),
            vec!["Hidden Action Card".to_string()],
        ),
        HandCard::ActionCard(id) => {
            let a = get_action_card(*id);
            let mut description = vec![];
            let action_type = a.civil_card.action_type;
            description.push(
                if action_type.free {
                    "As a free action"
                } else {
                    "As a regular action"
                }
                .to_string(),
            );
            let cost = action_type.cost;
            if !cost.is_empty() {
                description.push(format!("Cost: {cost}"));
            }
            description.push(a.civil_card.description);
            if let Some(t) = a.tactics_card {
                description.extend(vec![
                    format!("Tactics: {}", t.name),
                    format!("Unit Type: {:?}", t.fighter_requirement),
                    format!(
                        "Role: {:?}",
                        match t.role_requirement {
                            None => "None".to_string(),
                            Some(r) => match r {
                                CombatRole::Attacker => "Attacker".to_string(),
                                CombatRole::Defender => "Defender".to_string(),
                            },
                        }
                    ),
                    t.description.clone(),
                ]);
            }
            HandCardObject::new(
                card.clone(),
                ACTION_CARD_COLOR,
                a.civil_card.name.clone(),
                description,
            )
        }
        HandCard::Wonder(n) if n.is_empty() => HandCardObject::new(
            card.clone(),
            WONDER_CARD_COLOR,
            "Wonder Card".to_string(),
            vec!["Hidden Wonder Card".to_string()],
        ),
        HandCard::Wonder(name) => {
            let w = get_wonder(name);
            HandCardObject::new(
                card.clone(),
                WONDER_CARD_COLOR,
                w.name.clone(),
                vec![
                    w.description.clone(),
                    format!("Cost: {}", w.cost.to_string()),
                    format!("Required advances: {}", w.required_advances.join(", ")),
                ],
            )
        }
    }
}

pub fn select_cards_dialog(rc: &RenderContext, s: &MultiSelection<HandCard>) -> StateUpdate {
    bottom_centered_text(
        rc,
        format!(
            "{}: {} cards selected",
            s.request.description,
            s.selected.len()
        )
        .as_str(),
    );

    if ok_button(
        rc,
        crate::custom_phase_ui::multi_select_tooltip(s, s.request.is_valid(&s.selected), "cards"),
    ) {
        StateUpdate::response(EventResponse::SelectHandCards(s.selected.clone()))
    } else {
        StateUpdate::None
    }
}
