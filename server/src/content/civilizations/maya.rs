// Monumental Edition components: https://boardgamegeek.com/image/9490527
use crate::ability_initializer::{AbilityInitializerSetup, once_per_turn_ability};
use crate::advance::Advance;
use crate::city::MoodState;
use crate::city_pieces::Building;
use crate::civilization::Civilization;
use crate::content::custom_actions::{
    CustomAction, CustomActionActivation, CustomActionType, on_custom_action,
};
use crate::content::persistent_events::{
    EventResponse, PaymentRequest, PersistentEventRequest, ResourceRewardRequest,
    SelectedStructure, StructuresRequest,
};
use crate::events::EventPlayer;
use crate::game::{Game, GameState};
use crate::leader::{Leader, LeaderInfo, leader_position};
use crate::leader_ability::LeaderAbility;
use crate::map::{Block, Terrain};
use crate::payment::{PaymentConversion, PaymentConversionType, base_resources};
use crate::player::{CostTrigger, Data, Player};
use crate::player_events::CostInfo;
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use crate::structure::Structure;
use crate::tactics_card::CombatRole;
use std::collections::HashSet;

pub(crate) fn maya() -> Civilization {
    Civilization::new(
        "Maya",
        vec![terracing(), stelas(), ballcourts(), calendar()],
        vec![pakal(), siyaj(), wak()],
        Some(Block::new([
            Terrain::Fertile,
            Terrain::Mountain,
            Terrain::Forest,
            Terrain::Mountain,
        ])),
    )
}
fn terracing() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(SpecialAdvance::Terracing, SpecialAdvanceRequirement::Advance(Advance::Irrigation), "Terracing",
        "Collect food, wood or ore from Mountains. Settlers and leaders ignore the Mountain movement restriction.")
        .add_transient_event_listener(|e| &mut e.terrain_collect_options, -13, |options, _, _, _| {
            options.insert(Terrain::Mountain, HashSet::from([ResourcePile::food(1), ResourcePile::wood(1), ResourcePile::ore(1)]));
        }).build()
}
fn stelas_reward(p: &EventPlayer) -> Option<ResourceRewardRequest> {
    Some(ResourceRewardRequest::new(
        p.reward_options()
            .sum(1, &[ResourceType::CultureTokens, ResourceType::Ideas]),
        "Stelas · Gain 1 culture or idea".into(),
    ))
}
fn stelas() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(SpecialAdvance::Stelas, SpecialAdvanceRequirement::Advance(Advance::Arts), "Stelas",
        "Gain 1 culture token or 1 idea after constructing an Obelisk, founding a city, or capturing a city.")
        .add_resource_request(|e| &mut e.found_city, 27, |_, p, _| stelas_reward(p))
        .add_resource_request(|e| &mut e.construct, 27, |_, p, c| if c.building == Building::Obelisk { stelas_reward(p) } else { None })
        .add_resource_request(|e| &mut e.combat_end, 27, |_, p, c| if c.captured_city(p.index).is_some() { stelas_reward(p) } else { None })
        .build()
}
fn ballcourts() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(SpecialAdvance::Ballcourts, SpecialAdvanceRequirement::Advance(Advance::Sports), "Ballcourts",
        "When collecting or recruiting in a non-Angry city, pay 1 mood token for 1 extra capacity. Once per turn, a Spirituality advance costs 1 food less.")
        .add_transient_event_listener(|e| &mut e.advance_cost, -12, |cost, advance, game, p| {
            if game.cache.get_advance_group(crate::content::advances::AdvanceGroup::Spirituality).advances.iter().any(|a| a.advance == *advance) {
                once_per_turn_ability(p, cost, &(), &(), |c| &mut c.info.info, |c, _, _, p| {
                    // Research's default is ideas; remove one equivalent food from the 2-resource cost.
                    if c.cost.default.ideas > 0 { c.cost.default.ideas -= 1; } else { c.cost.default.food = c.cost.default.food.saturating_sub(1); }
                    c.info.add_log(p, "Ballcourts reduces Spirituality research by 1 food");
                });
            }
        }).build()
}
pub(crate) fn ballcourts_available(p: &Player, position: Position) -> bool {
    p.has_special_advance(SpecialAdvance::Ballcourts)
        && p.get_city(position).mood_state != MoodState::Angry
}
fn calendar() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(SpecialAdvance::Calendar, SpecialAdvanceRequirement::Advance(Advance::Astronomy), "Calendar",
        "As a free action, look at the next event. You may pay 1 culture token to put it at the bottom of the deck, then use Calendar again.")
        .add_custom_action(CustomActionType::Calendar, |c| c.any_times().free_action().no_resources(), |b| b
            .add_persistent_event_listener(|e| &mut e.custom_action, 0, |game, p, _| {
                let id = crate::card::draw_card_from_pile(game, p, "Events", |g| &mut g.incidents_left,
                    |g| g.cache.get_incidents().iter().map(|i| i.id).collect(),
                    |p| p.action_cards.iter().filter_map(|id| id.checked_sub(crate::content::incidents::great_persons::GREAT_PERSON_OFFSET)).collect()).expect("event deck");
                game.incidents_left.insert(0, id);
                let card = game.cache.get_incident(id);
                let request = PaymentRequest::optional(p.payment_options().resources(p.get(game), ResourcePile::culture_tokens(1)),
                    &format!("{} · {}\nPay 1 culture to put this event at the bottom, or keep it on top.", card.name, card.description(game).join(" ")));
                Some(PersistentEventRequest::Payment(vec![request]))
            }, |game, p, action, request, _| {
                let (EventResponse::Payment(payments), PersistentEventRequest::Payment(requests)) = (action, request) else { panic!("Calendar payment"); };
                assert_eq!(payments.len(), 1);
                crate::resource::pay_cost(game, p.index, &requests[0], &payments[0]);
                if !payments[0].is_empty() {
                    let id = game.incidents_left.remove(0); game.incidents_left.push(id);
                    p.log(game, "Placed the next event at the bottom of the deck");
                } else { p.log(game, "Kept the next event on top of the deck"); }
            }), |_, _| true).build()
}
fn art_cost(c: &mut CostInfo, p: &EventPlayer) {
    let resources = base_resources();
    let mut pairs = vec![];
    for (i, first) in resources.iter().enumerate() {
        for second in &resources[i..] {
            pairs.push(first.clone() + second.clone());
        }
    }
    c.cost.conversions.push(PaymentConversion::resource_options(
        pairs,
        ResourcePile::culture_tokens(1),
        PaymentConversionType::MayNotOverpay(1),
    ));
    c.info.add_log(
        p,
        "Art and Architecture: 1 culture may replace 2 construction resources",
    );
}
fn pakal() -> LeaderInfo {
    LeaderInfo::new(Leader::Pakal, "Pakal",
        LeaderAbility::builder("Sun Shield", "In the first combat round, cancel 1 hit when Pakal defends a city with an Obelisk in your color.")
            .add_combat_strength_listener(115, |game, c, s, role| {
                if c.stats.round == 1 && role == CombatRole::Defender && c.has_leader(role, game)
                    && c.defender_city(game).is_some_and(|city| city.pieces.obelisk == Some(c.player(role))) { s.hit_cancels += 1; s.roll_log.push("Sun Shield cancels 1 hit".into()); }
            }).build(),
        LeaderAbility::builder("Art and Architecture", "When constructing a building in Pakal's city, 1 culture token may replace 2 resources.")
            .add_transient_event_listener(|e| &mut e.building_cost, 14, |c, _, game, p| {
                if c.city_position == Some(leader_position(p.get(game))) { art_cost(c, p); }
            }).build())
}

