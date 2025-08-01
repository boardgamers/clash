use crate::action::lose_action;
use crate::city;
use crate::city::MoodState;
use crate::content::effects::PermanentEffect;
use crate::content::incidents::famine::kill_incident_units;
use crate::content::incidents::good_year::select_player_to_gain_settler;
use crate::content::persistent_events::{PaymentRequest, PositionRequest, UnitsRequest};
use crate::game::Game;
use crate::incident::{
    DecreaseMood, Incident, IncidentBaseEffect, IncidentBuilder, MoodModifier, decrease_mod_and_log,
};
use crate::payment::{PaymentConversion, PaymentConversionType};
use crate::player::Player;
use crate::player_events::IncidentTarget;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::status_phase::{
    ChangeGovernmentOption, add_change_government, can_change_government_for_free, get_status_phase,
};
use crate::unit::kill_units;
use crate::wonder::draw_public_wonder;
use itertools::Itertools;

pub(crate) fn civil_war_incidents() -> Vec<Incident> {
    vec![
        migration(34),
        migration(35),
        civil_war(36),
        civil_war(37),
        revolution(),
        uprising(),
        envoy(),
    ]
}

fn migration(id: u8) -> Incident {
    select_player_to_gain_settler(Incident::builder(
        id,
        "Migration",
        "Select a player to gain 1 settler in one of their cities. \
        Decrease the mood in one of your cities.",
        IncidentBaseEffect::GoldDeposits,
    ))
    .add_decrease_mood(
        IncidentTarget::ActivePlayer,
        MoodModifier::Decrease,
        |p, _game, _| DecreaseMood::new(city::non_angry_cites(p), 1),
    )
    .build()
}

fn civil_war(id: u8) -> Incident {
    Incident::builder(
        id,
        "Civil War",
        "Select a non-Happy city with an Infantry: \
            kill the Infantry and decrease the mood. If no such city exists, \
            select a city to decrease the mood.",
        IncidentBaseEffect::None,
    )
    .add_decrease_mood(
        IncidentTarget::ActivePlayer,
        MoodModifier::Decrease,
        |p, _game, _| {
            if non_happy_cites_with_infantry(p).is_empty() {
                DecreaseMood::new(city::non_angry_cites(p), 1)
            } else {
                DecreaseMood::none()
            }
        },
    )
    .add_incident_position_request(
        IncidentTarget::ActivePlayer,
        0,
        |game, p, i| {
            let p = p.get(game);
            let suffix = if !city::non_angry_cites(p).is_empty() && i.player.payment.is_empty() {
                " and decrease the mood"
            } else {
                ""
            };
            let choices = non_happy_cites_with_infantry(p);
            let needed = 1..=1;
            let description =
                &format!("Select a non-Happy city with an Infantry to kill the Infantry {suffix}");
            Some(PositionRequest::new(choices, needed, description))
        },
        |game, s, i| {
            let position = s.choice[0];
            let mood = game.get_any_city(position).mood_state.clone();
            if i.player.payment.is_empty() && !matches!(mood, MoodState::Angry) {
                decrease_mod_and_log(game, s, MoodModifier::Decrease);
            }
            let unit = game
                .player(s.player_index)
                .get_units(position)
                .iter()
                .filter(|u| u.is_infantry())
                .sorted_by_key(|u| u.movement_restrictions.len())
                .next_back()
                .expect("infantry should exist")
                .id;
            kill_units(game, &[unit], s.player_index, None, &s.origin);
        },
    )
    .build()
}

fn non_happy_cites_with_infantry(p: &Player) -> Vec<Position> {
    p.cities
        .iter()
        .filter(|c| {
            !matches!(c.mood_state, MoodState::Happy)
                && p.get_units(c.position).iter().any(|u| u.is_infantry())
        })
        .map(|c| c.position)
        .collect_vec()
}

