use crate::city::MoodState;
use crate::content::custom_phase_actions::{new_position_request, PaymentRequest, UnitsRequest};
use crate::content::incidents_famine::{decrease_mod_and_log, decrease_mood_incident_city};
use crate::content::incidents_population_boom::select_player_to_gain_settler;
use crate::game::{Game, GameState};
use crate::incident::{Incident, IncidentBaseEffect, IncidentBuilder, PermanentIncidentEffect};
use crate::payment::{PaymentConversion, PaymentConversionType, PaymentOptions};
use crate::player::Player;
use crate::player_events::IncidentTarget;
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::status_phase::{add_change_government, can_change_government_for_free};
use crate::unit::UnitType;
use crate::wonder::draw_wonder_from_pile;
use itertools::Itertools;

pub(crate) fn civil_wars() -> Vec<Incident> {
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
    let mut b = Incident::builder(
        id,
        "Migration",
        "Select a player to gain 1 settler in one of their cities. Decrease the mood in one of your cities.",
        IncidentBaseEffect::GoldDeposits,
    );
    b = select_player_to_gain_settler(b);
    b = b.add_myths_payment(IncidentTarget::ActivePlayer, |_game, p| {
        u32::from(!non_angry_cites(p).is_empty())
    });
    decrease_mood_incident_city(b, IncidentTarget::ActivePlayer, 0, |game, player_index| {
        (non_angry_cites(game.get_player(player_index)), 1)
    })
    .build()
}

fn non_angry_cites(p: &Player) -> Vec<Position> {
    p.cities
        .iter()
        .filter(|c| !matches!(c.mood_state, MoodState::Angry))
        .map(|c| c.position)
        .collect_vec()
}

fn civil_war(id: u8) -> Incident {
    let mut b = Incident::builder(
        id,
        "Civil War",
        "Select a non-Happy city with an Infantry: kill the Infantry and decrease the mood. If no such city exists, select a city to decrease the mood.",
        IncidentBaseEffect::None,
    );
    b = b.add_myths_payment(IncidentTarget::ActivePlayer, |_game, p| {
        u32::from(!non_angry_cites(p).is_empty())
    });
    b = decrease_mood_incident_city(b, IncidentTarget::ActivePlayer, 1, |game, player_index| {
        if !game.current_event_player().payment.is_empty() {
            return (vec![], 0);
        }
        if non_happy_cites_with_infantry(game.get_player(player_index)).is_empty() {
            return (non_angry_cites(game.get_player(player_index)), 1);
        }
        (vec![], 0)
    });
    b = b.add_incident_position_request(
        IncidentTarget::ActivePlayer,
        0,
        |game, player_index, _incident| {
            let p = game.get_player(player_index);
            let suffix = if !non_angry_cites(p).is_empty()
                && game.current_event_player().payment.is_empty()
            {
                " and decrease the mood"
            } else {
                ""
            };
            Some(new_position_request(
                non_happy_cites_with_infantry(p),
                1..=1,
                &format!("Select a non-Happy city with an Infantry to kill the Infantry {suffix}"),
            ))
        },
        |game, s| {
            let position = s.choice[0];
            let mood = game.get_any_city(position).mood_state.clone();
            if game.current_event_player().payment.is_empty() && !matches!(mood, MoodState::Angry) {
                decrease_mod_and_log(game, s);
            }
            let unit = game
                .get_player(s.player_index)
                .get_units(position)
                .iter()
                .filter(|u| matches!(u.unit_type, UnitType::Infantry))
                .sorted_by_key(|u| u.movement_restrictions.len())
                .next_back()
                .expect("infantry should exist")
                .id;
            game.add_info_log_item(&format!(
                "{} killed an Infantry in {}",
                s.player_name, position
            ));
            game.kill_unit(unit, s.player_index, None);
        },
    );
    b.build()
}

fn non_happy_cites_with_infantry(p: &Player) -> Vec<Position> {
    p.cities
        .iter()
        .filter(|c| {
            !matches!(c.mood_state, MoodState::Happy)
                && p.get_units(c.position)
                    .iter()
                    .any(|u| matches!(u.unit_type, UnitType::Infantry))
        })
        .map(|c| c.position)
        .collect_vec()
}

fn revolution() -> Incident {
    let mut b = Incident::builder(
        38,
        "Revolution",
        "You may kill one of your Army units each to avoid the following steps: Step 1: Loose one action (from your next turn if in Status phase). Step 2: Change your government for free if possible.",
        IncidentBaseEffect::GoldDeposits,
    );
    b = kill_unit_for_revolution(
        b,
        3,
        "Kill a unit to avoid losing an action",
        |game, _player| can_loose_action(game),
    );
    b = b.add_simple_incident_listener(IncidentTarget::ActivePlayer, 2, |game, player, _, _| {
        if can_loose_action(game) && game.current_event_player().sacrifice == 0 {
            loose_action(game, player);
        }
    });
    b = kill_unit_for_revolution(
        b,
        1,
        "Kill a unit to avoid changing government",
        |_game, player| can_change_government_for_free(player),
    );
    b = add_change_government(
        b,
        |event| &mut event.on_incident,
        false,
        ResourcePile::empty(),
    );
    b.build()
}