fn influence_targets(game: &Game, p: &Player) -> Vec<SelectedStructure> {
    let pos = leader_position(p);
    game.players
        .iter()
        .flat_map(|p| &p.cities)
        .flat_map(|city| {
            city.pieces
                .buildings(None)
                .into_iter()
                .map(|b| SelectedStructure::new(city.position, Structure::Building(b)))
        })
        .filter(|target| {
            crate::cultural_influence::influence_culture_boost_cost_from(
                game,
                p.index,
                target,
                &crate::playing_actions::PlayingActionType::InfluenceCultureAttempt,
                false,
                false,
                game.get_any_city(target.position).player_index,
                Some(pos),
            )
            .is_ok()
        })
        .collect()
}
fn influx<E, V, B>(
    b: B,
    event: E,
    position: impl Fn(&V) -> Option<Position> + Clone + Send + Sync + 'static,
) -> B
where
    B: AbilityInitializerSetup,
    V: Clone + PartialEq,
    E: Fn(
            &mut crate::player_events::PersistentEvents,
        ) -> &mut crate::player_events::PersistentEvent<V>
        + Clone
        + Send
        + Sync
        + 'static,
{
    b.add_structures_request(
        event,
        -22,
        move |game, p, value| {
            if position(value) != Some(leader_position(p.get(game))) {
                return None;
            }
            let targets = influence_targets(game, p.get(game));
            (!targets.is_empty()).then(|| {
                StructuresRequest::new(targets, 0..=1, "Cultural Influx · Influence from this city")
            })
        },
        |game, s, _| {
            if let Some(target) = s.choice.first() {
                let mut attempt = crate::cultural_influence::InfluenceCultureAttempt::new(
                    target.clone(),
                    crate::playing_actions::PlayingActionType::InfluenceCultureAttempt,
                );
                attempt.starting_position = Some(leader_position(s.player().get(game)));
                crate::cultural_influence::execute_influence_culture_attempt(
                    game,
                    s.player_index,
                    &attempt,
                )
                .expect("influx target");
            }
        },
    )
}
fn siyaj() -> LeaderInfo {
    let cultural = LeaderAbility::builder(
        "Cultural Influx",
        "After constructing a building or wonder in Siyaj K'ak's city, you may attempt Cultural Influence from that city as a free action.",
    );
    let cultural = influx(cultural, |e| &mut e.construct, |c| c.city_position);
    let cultural = influx(
        cultural,
        |e| &mut e.play_wonder_card,
        |w| w.selected_position,
    );
    LeaderInfo::new(Leader::SiyajKak, "Siyaj K'ak'",
        LeaderAbility::builder("Fire Is Born", "In the first combat round attacking a city, Siyaj K'ak' adds combat value equal to its size.")
            .add_combat_strength_listener(116, |game, c, s, role| {
                if c.stats.round == 1 && role == CombatRole::Attacker && c.has_leader(role, game) {
                    if let Some(city) = c.defender_city(game) { s.extra_combat_value += city.size() as i8; s.roll_log.push(format!("Fire Is Born adds +{} combat value", city.size())); }
                }
            }).build(), cultural.build())
}

