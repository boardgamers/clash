use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder};
use crate::city_pieces::Building::Academy;
use crate::content::advances::{advance_group_builder, get_group, AdvanceGroup};
use crate::content::custom_phase_actions::PaymentRequest;
use crate::payment::PaymentOptions;
use crate::resource_pile::ResourcePile;

pub(crate) fn education() -> AdvanceGroup {
    advance_group_builder(
        "Education",
        vec![
            writing(),
            public_education(),
            free_education(),
            philosophy(),
        ],
    )
}

fn writing() -> AdvanceBuilder {
    Advance::builder("Writing", "todo")
        .with_advance_bonus(CultureToken)
        .with_unlocked_building(Academy)
}

fn public_education() -> AdvanceBuilder {
    Advance::builder(
        "Public Education",
        "Once per turn, when you collect resources in a city with an Academy, gain 1 idea",
    )
    .with_advance_bonus(MoodToken)
    .add_once_per_turn_listener(
        |event| &mut event.on_collect,
        |e| &mut e.info.info,
        |i, game, ()| {
            let player = game.get_player(game.active_player());
            if player.get_city(i.city).pieces.academy.is_some() {
                i.total += ResourcePile::ideas(1);
                i.info
                    .log
                    .push("Public Education gained 1 idea".to_string());
            }
        },
        0,
    )
}

fn free_education() -> AdvanceBuilder {
    Advance::builder(
        "Free Education",
        "After you buy an Advance by paying for it with at least 1 gold or 1 idea, you may pay
        an extra 1 idea to gain 1 mood token",
    )
    .with_advance_bonus(MoodToken)
    .add_payment_request_listener(
        |e| &mut e.on_advance,
        1,
        |_game, _player_index, i| {
            if i.name == "Free Education" {
                None
            } else if i.payment.has_at_least(&ResourcePile::gold(1))
                || i.payment.has_at_least(&ResourcePile::ideas(1))
            {
                Some(vec![PaymentRequest {
                    cost: PaymentOptions::resources(ResourcePile::ideas(1)),
                    name: "Pay extra 1 idea for a mood token".to_string(),
                    optional: true,
                }])
            } else {
                None
            }
        },
        |game, s| {
            let pile = &s.choice[0];
            if pile.is_empty() {
                game.add_info_log_item(&format!(
                    "{} declined to pay for free education",
                    s.player_name
                ));
                return;
            }
            game.add_info_log_item(&format!(
                "{} paid {} for free education to gain 1 mood token",
                s.player_name, pile
            ));
            game.get_player_mut(s.player_index)
                .gain_resources(ResourcePile::mood_tokens(1));
        },
    )
}

fn philosophy() -> AdvanceBuilder {
    Advance::builder(
        "Philosophy",
        "Immediately gain 1 idea after getting a Science advance",
    )
    .add_one_time_ability_initializer(|game, player_index| {
        game.players[player_index].gain_resources(ResourcePile::ideas(1));
    })
    .add_simple_current_event_listener(
        |event| &mut event.on_advance,
        0,
        |game, player_index, player_name, advance| {
            if get_group("Science")
                .advances
                .iter()
                .any(|a| a.name == advance.name)
            {
                let player = game.get_player_mut(player_index);
                player.gain_resources(ResourcePile::ideas(1));
                game.add_info_log_item(&format!("{player_name} gained 1 idea from Philosophy"));
            }
        },
    )
    .with_advance_bonus(MoodToken)
}