fn revolution() -> Incident {
    let mut b = Incident::builder(
        38,
        "Revolution",
        "You may kill one of your Army units each to avoid the following steps: \
        Step 1: Lose one action (from your next turn if in Status phase). \
        Step 2: Change your government for free if possible.",
        IncidentBaseEffect::GoldDeposits,
    );
    b = kill_unit_for_revolution(
        b,
        11,
        "Kill a unit to avoid losing an action",
        |game, _player| can_lose_action(game),
    );
    b = b.add_simple_incident_listener(IncidentTarget::ActivePlayer, 2, |game, p, i| {
        if can_lose_action(game) && i.player.sacrifice == 0 {
            if get_status_phase(game).is_some() {
                p.log(game, "Lose an action for the next turn");
                game.permanent_effects
                    .push(PermanentEffect::RevolutionLoseAction(p.index));
            } else {
                lose_action(game, p);
            }
        }
    });
    b = kill_unit_for_revolution(
        b,
        10,
        "Kill a unit to avoid changing government",
        |game, player| can_change_government_for_free(player, game),
    );
    b = add_change_government(
        b,
        |event| &mut event.incident,
        ChangeGovernmentOption::FreeAndMandatory,
        |i, p, game| i.active_player == p && can_change_government_for_free(game.player(p), game),
        |_, _| {}, // don't need to pay
        |_| true,
    );
    b.build()
}

fn kill_unit_for_revolution(
    b: IncidentBuilder,
    priority: i32,
    description: &str,
    pred: impl Fn(&Game, &Player) -> bool + 'static + Clone + Sync + Send,
) -> IncidentBuilder {
    let description = description.to_string();
    b.add_incident_units_request(
        IncidentTarget::ActivePlayer,
        priority,
        move |game, p, i| {
            i.player.sacrifice = 0;
            let units = p
                .get(game)
                .units
                .iter()
                .filter(|u| u.is_army_unit())
                .map(|u| u.id)
                .collect_vec();
            Some(UnitsRequest::new(
                p.index,
                if pred(game, p.get(game)) {
                    units
                } else {
                    vec![]
                },
                0..=1,
                &description,
            ))
        },
        |game, s, i| {
            kill_incident_units(game, s);
            if !s.choice.is_empty() {
                i.player.sacrifice = 1;
            }
        },
    )
}

fn can_lose_action(game: &Game) -> bool {
    get_status_phase(game).is_some() || game.actions_left > 0
}

#[allow(clippy::float_cmp)]
fn uprising() -> Incident {
    Incident::builder(
        39,
        "Uprising",
        "Pay 1-4 mood or culture tokens if possible. \
                      Each token is worth half a point at the end of the game.",
        IncidentBaseEffect::None,
    )
    .add_incident_payment_request(
        IncidentTarget::ActivePlayer,
        0,
        |game, p, _incident| {
            let player = p.get(game);
            let mut cost = p.payment_options().tokens(player, 4);
            cost.conversions.push(PaymentConversion::resource_options(
                vec![
                    ResourcePile::mood_tokens(1),
                    ResourcePile::culture_tokens(1),
                ],
                ResourcePile::empty(),
                PaymentConversionType::MayOverpay(3),
            ));
            player
                .can_afford(&cost)
                .then_some(vec![PaymentRequest::mandatory(
                    cost,
                    "Pay 1-4 mood or culture tokens",
                )])
        },
        |game, s, _| {
            let player = game.player_mut(s.player_index);
            let pile = &s.choice[0];
            let v = pile.amount() as f32 / 2_f32;
            player.gain_event_victory_points(v, &s.origin);
            s.log(
                game,
                &format!(
                    "Gain {} victory point{}",
                    v,
                    if v == 1.0 { "" } else { "s" }
                ),
            );
        },
    )
    .build()
}

fn envoy() -> Incident {
    Incident::builder(
        40,
        "Envoy",
        "Gain 1 idea and 1 culture token. \
        Select another player to gain 1 culture token. \
        Draw the top card from the wonder deck. \
        This card can be taken by anyone instead of drawing from the wonder pile.",
        IncidentBaseEffect::BarbariansMove,
    )
    .add_simple_incident_listener(IncidentTarget::ActivePlayer, 1, |game, player, _| {
        player.gain_resources(
            game,
            ResourcePile::ideas(1) + ResourcePile::culture_tokens(1),
        );

        draw_public_wonder(game, player);
    })
    .add_incident_player_request(
        IncidentTarget::ActivePlayer,
        "Select a player to gain 1 culture token",
        |_p, _, _| true,
        0,
        |game, s, _| {
            s.other_player(s.choice, game)
                .gain_resources(game, ResourcePile::culture_tokens(1));
        },
    )
    .build()
}