fn replacement_buildings(
    game: &Game,
    p: &Player,
    removed: &SelectedStructure,
) -> Vec<SelectedStructure> {
    let Structure::Building(old) = removed.structure else {
        return vec![];
    };
    let mut city =
        crate::city::City::from_data(p.get_city(removed.position).cloned_data(), p.index);
    city.pieces.set_building_owner(old, None);
    city.mood_state = MoodState::Happy;
    crate::city_pieces::BUILDINGS
        .into_iter()
        .filter(|b| {
            *b != old
                && crate::construct::can_construct(
                    &city,
                    *b,
                    p,
                    game,
                    CostTrigger::NoModifiers,
                    &[
                        crate::construct::ConstructDiscount::NoCityActivation,
                        crate::construct::ConstructDiscount::NoResourceCost,
                    ],
                )
                .is_ok()
        })
        .filter(|b| {
            *b != Building::Port
                || !crate::construct::new_building_positions(game, *b, &city).is_empty()
        })
        .map(|b| SelectedStructure::new(city.position, Structure::Building(b)))
        .collect()
}
fn wak() -> LeaderInfo {
    LeaderInfo::new(Leader::WakChanilAjaw, "Wak Chanil Ajaw",
        LeaderAbility::builder("New Dynasty", "Wak Chanil Ajaw adds 2 combat value within 2 spaces of one of your cities.")
            .add_combat_strength_listener(117, |game, c, s, role| {
                if c.has_leader(role, game) && game.player(c.player(role)).cities.iter().any(|city| city.position.distance(c.defender_position()) <= 2) {
                    s.extra_combat_value += 2; s.roll_log.push("New Dynasty adds +2 combat value".into());
                }
            }).build(),
        LeaderAbility::builder("Reconstruction", "After a Move action in which Wak Chanil Ajaw captured a city, you may replace a building in your color in her city with a different building, free of resource and action costs, even if Angry.")
            .add_simple_persistent_event_listener(|e| &mut e.combat_end, -21, |game, p, stats| {
                if stats.captured_city(p.index).is_some() && stats.player(p.index).survived_leader() {
                    p.get_mut(game).custom_data.insert("Reconstruction".into(), Data::Number(1));
                }
            })
            .add_transient_event_listener(|e| &mut e.after_action, -20, |game, _, _, p| {
                if game.state == GameState::Playing && game.events.is_empty() && p.get_mut(game).custom_data.remove("Reconstruction").is_some() {
                    let position = leader_position(p.get(game));
                    if p.get(game).try_get_city(position).is_some() {
                        on_custom_action(game, p.index, CustomActionActivation::new(CustomAction::new(CustomActionType::Reconstruction, Some(position)), ResourcePile::empty()));
                    }
                }
            })
            .add_custom_action(CustomActionType::Reconstruction, |c| c.any_times().free_action().no_resources(), |b| b
                .add_structures_request(|e| &mut e.custom_action, 2, |game, p, a| {
                    let pos = a.action.city.expect("reconstruction city");
                    let targets = p.get(game).get_city(pos).pieces.buildings(Some(p.index)).into_iter().map(|b| SelectedStructure::new(pos, Structure::Building(b)))
                        .filter(|s| !replacement_buildings(game, p.get(game), s).is_empty()).collect::<Vec<_>>();
                    (!targets.is_empty()).then(|| StructuresRequest::new(targets, 0..=1, "Reconstruction · Choose a building to replace"))
                }, |_, s, a| a.selected_structure = s.choice.first().cloned())
                .add_structures_request(|e| &mut e.custom_action, 1, |game, p, a| {
                    a.selected_structure.as_ref().map(|old| StructuresRequest::new(replacement_buildings(game, p.get(game), old), 1..=1, "Reconstruction · Choose the new building"))
                }, |game, s, a| {
                    let old = a.selected_structure.take().unwrap();
                    let Structure::Building(old_building) = old.structure else { unreachable!() };
                    let Structure::Building(new_building) = s.choice[0].structure else { unreachable!() };
                    crate::city_pieces::lose_building(game, &s.player(), old_building, old.position);
                    // Port placement is completed by the next position request.
                    a.selected_structure = Some(s.choice[0].clone());
                    if new_building != Building::Port { reconstruct(game, &s.player(), new_building, old.position, None); }
                })
                .add_position_request(|e| &mut e.custom_action, 0, |game, p, a| {
                    let selected = a.selected_structure.as_ref()?;
                    if selected.structure != Structure::Building(Building::Port) { return None; }
                    Some(crate::content::persistent_events::PositionRequest::new(crate::construct::new_building_positions(game, Building::Port, p.get(game).get_city(selected.position)).into_iter().flatten().collect(), 1..=1, "Reconstruction · Place Port"))
                }, |game, s, a| reconstruct(game, &s.player(), Building::Port, a.selected_structure.as_ref().unwrap().position, Some(s.choice[0]))),
                |_, _| false).build())
}
fn reconstruct(
    game: &mut Game,
    p: &EventPlayer,
    building: Building,
    position: Position,
    port: Option<Position>,
) {
    crate::construct::do_construct(
        game,
        p.index,
        &crate::construct::Construct::new(position, building, ResourcePile::empty())
            .with_port_position(port),
        false,
        &p.origin,
    );
}
