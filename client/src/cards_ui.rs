use crate::client_state::{ActiveDialog, NO_UPDATE, RenderResult, StateUpdate};
use crate::custom_phase_ui;
use crate::custom_phase_ui::MultiSelection;
use crate::dialog_ui::ok_button;
use crate::layout_ui::{bottom_centered_text, button_pressed, rect_from};
use crate::log_ui::MultilineText;
use crate::player_ui::get_combat;
use crate::render_context::RenderContext;
use crate::select_ui::HighlightType;
use custom_phase_ui::multi_select_tooltip_from_error;
use itertools::Itertools;
use macroquad::color::BLACK;
use macroquad::math::{Rect, Vec2, vec2};
use macroquad::prelude::{BEIGE, Color, GREEN, RED, YELLOW};
use server::action::Action;
use server::card::{HandCard, HandCardType, hand_cards, validate_card_selection};
use server::content::persistent_events::EventResponse;
use server::events::check_event_origin;
use server::objective_card::ObjectiveType;
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::wonder::{Wonder, WonderInfo};

pub(crate) struct HandCardObject {
    id: HandCard,
    name: String,
    pub description: MultilineText,
    color: Color,
}

impl HandCardObject {
    pub(crate) fn new(id: HandCard, color: Color, name: &str, description: MultilineText) -> Self {
        Self {
            id,
            name: name.chars().take(17).collect(),
            description,
            color,
        }
    }
}

const ACTION_CARD_COLOR: Color = RED;
const OBJECTIVE_CARD_COLOR: Color = BEIGE;
const WONDER_CARD_COLOR: Color = YELLOW;

pub(crate) struct SelectionInfo {
    selection: MultiSelection<HandCard>,
    show_names: Vec<(u8, String)>,
}

impl SelectionInfo {
    fn new(selection: MultiSelection<HandCard>, show_names: Vec<(u8, String)>) -> Self {
        Self {
            selection,
            show_names,
        }
    }
}

pub(crate) fn show_cards(rc: &RenderContext) -> RenderResult {
    let p = rc.shown_player;
    let cards = hand_cards(p, &HandCardType::get_all_and_public(), rc.game);
    let size = vec2(180., 30.);

    let selection = match &rc.state.active_dialog {
        ActiveDialog::HandCardsRequest(r) => Some(SelectionInfo::new(
            r.clone(),
            validate_card_selection(&r.request.choices, rc.game).unwrap_or_default(),
        )),
        _ => None,
    };

    let swap_cards = selection
        .iter()
        .flat_map(|s| {
            s.selection
                .request
                .choices
                .clone()
                .into_iter()
                .filter(|c| !cards.contains(c))
        })
        .collect_vec();

    draw_cards(rc, &cards, selection.as_ref(), size, -85.)?;
    draw_cards(rc, &swap_cards, selection.as_ref(), size, -310.)?;
    NO_UPDATE
}

fn draw_cards(
    rc: &RenderContext,
    cards: &Vec<HandCard>,
    selection: Option<&SelectionInfo>,
    size: Vec2,
    x_offset: f32,
) -> RenderResult {
    let screen = rc.state.screen_size;
    let mut y = (cards.len() as f32 * -size.y) / 2.;
    for card in cards {
        let pos = vec2(screen.x, screen.y / 2.0) + vec2(-size.x + x_offset, y);
        draw_card(rc, rect_from(pos, size), selection, card)?;

        y += size.y;
    }
    NO_UPDATE
}

fn draw_card(
    rc: &RenderContext,
    rect: Rect,
    selection: Option<&SelectionInfo>,
    card: &HandCard,
) -> RenderResult {
    let c = get_card_object(rc, card, selection);

    rc.draw_rectangle_with_text(rect, c.color, &c.name);
    let (thickness, border) = highlight(rc, &c, selection);
    rc.draw_rectangle_lines(rect, thickness, border);

    if button_pressed(rect, rc, &c.description, 150.) {
        if let Some(s) = selection {
            return StateUpdate::open_dialog(ActiveDialog::HandCardsRequest(
                s.selection.clone().toggle(c.id),
            ));
        }
        if can_play_card(rc, card) {
            return play_card(rc, card);
        }
    }
    NO_UPDATE
}

fn can_play_card(rc: &RenderContext, card: &HandCard) -> bool {
    match card {
        HandCard::ActionCard(id) => rc.can_play_action(&PlayingActionType::ActionCard(*id)),
        HandCard::Wonder(name) => rc.can_play_action(&PlayingActionType::WonderCard(*name)),
        HandCard::ObjectiveCard(_) => false,
    }
}

fn play_card(rc: &RenderContext, card: &HandCard) -> RenderResult {
    match card {
        HandCard::ActionCard(a) => StateUpdate::execute_with_confirm(
            vec![format!(
                "Play Action Card: {}",
                rc.game.cache.get_civil_card(*a).name
            )],
            Action::Playing(PlayingAction::ActionCard(*a)),
        ),
        HandCard::Wonder(name) => StateUpdate::execute_with_confirm(
            vec![format!("Play Wonder Card: {}", name.name())],
            Action::Playing(PlayingAction::WonderCard(*name)),
        ),
        HandCard::ObjectiveCard(_) => panic!("objective cards are not played as actions"),
    }
}