fn kill_unit_for_revolution(
    b: IncidentBuilder,
    priority: i32,
    description: &str,
    pred: impl Fn(&Game, &Player) -> bool + 'static + Clone,
) -> IncidentBuilder {
    let description = description.to_string();
    b.add_incident_units_request(
        IncidentTarget::ActivePlayer,
        priority,
        move |game, player_index, _incident| {
            game.current_event_mut().player.sacrifice = 0;
            let units = game
                .get_player(player_index)
                .units
                .iter()
                .filter(|u| u.unit_type.is_army_unit())
                .map(|u| u.id)
                .collect_vec();
            Some(UnitsRequest::new(
                player_index,
                if pred(game, game.get_player(player_index)) {
                    units
                } else {
                    vec![]
                },
                0..=1,
                &description,
            ))
        },
        |game, s| {
            if s.choice.is_empty() {
                game.add_info_log_item(&format!("{} did not kill an Army unit", s.player_name));
                return;
            }
            game.add_info_log_item(&format!("{} killed an Army unit", s.player_name));
            game.kill_unit(s.choice[0], s.player_index, None);
            game.current_event_mut().player.sacrifice = 1;
        },
    )
}

fn can_loose_action(game: &Game) -> bool {
    match game.state() {
        GameState::StatusPhase(_) => true,
        _ => game.actions_left > 0,
    }
}

fn loose_action(game: &mut Game, player: usize) {
    let name = game.player_name(player);
    if let GameState::StatusPhase(_) = game.state() {
        game.add_info_log_item(&format!("{name} lost an action for the next turn"));
        game.permanent_incident_effects
            .push(PermanentIncidentEffect::LooseAction(player));
    } else {
        game.add_info_log_item(&format!("{name} lost an action"));
        game.actions_left -= 1;
    };
}

#[allow(clippy::float_cmp)]
fn uprising() -> Incident {
    Incident::builder(39, "Uprising", "Pay 1-4 mood or culture tokens if possible. Each token is worth half a point at the end of the game.", IncidentBaseEffect::None)
        .add_incident_payment_request(
            IncidentTarget::ActivePlayer,
            0,
            |game, player_index, _incident| {
                let player = game.get_player(player_index);
                let mut cost = PaymentOptions::sum(4, &[ResourceType::MoodTokens, ResourceType::CultureTokens]);
                cost.conversions.push(PaymentConversion::new(
                    vec![ResourcePile::mood_tokens(1), ResourcePile::culture_tokens(1)],
                    ResourcePile::empty(),
                    PaymentConversionType::MayOverpay(3),
                ));
                player.can_afford(&cost).then_some(
                    vec![PaymentRequest::new(cost, "Pay 1-4 mood or culture tokens", false)])
            },
            |game, s| {
                let player = game.get_player_mut(s.player_index);
                let pile = &s.choice[0];
                let v = pile.resource_amount() as f32 / 2_f32;
                player.event_victory_points += v;
                game.add_info_log_item(&format!("{} paid {} to gain {} victory point{}", s.player_name, pile, v,
                                                if v == 1.0 { "" } else { "s" }));
            },
        )
        .build()
}

fn envoy() -> Incident {
    Incident::builder(40,
                      "Envoy",
                      "Gain 1 idea and 1 culture token. Select another player to gain 1 culture token. Draw the top card from the wonder deck. This card can be taken by anyone instead of drawing from the wonder pile.",
                      IncidentBaseEffect::BarbariansMove)
        .add_simple_incident_listener(
            IncidentTarget::ActivePlayer,
            1,
            |game, player, player_name, _| {
                game.add_info_log_item(&format!("{player_name} gained 1 idea and 1 culture token"));
                game.get_player_mut(player).gain_resources(
                    ResourcePile::culture_tokens(1) + ResourcePile::ideas(1));

                let wonder_from_pile = draw_wonder_from_pile(game);
                if let Some(wonder) = wonder_from_pile {
                    game.add_info_log_item(&format!("{} is now available to be taken by anyone",
                                                    wonder.name));
                    game.permanent_incident_effects.push(
                        PermanentIncidentEffect::PublicWonderCard(wonder.name));
                }
            },
        )
        .add_incident_player_request(
            "Select a player to gain 1 culture token",
            |_p| true,
            0,
            |game, s| {
                let p = s.choice;
                game.add_info_log_item(&format!("{} was selected to gain 1 culture token.",
                                                game.player_name(p)));
                game.get_player_mut(p).gain_resources(ResourcePile::culture_tokens(1));
            })
        .build()
}
