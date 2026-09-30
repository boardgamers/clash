use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::gain_action;
use crate::advance::{Advance, AdvanceBuilder, AdvanceInfo};
use crate::city::MoodState;
use crate::content::advances::{AdvanceGroup, AdvanceGroupInfo, advance_group_builder};
use crate::content::custom_actions::CustomActionType::{AbsolutePower, ForcedLabor};
use crate::content::persistent_events::ResourceRewardRequest;
use crate::game::GameOptions;
use crate::player::Player;
use crate::resource_pile::ResourcePile;

pub(crate) fn autocracy(options: &GameOptions) -> AdvanceGroupInfo {
    advance_group_builder(
        AdvanceGroup::Autocracy,
        "Autocracy",
        options,
        vec![
            nationalism(),
            totalitarianism(),
            absolute_power(),
            forced_labor(),
        ],
    )
}

fn nationalism() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Nationalism,
        "Nationalism",
        "After each Recruit action in which you recruit at least one army unit or Ship, gain 1 mood token or 1 culture token in total, regardless of the number of units recruited.",
    )
    .add_resource_request(
        |event| &mut event.recruit,
        1,
        |_game, p, recruit| {
            recruit
                .units
                .clone()
                .to_vec()
                .iter()
                .any(|u| u.is_army_unit() || u.is_ship())
                .then_some(ResourceRewardRequest::new(
                    p.reward_options().tokens(1),
                    "Select token to gain".to_string(),
                ))
        },
    )
}

fn totalitarianism() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Totalitarianism,
        "Totalitarianism",
        "Opponents cannot spend culture tokens to boost the range or die roll of Influence Culture attempts against your cities containing your army units.",
    )
    .add_transient_event_listener(
        |event| &mut event.on_influence_culture_attempt,
        0,
        |r, city, game, _| {
            if let Ok(info) = r
                && info.is_defender
                && game
                    .player(city.player_index)
                    .get_units(city.position)
                    .iter()
                    .any(|u| u.is_army_unit())
            {
                info.set_no_boost();
            }
        },
    )
}

fn absolute_power() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::AbsolutePower,
        "Absolute Power",
        "Once per turn, as a free action, you may spend 2 mood tokens to get an additional action",
    )
    .add_custom_action(
        AbsolutePower,
        |c| {
            c.once_per_turn()
                .free_action()
                .resources(ResourcePile::mood_tokens(2))
        },
        |b| {
            b.add_simple_persistent_event_listener(
                |event| &mut event.custom_action,
                0,
                |game, p, _| {
                    gain_action(game, p);
                },
            )
        },
        |_, _| true,
    )
}

fn forced_labor() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::ForcedLabor,
        "Forced Labor",
        "Once per turn, pay 1 mood token without spending an action to treat all your Angry cities as Neutral for the rest of your turn. Each of those cities can still be activated only once.",
    )
    .add_custom_action(
        ForcedLabor,
        |c| {
            c.once_per_turn()
                .free_action()
                .resources(ResourcePile::mood_tokens(1))
        },
        |b| {
            b.add_simple_persistent_event_listener(
                |event| &mut event.custom_action,
                0,
                |game, p, _| {
                    p.log(game, "Treating Angry cities as neutral");
                },
            )
        },
        |_game, player| any_angry(player),
    )
}

fn any_angry(player: &Player) -> bool {
    player
        .cities
        .iter()
        .any(|city| city.mood_state == MoodState::Angry)
}
