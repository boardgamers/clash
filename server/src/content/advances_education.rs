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
        |e| &mut e.content.info,
        |player, game, pos| {
            if game.get_city(player.index, *pos).pieces.academy.is_some() {
                player.gain_resources(ResourcePile::ideas(1));
                player.add_info_log_item("Public Education gained 1 idea");
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
        |e| &mut e.on_advance_custom_phase,
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
        |game, payment| {
            payment.to_commands(game, |c, _game, payment| {
                c.add_info_log_item(&format!(
                    "{} paid {} for free education to gain 1 mood token",
                    c.name, payment[0]
                ));
                c.gain_resources(ResourcePile::mood_tokens(1));
            });
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
    .add_ability_undo_deinitializer(|game, player_index| {
        game.players[player_index].lose_resources(ResourcePile::ideas(1));
    })
    .add_player_event_listener(
        |event| &mut event.on_advance,
        |player, _, advance| {
            if get_group("Science")
                .advances
                .iter()
                .any(|a| &a.name == advance)
            {
                player.gain_resources(ResourcePile::ideas(1));
                player.add_info_log_item("Philosophy gained 1 idea");
            }
        },
        0,
    )
    .with_advance_bonus(MoodToken)
}
