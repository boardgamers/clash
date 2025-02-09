use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder};
use crate::city_pieces::Building::Academy;
use crate::content::advances::{advance_group_builder, get_advances_by_group, AdvanceGroup};
use crate::content::custom_phase_actions::CustomPhasePaymentRequest;
use crate::payment::PaymentOptions;
use crate::resource_pile::ResourcePile;

pub(crate) fn education() -> AdvanceGroup {
    advance_group_builder(
        "Education",
        vec![
            Advance::builder("Writing", "todo")
                .with_advance_bonus(CultureToken)
                .with_unlocked_building(Academy),
            free_education(),
            philosophy(),
        ],
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
            if get_advances_by_group("Science")
                .iter()
                .any(|a| &a.name == advance)
            {
                player.gain_resources(ResourcePile::ideas(1));
                player.add_to_last_log_item(". Philosophy gained 1 idea.");
            }
        },
        0,
    )
    .with_advance_bonus(MoodToken)
}

fn free_education() -> AdvanceBuilder {
    Advance::builder(
        "Free Education",
        "After you buy an Advance by paying for it with at least 1 gold or 1 idea, you may pay
        an extra 1 idea to gain 1 mood token",
    )
    .with_advance_bonus(MoodToken)
    .add_payment_request_with_commands_listener(
        |e| &mut e.on_advance_custom_phase,
        1,
        |_game, _player_index, i| {
            if i.name == "Free Education" {
                None
            } else if i.payment.has_at_least(&ResourcePile::gold(1))
                || i.payment.has_at_least(&ResourcePile::ideas(1))
            {
                Some(vec![CustomPhasePaymentRequest {
                    cost: PaymentOptions::resources(ResourcePile::ideas(1)),
                    name: "Pay extra 1 idea for a mood token".to_string(),
                    optional: true,
                }])
            } else {
                None
            }
        },
        |c, _game, payment| {
            c.add_to_last_log_item(&format!(
                "{} paid {} for free education to gain 1 mood token",
                c.name, payment[0]
            ));
            c.gain_resources(ResourcePile::mood_tokens(1));
        },
    )
}
