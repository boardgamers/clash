use crate::ability_initializer::{
    AbilityInitializerBuilder, AbilityInitializerSetup, AbilityListeners,
};
use crate::advance::{Advance, gain_advance_without_payment};
use crate::city::{City, MoodState, activate_city};
use crate::content::ability::AbilityBuilder;
use crate::content::advances::AdvanceGroup;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::AdvanceRequest;
use crate::events::{EventOrigin, EventPlayer};
use crate::game::Game;
use crate::leader::leader_position;
use crate::player::Player;
use crate::wonder::{Wonder, force_draw_wonder_from_anywhere};
use itertools::Itertools;

#[derive(Clone)]
pub struct LeaderAbility {
    pub name: String,
    pub description: String,
    pub listeners: AbilityListeners,
}

impl LeaderAbility {
    #[must_use]
    pub fn builder(name: &str, description: &str) -> LeaderAbilityBuilder {
        LeaderAbilityBuilder::new(name.to_string(), description.to_string())
    }

    #[must_use]
    pub fn advance_gain_custom_action(
        name: &str,
        action: CustomActionType,
        group: AdvanceGroup,
    ) -> LeaderAbility {
        LeaderAbilityBuilder::new(
            name.to_string(),
            format!(
                "If the leader city is happy: As an action, pay 1 mood or culture token: \
                Gain 1 {group} advance for free.",
            ),
        )
        .add_custom_action(
            action,
            |c| c.any_times().action().tokens(1),
            move |b| use_get_advance(b, group),
            move |game, p| {
                !advances_in_group(game, p, group).is_empty()
                    && p.try_get_city(leader_position(p))
                        .is_some_and(|c| c.mood_state == MoodState::Happy)
            },
        )
        .build()
    }

    #[must_use]
    pub fn wonder_expert(wonder: Wonder) -> LeaderAbility {
        LeaderAbilityBuilder::new(
            wonder.name(),
            format!(
                "A: When drawing a wonder: Take {wonder} from anywhere in the game - \
                unless already built. If a player or Envoy had {wonder}, \
                they get to draw a new wonder instead. \
                B: Building {wonder} costs 2 culture tokens less.\
                C: Building any wonder in the leader city is a free action.",
            ),
        )
        .add_simple_persistent_event_listener(
            |e| &mut e.draw_wonder_card,
            1,
            move |game, player, draw| {
                if force_draw_wonder_from_anywhere(game, player, wonder) {
                    draw.drawn = true;
                }
            },
        )
        .add_transient_event_listener(
            |event| &mut event.wonder_cost,
            0,
            move |i, w, game, p| {
                if w.wonder == wonder {
                    i.cost.default.culture_tokens -= 2;
                    i.info.add_log(
                        p,
                        &format!("Reduce the cost of {wonder} by 2 culture tokens"),
                    );
                }

                if w.city_position == leader_position(game.player(w.player)) {
                    i.ignore_action_cost = true;
                    i.info.add_log(
                        p,
                        &format!("{wonder} can be built in the leader city as a free action"),
                    );
                }
            },
        )
        .build()
    }
}

pub struct LeaderAbilityBuilder {
    name: String,
    description: String,
    builder: AbilityInitializerBuilder,
}

impl LeaderAbilityBuilder {
    fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            builder: AbilityInitializerBuilder::new(),
        }
    }

    #[must_use]
    pub fn build(self) -> LeaderAbility {
        LeaderAbility {
            name: self.name,
            description: self.description,
            listeners: self.builder.build(),
        }
    }
}

impl AbilityInitializerSetup for LeaderAbilityBuilder {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder {
        &mut self.builder
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::LeaderAbility(self.name.clone())
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}

fn use_get_advance(b: AbilityBuilder, group: AdvanceGroup) -> AbilityBuilder {
    b.add_advance_request(
        |event| &mut event.custom_action,
        0,
        move |game, p, _| {
            let player = p.get(game);
            Some(AdvanceRequest::new(advances_in_group(game, player, group)))
        },
        move |game, s, c| {
            gain_advance_without_payment(game, s.choice, &s.player(), c.payment.clone(), true);
        },
    )
}

fn advances_in_group(game: &Game, player: &Player, group: AdvanceGroup) -> Vec<Advance> {
    game.cache
        .get_advance_group(group)
        .advances
        .iter()
        .filter_map(|a| {
            player
                .can_advance_free(a.advance, game)
                .then_some(a.advance)
        })
        .collect_vec()
}

pub(crate) fn can_activate_leader_city(game: &Game, p: &Player) -> bool {
    game.try_get_any_city(leader_position(p))
        .is_some_and(City::can_activate)
}

pub(crate) fn activate_leader_city(game: &mut Game, player: &EventPlayer) {
    activate_city(leader_position(player.get(game)), game, &player.origin);
}