fn highlight(
    rc: &RenderContext,
    c: &HandCardObject,
    selection: Option<&SelectionInfo>,
) -> (f32, Color) {
    if let Some(s) = selection {
        if s.selection.selected.contains(&c.id) {
            return (8.0, GREEN);
        }
        if s.selection.request.choices.contains(&c.id) {
            return (8.0, HighlightType::Choices.color());
        }
    } else if can_play_card(rc, &c.id) {
        return (8.0, HighlightType::Choices.color());
    }
    (2.0, BLACK)
}

fn get_card_object(
    rc: &RenderContext,
    card: &HandCard,
    selection: Option<&SelectionInfo>,
) -> HandCardObject {
    match card {
        HandCard::ActionCard(a) if *a == 0 => HandCardObject::new(
            card.clone(),
            ACTION_CARD_COLOR,
            "Action Card",
            MultilineText::of(rc, "Hidden Action Card"),
        ),
        HandCard::ActionCard(id) => action_card_object(rc, *id),
        HandCard::ObjectiveCard(o) if *o == 0 => HandCardObject::new(
            card.clone(),
            OBJECTIVE_CARD_COLOR,
            "Objective Card",
            MultilineText::of(rc, "Hidden Objective Card"),
        ),
        HandCard::ObjectiveCard(id) => objective_card_object(rc, *id, selection),
        HandCard::Wonder(n) if n == &Wonder::Hidden => HandCardObject::new(
            card.clone(),
            WONDER_CARD_COLOR,
            "Wonder Card",
            MultilineText::of(rc, "Hidden Wonder Card"),
        ),
        HandCard::Wonder(name) => {
            let w = rc.game.cache.get_wonder(*name);
            HandCardObject::new(
                card.clone(),
                WONDER_CARD_COLOR,
                &w.name(),
                wonder_description(rc, w),
            )
        }
    }
}

pub(crate) fn wonder_description(rc: &RenderContext, w: &WonderInfo) -> MultilineText {
    let mut description = MultilineText::default();
    description.add(rc, &w.name());
    description.add(rc, w.description.as_str());
    description.add(rc, &format!("Cost: {}", w.cost));
    description.add(
        rc,
        &format!("Required advance: {}", w.required_advance.name(rc.game)),
    );
    description
}

pub(crate) fn action_card_object(rc: &RenderContext, id: u8) -> HandCardObject {
    let a = rc.game.cache.get_action_card(id);

    let name = match &a.tactics_card {
        Some(t) => {
            if get_combat(rc.game).is_some() {
                t.name.clone()
            } else {
                a.civil_card.name.clone()
            }
        }
        _ => a.civil_card.name.clone(),
    };

    let mut description = MultilineText::default();
    description.add(rc, &format!("Civil: {}", a.civil_card.name));
    let action_type = &a.civil_card.action_type;
    description.add(
        rc,
        if action_type.free {
            "As a free action"
        } else {
            "As a regular action"
        },
    );
    let cost = &action_type.payment_options(rc.shown_player, check_event_origin());
    if !cost.is_free() {
        description.add(rc, &format!("Cost: {cost}"));
    }
    description.add(rc, a.civil_card.description.as_str());
    if let Some(t) = &a.tactics_card {
        description.add(rc, &format!("Tactics: {}", t.name));
        description.add(
            rc,
            &format!(
                "Unit Types: {}",
                t.fighter_requirement
                    .iter()
                    .map(|f| format!("{f}"))
                    .join(", ")
            ),
        );
        description.add(
            rc,
            &format!(
                "Role: {}",
                match t.role_requirement {
                    None => "Attacker or Defender".to_string(),
                    Some(r) => format!("{r}"),
                }
            ),
        );
        description.add(
            rc,
            &format!(
                "Location: {}",
                match &t.location_requirement {
                    None => "Any".to_string(),
                    Some(l) => format!("{l}"),
                }
            ),
        );
        description.add(rc, t.description.as_str());
    }
    HandCardObject::new(
        HandCard::ActionCard(id),
        ACTION_CARD_COLOR,
        &name,
        description,
    )
}

pub(crate) fn objective_card_object(
    rc: &RenderContext,
    id: u8,
    selection: Option<&SelectionInfo>,
) -> HandCardObject {
    let card = rc.game.cache.get_objective_card(id);

    let mut description = MultilineText::default();
    for o in &card.objectives {
        description.add(rc, &format!("Objective: {}", o.name));
        description.add(
            rc,
            &format!(
                "Type: {}",
                match o.get_type() {
                    ObjectiveType::Instant => "Instant",
                    ObjectiveType::StatusPhase => "Status Phase",
                }
            ),
        );
        description.add(rc, o.description.as_str());
    }

    let name = selection
        .as_ref()
        .and_then(|s| {
            s.show_names
                .iter()
                .find_map(|(i, n)| (i == &id).then_some(n.clone()))
        })
        .unwrap_or_else(|| card.objectives.iter().map(|o| o.name.clone()).join(", "));

    HandCardObject::new(
        HandCard::ObjectiveCard(id),
        OBJECTIVE_CARD_COLOR,
        &name,
        description,
    )
}

pub(crate) fn select_cards_dialog(
    rc: &RenderContext,
    s: &MultiSelection<HandCard>,
) -> RenderResult {
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
        multi_select_tooltip_from_error(
            s,
            s.request.is_valid(&s.selected),
            "cards",
            validate_card_selection(&s.selected, rc.game).err(),
        ),
    ) {
        StateUpdate::response(EventResponse::SelectHandCards(s.selected.clone()))
    } else {
        NO_UPDATE
    }
}
