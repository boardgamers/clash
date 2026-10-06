use crate::action::Action;
use crate::advance::{Advance, AdvanceAction, is_special_advance_active};
use crate::card::HandCard;
use crate::city::MoodState;
use crate::collect::{
    Collect, PositionCollection, collect_event_origin, possible_resource_collections,
};
use crate::consts::OBJECTIVE_VICTORY_POINTS;
use crate::content::advances::AdvanceGroup;
use crate::content::persistent_events::{
    EventResponse, PersistentEventRequest, PersistentEventType,
};
use crate::game::{Game, GameState};
use crate::objective_card::ObjectiveType;
use crate::payment::PaymentOptions;
use crate::player::CostTrigger;
use crate::playing_actions::{PlayingAction, PlayingActionType};
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::special_advance::SpecialAdvanceRequirement;
use crate::victory_points::victory_points_parts;
use crate::wonder::Wonder;
use serde_json::{Value, json};
mod actions;
mod collection_potential;
mod decisions;
mod journal;
mod turn;
mod waiting;
pub use actions::recruit_preview;

pub fn query(game: &Game, seat: usize, input: Value) -> Result<Value, String> {
    if seat >= game.players.len() || !game.player(seat).is_human() {
        return Err("Choose a player".into());
    }
    match input["kind"].as_str() {
        Some("decision") => decisions::preview(game, seat, &input),
        Some("researchPlan") => research_plan(game, seat, &input),
        Some("movement") if input["city"].is_string() => actions::nomad_movement(
            game,
            seat,
            serde_json::from_value(input["city"].clone()).map_err(|e| e.to_string())?,
            serde_json::from_value(input["units"].clone()).map_err(|e| e.to_string())?,
        ),
        Some("movement") => actions::movement(
            game,
            seat,
            serde_json::from_value(input["units"].clone()).map_err(|e| e.to_string())?,
        ),
        Some("recruit") => actions::recruit_extended(game, seat, &input),
        Some("happiness") => actions::happiness_preview(game, seat, &input),
        Some("collectionPotential") => Ok(collection_potential::potentials(
            game,
            seat,
            &serde_json::from_value(input["variant"].clone()).map_err(|e| e.to_string())?,
        )),
        Some("collect") => collect_variant_preview(
            game,
            seat,
            serde_json::from_value(input["city"].clone()).map_err(|e| e.to_string())?,
            serde_json::from_value(input["selections"].clone()).map_err(|e| e.to_string())?,
            serde_json::from_value(input["variant"].clone()).map_err(|e| e.to_string())?,
            input["ballcourts"].as_bool().unwrap_or(false),
        ),
        _ => Err("Unknown preview".into()),
    }
}

pub fn view(game: &Game, seat: Option<usize>) -> Value {
    let seat = seat.filter(|i| *i < game.players.len() && game.player(*i).is_human());
    let active = game.active_player();
    let active_players = game.active_players();
    let waiting_for = waiting::describe(game);
    let civilization_draft = game.civilization_draft.as_ref().map(|draft| {
        json!({
            "ready": draft.ready,
            "chosen": seat.and_then(|i| draft.choices[i].clone()),
            "waiting": seat.is_none_or(|i| draft.ready[i]),
        })
    });
    let playing = game.state == GameState::Playing && game.events.is_empty();
    let objective_phase = game.events.last().is_some_and(|event| {
        matches!(event.event_type, PersistentEventType::SelectObjectives(_))
            && matches!(
                game.current_event_handler().map(|h| &h.request),
                Some(PersistentEventRequest::SelectHandCards(_))
            )
    });
    let moving = matches!(game.state, GameState::Movement(_)) && game.events.is_empty();
    let choice = seat.and_then(|seat| choice_decision(game, seat));
    let exploration = seat.and_then(|seat| exploration_decision(game, seat));
    let decision = seat.and_then(|seat| decisions::describe(game, seat));
    let supported_phase = playing
        || objective_phase
        || moving
        || choice.is_some()
        || exploration.is_some()
        || decision.is_some()
        || matches!(
            game.state,
            GameState::ChooseCivilization | GameState::Finished
        );
    let can_play = seat == Some(active) && playing;
    let event_catalog = journal::catalog(game);
    // Static rules reference: includes no hand identities or deck order.
    let card_catalog = game.cache.get_action_cards().iter()
        .chain(game.cache.get_incidents().iter().filter_map(|i| i.action_card.as_ref()))
        .map(|c| json!({"id":c.id,"name":c.civil_card.name,"description":c.civil_card.description,
            "free":c.civil_card.action_type.free,
            "tactics":c.tactics_card.as_ref().map(|t|json!({"name":t.name,"description":t.description}))}))
        .collect::<Vec<_>>();
    // A reference catalog comes from rules, never the deck or anyone's hand.
    let mut wonder_catalog = game.cache.get_wonders().iter().map(|info| json!({
        "id":info.wonder,"name":info.name(),"description":info.description,
        "cost":info.cost.default_payment(),"requiredAdvance":info.required_advance.name(game),
        "requiredAdvanceOwned":false,"builtPoints":info.built_victory_points,
        "ownedPoints":info.owned_victory_points
    })).collect::<Vec<_>>();
    wonder_catalog.sort_by_key(|w| w["name"].as_str().unwrap_or_default().to_string());
    // Built wonders are public, including for spectators. Read their effects from
    // the same definitions as the cards in hand, without exposing anyone's hand.
    let built_wonders = game
        .players
        .iter()
        .flat_map(|p| &p.cities)
        .flat_map(|c| &c.pieces.wonders)
        .filter(|w| **w != Wonder::Hidden)
        .map(|wonder| {
            let info = wonder.info(game);
            json!({"id":wonder,"name":info.name(),"description":info.description,
                "builtPoints":info.built_victory_points,"ownedPoints":info.owned_victory_points})
        })
        .collect::<Vec<_>>();
    let log_origin_names: std::collections::BTreeMap<String, String> = game
        .log
        .iter()
        .flat_map(|age| &age.rounds)
        .flat_map(|round| &round.turns)
        .flat_map(|turn| &turn.actions)
        .flat_map(|action| &action.items)
        .map(|item| {
            (
                serde_json::to_string(&item.origin).expect("serializable origin"),
                item.origin.name(game),
            )
        })
        .collect();
    let pending_event = game.events.iter().rev().find_map(|event| {
        if let PersistentEventType::Incident(info) = &event.event_type {
            Some(json!({"id":info.incident_id,"player":active}))
        } else {
            None
        }
    });
    // A map-reading aid, not legal move offers. Use visible terrain only, including for spectators.
    let mut sea_routes = game
        .map
        .tiles
        .iter()
        .filter(|(_, terrain)| terrain.is_water())
        .flat_map(|(position, _)| crate::move_routes::navigation_paths(&game.map, *position))
        .collect::<Vec<_>>();
    sea_routes.sort_by_key(|path| format!("{path:?}"));
    let barbarian_guide = game
        .players
        .iter()
        .filter(|p| p.is_human())
        .map(|p| crate::barbarians::map_guide(game, p.index))
        .collect::<Vec<_>>();
    let pirate_spawns = game
        .players
        .iter()
        .filter(|p| p.is_human())
        .map(|p| {
            let sorted = |blockade| {
                let mut positions = crate::pirates::pirate_spawn_positions(game, p.index, blockade);
                positions.sort_by_key(std::string::ToString::to_string);
                positions
            };
            json!({"player":p.index,"first":sorted(true),"second":sorted(false)})
        })
        .collect::<Vec<_>>();
    let players = game.players.iter().filter(|p| p.is_human()).map(|p| json!({
        "index": p.index, "name": game.player_name(p.index), "civilization": p.civilization.name, "capital":crate::map::capital_city_position(game,p),
        "score": p.victory_points(game),
        "eventTokens": p.incident_tokens,
        "advances": game.cache.get_advances().keys().filter(|a| p.can_use_advance(**a)).map(|a| {
            let mut item = advance_description(game, *a);
            item["borrowed"] = json!(!p.has_advance(*a));
            item["borrowedSource"] = json!(if crate::content::civilizations::egypt::grants_advance(p, *a) { "Man God" } else { "Great Library" });
            item
        }).chain(p.special_advances.iter().map(|a| {
            let info = a.info(game);
            json!({"id":a,"name":info.name,"description":info.description,"group":p.civilization.name,"order":1000,"borrowed":false})
        })).collect::<Vec<_>>(),
        "civilizationAdvances": p.civilization.special_advances.iter().enumerate().map(|(order, info)| {
            let required = match info.requirement {
                SpecialAdvanceRequirement::Advance(a) => vec![a],
                SpecialAdvanceRequirement::AnyGovernment => {
                    let mut advances = game.cache.get_advances().iter().filter(|(_, a)| a.leading_government).map(|(a, _)| *a).collect::<Vec<_>>();
                    advances.sort();
                    advances
                }
            };
            json!({"id":info.advance,"name":info.name,"description":info.description,"group":p.civilization.name,"order":order,
                "owned":p.has_special_advance(info.advance),
                "active":p.has_special_advance(info.advance) && is_special_advance_active(info.advance, p.advances, game),
                "requirement":info.requirement.name(game),
                "prerequisites":required.iter().map(|a|json!({"id":a,"name":a.name(game)})).collect::<Vec<_>>()})
        }).collect::<Vec<_>>(),
        "scoreParts": victory_points_parts(p, game).map(|(name, points)| json!({"name": name, "points": points})),
        "completedObjectives": p.completed_objectives.iter().map(|completed| {
            let objective = game.cache.get_objective(&completed.name);
            json!({"name":completed.name,"description":objective.description,"points":OBJECTIVE_VICTORY_POINTS})
        }).collect::<Vec<_>>(),
        "leaders": p.units.iter().filter_map(|u| {
            if let crate::unit::UnitType::Leader(leader) = u.unit_type {
                let info = game.cache.get_leader(&leader);
                Some(json!({"id":leader,"unit":u.id,"position":u.position,"name":info.name,
                    "abilities":info.abilities.iter().map(|a|json!({"name":a.name,"description":a.description})).collect::<Vec<_>>()}))
            } else { None }
        }).collect::<Vec<_>>(),
        "civilizationLeaders": p.civilization.leaders.iter().map(|info| json!({
            "id":info.leader,"name":info.name,"recruited":p.recruited_leaders.contains(&info.leader),
            "abilities":info.abilities.iter().map(|a|json!({"name":a.name,"description":a.description})).collect::<Vec<_>>()
        })).collect::<Vec<_>>(),
        "cities": p.cities.iter().map(|c| json!({"position": c.position, "size": c.size(), "capacity": c.mood_modified_size(p), "mood": c.mood_state, "activations": c.activations,"protection":crate::content::civilizations::egypt::protection(p,c.position),"independentPort":crate::content::civilizations::phoenicia::independent_port(game,c.pieces.port),"influenceMarker":c.influence_marker})).collect::<Vec<_>>()
    })).collect::<Vec<_>>();
    let Some(seat) = seat else {
        return json!({"logOriginNames":log_origin_names,"builtWonders":built_wonders,"cardCatalog":card_catalog,"eventCatalog":event_catalog,"pendingEvent":pending_event,"civilizationDraft":civilization_draft,"waitingFor":waiting_for,"activePlayers":active_players,"activePlayer": active, "canPlay": false, "supportedPhase": supported_phase, "players": players, "cities": [], "advances": [], "objectiveCards": [], "wonderCards": [], "wonderCatalog":wonder_catalog, "objectiveDecision": null, "cityActions": [], "settlers": [], "stopMovement": null, "canUndo": false, "canEndTurn": false, "barbarianGuide":barbarian_guide,"pirateSpawns":pirate_spawns,"seaRoutes":sea_routes});
    };
    let p = game.player(seat);
    let wonder_cards = p.wonder_cards.iter().filter(|wonder| **wonder != Wonder::Hidden).map(|wonder| {
        let info = wonder.info(game);
        let reason = if can_play { PlayingActionType::WonderCard(*wonder).is_available(game,seat).err() } else { Some("Wait for your turn".into()) };
        json!({"id": wonder, "name": info.name(), "description": info.description,
            "reason":reason,"action":reason.is_none().then(||Action::Playing(PlayingAction::WonderCard(*wonder))),
            "cost": info.cost.default_payment(), "requiredAdvance": info.required_advance.name(game),
            "requiredAdvanceOwned": p.has_advance(info.required_advance),
            "builtPoints": info.built_victory_points, "ownedPoints": info.owned_victory_points})
    }).collect::<Vec<_>>();
    // Hidden cards use ID zero in the player-filtered state; never resolve them.
    let objective_cards = p
        .objective_cards
        .iter()
        .filter(|id| **id != 0)
        .map(|id| {
            let card = game.cache.get_objective_card(*id);
            json!({"id": card.id, "objectives": card.objectives.iter().map(|objective| json!({
            "name": objective.name,
            "description": objective.description,
            "progress": objective.progress.as_ref().map(|progress| progress(game, p)).unwrap_or_default(),
            "conditionMet": objective.status_phase_check.as_ref().is_some_and(|check| check(game, p)),
            "scoringAge": game.age + u32::from(crate::status_phase::get_status_phase(game).is_some_and(|phase| !matches!(phase, crate::status_phase::StatusPhaseState::CompleteObjectives))),
            "timing": match objective.get_type() {
                ObjectiveType::Instant => "Instant",
                ObjectiveType::StatusPhase => "Status phase",
            }
        })).collect::<Vec<_>>()})
        })
        .collect::<Vec<_>>();
    let collect_type = PlayingActionType::Collect;
    let collect_reason = if crate::collect::available_collect_actions(game, seat).is_empty() {
        collect_type.is_available(game, seat).err()
    } else {
        None
    };
    let origin = collect_event_origin(&collect_type, p);
    let cities = p.cities.iter().map(|city| {
        let mut after_activation = crate::city::City::from_data(city.cloned_data(), seat);
        if city.is_activated() {
            after_activation.mood_state = match city.mood_state { MoodState::Happy => MoodState::Neutral, _ => MoodState::Angry };
        }
        let info = possible_resource_collections(game, city.position, seat, &origin, CostTrigger::NoModifiers);
        let mut choices = info.choices.iter().flat_map(|(position, piles)| piles.iter().map(move |pile| {
            let mut bonuses = vec![];
            if p.has_special_advance(crate::special_advance::SpecialAdvance::RiceCultivation)
                && is_special_advance_active(crate::special_advance::SpecialAdvance::RiceCultivation, p.advances, game)
                && crate::content::civilizations::china::rice_cultivation_tile(game, seat, city.position, *position) {
                bonuses.push(json!({"source":"Rice Cultivation","pile":ResourcePile::food(1),"limit":2}));
            }
            if pile.food == 1 && p.has_special_advance(crate::special_advance::SpecialAdvance::Canals)
                && is_special_advance_active(crate::special_advance::SpecialAdvance::Canals, p.advances, game) {
                bonuses.push(json!({"source":"Canals","pile":ResourcePile::food(1),"limit":1,"condition":"exactlyOneFood"}));
            }
            json!({"position":position,"pile":pile,"bonuses":bonuses})
        })).collect::<Vec<_>>();
        choices.sort_by_key(Value::to_string);
        let reason = if !can_play { Some("Wait for your turn".to_string()) } else if !city.can_activate() { Some("This city has already been activated while angry".to_string()) } else { collect_reason.clone() };
        json!({"position":city.position,"capital":city.position == crate::map::capital_city_position(game,p),"size":city.size(),"capacity":info.max_selection,"maxPerTile":info.max_per_tile,"maxRange2":info.max_range2_tiles,"mood":city.mood_state,"activations":city.activations,"reason":reason,"choices":choices,
            "collectionBonuses":if crate::content::civilizations::japan::pottery_available(game,p) {
                vec![json!({"source":"Pottery","minimum":ResourcePile::food(crate::content::civilizations::japan::POTTERY_FOOD),"pile":ResourcePile::culture_tokens(1)})]
            } else {vec![]},
            "ballcourts":crate::content::civilizations::maya::ballcourts_available(p,city.position),
            "piratePort":crate::content::civilizations::carthage::pirate_port(game,p,city.position),
            "shogunateDraftCost":crate::content::advances::warfare::draft_cost(p),
            "shogunateDraft":p.has_special_advance(crate::special_advance::SpecialAdvance::Shogunate)&&p.can_use_advance(Advance::Draft)&&!p.event_info.contains_key("Shogunate Draft"),
            "canActivate":city.can_activate(), "activationMood":after_activation.mood_state,"activationCapacity":after_activation.mood_modified_size(p)})
    }).collect::<Vec<_>>();
    let advance_choice = if seat == active {
        game.current_event_handler()
            .and_then(|handler| match &handler.request {
                PersistentEventRequest::SelectAdvance(request) if handler.response.is_none() => {
                    Some((&request.choices, decisions::advance_mode(game, handler)))
                }
                _ => None,
            })
    } else {
        None
    };
    let research_uses_event_marker = advance_choice.is_none()
        || game
            .current_event_handler()
            .is_some_and(|handler| decisions::advance_uses_event_marker(game, handler));
    let mut advances = game.cache.get_advances().iter().map(|(advance, info)| {
        let cost = p.advance_cost(*advance, game, CostTrigger::NoModifiers).cost;
        let payment = cost.first_valid_payment(&p.resources);
        let payment_options = if advance_choice.is_some_and(|(_, mode)| mode != "paid") {
            vec![ResourcePile::empty()]
        } else {
            advance_payments(&cost)
        };
        let owned = p.has_advance(*advance);
        let reason = if owned { Some("Already researched".to_string()) }
        else if let Some((choices, _)) = advance_choice {
            if choices.contains(advance) { None } else { Some("Unavailable for this choice".to_string()) }
        }
        else if !can_play { Some("Wait for your turn".to_string()) }
        else if let Err(reason) = PlayingActionType::Advance.is_available(game, seat) { Some(reason) }
        else if let Some(required) = info.required.filter(|a| !p.has_advance(*a)) { Some(format!("Requires {}", required.name(game))) }
        else if !p.can_advance_free(*advance, game) { Some("Conflicts with your current advances".to_string()) }
        else if payment.is_none() { Some("Not enough resources".to_string()) } else { None };
        let action = if reason.is_some() { None }
            else if advance_choice.is_some() { Some(Action::Response(EventResponse::SelectAdvance(*advance))) }
            else { payment.clone().map(|payment| Action::Playing(PlayingAction::Advance(AdvanceAction::new(*advance, payment)))) };
        let mut item = advance_description(game, *advance);
        let borrowing = advance_choice.is_some_and(|(_, mode)| mode == "borrow");
        if borrowing { item["bonus"] = Value::Null; }
        let philosophy_bonus = !owned && !borrowing && !game.is_update_patch()
            && p.can_use_advance(Advance::Philosophy)
            && game.cache.get_advance_group(AdvanceGroup::Science).advances.iter().any(|a| a.advance == *advance);
        item["bonusEffects"] = if philosophy_bonus {
            json!([{"source":"Philosophy","pile":ResourcePile::ideas(1)}])
        } else { json!([]) };
        let resources = ResourceType::all().into_iter().chain([ResourceType::Captives]).filter(|r| payment_options.iter().any(|p| p.get(r) > 0))
            .flat_map(|r| serde_json::to_value(ResourcePile::of(r, 1)).unwrap().as_object().unwrap().keys().cloned().collect::<Vec<_>>()).collect::<Vec<_>>();
        let cost_amount = payment_options.iter().map(ResourcePile::amount).min().unwrap_or(0);
        let cost_groups = payment_options.iter().map(ResourcePile::amount).collect::<std::collections::BTreeSet<_>>()
            .into_iter().map(|amount| {
                let resources = ResourceType::all().into_iter().chain([ResourceType::Captives])
                    .filter(|r| payment_options.iter().any(|p| p.amount() == amount && p.get(r) > 0))
                    .flat_map(|r| serde_json::to_value(ResourcePile::of(r, 1)).unwrap().as_object().unwrap().keys().cloned().collect::<Vec<_>>())
                    .collect::<Vec<_>>();
                json!({"amount":amount,"resources":resources})
            }).collect::<Vec<_>>();
        item.as_object_mut().unwrap().extend(json!({
            "triggersEvent":action.is_some() && p.incident_tokens == 1 && research_uses_event_marker,
            "owned":owned,"reason":reason,"payment":if advance_choice.is_some_and(|(_, mode)| mode != "paid") {ResourcePile::empty()} else {payment.unwrap_or_else(|| cost.default_payment())},"action":action,
            "costAmount":cost_amount,"costResources":resources,
            "costGroups":cost_groups,
            "payments":payment_options.into_iter().filter(|payment| advance_choice.is_none() && p.resources.has_at_least(payment)).map(|payment| {
                json!({"action":reason.is_none().then(|| Action::Playing(PlayingAction::Advance(AdvanceAction::new(*advance, payment.clone())))),"payment":payment})
            }).collect::<Vec<_>>()
        }).as_object().unwrap().clone());
        item
    }).collect::<Vec<_>>();
    advances.sort_by_key(|a| a["name"].as_str().unwrap_or_default().to_string());
    json!({"logOriginNames":log_origin_names,"cardCatalog":card_catalog,"eventCatalog":event_catalog,"pendingEvent":pending_event,"civilizationDraft":civilization_draft,"waitingFor":waiting_for,"activePlayers":active_players,"activePlayer":active,"canPlay":can_play,"supportedPhase":supported_phase,"players":players,"cities":cities,"advances":advances,"objectiveCards":objective_cards,"objectiveDecision":objective_decision(game, seat),
        "influenceContext":decisions::influence_context(game, seat), "choiceDecision":choice, "explorationDecision":exploration, "wonderCards":wonder_cards, "wonderCatalog":wonder_catalog, "builtWonders":built_wonders, "decision":decision,
        "civilizations":crate::game_setup::civilization_choices(game, seat).iter().map(|c|json!({"name":c.name,
            "advances":c.special_advances.iter().map(|a|json!({"name":a.name,"description":a.description,"requirement":a.requirement.name(game)})).collect::<Vec<_>>(),
            "leaders":c.leaders.iter().map(|l|json!({"name":l.name,"abilities":l.abilities.iter().map(|a|json!({"name":a.name,"description":a.description})).collect::<Vec<_>>()})).collect::<Vec<_>>(),
            "action":Action::ChooseCivilization(c.name.clone())})).collect::<Vec<_>>(),
        "actionCards":actions::cards(game,seat,can_play), "specialActions":actions::special(game,seat,can_play), "influence":actions::influence(game,seat,can_play),
        "collectActions":if can_play {crate::collect::available_collect_actions(game,seat).iter().map(|a|{ let cost = a.cost(game,seat); json!({"value":a,"name":a.origin(p).name(game),"free":cost.free,"payment":cost.payment_options(p,a.origin(p)).default_payment()}) }).collect::<Vec<_>>()} else {vec![]},
        "happinessActions":if can_play {crate::happiness::available_happiness_actions(game,seat).iter().map(|a|json!({"value":a,"name":a.origin(p).name(game),"free":a.cost(game,seat).free,"surcharge":a.payment_options(game,seat).default})).collect::<Vec<_>>()} else {vec![]},
        "units":p.units.iter().map(|u|json!({"id":u.id,"type":u.unit_type,"position":u.position,"carrier":u.carrier_id,"pirate":u.pirate,"movementNotes":movement_notes(game,p,u)})).collect::<Vec<_>>(),
        "nomadCities":p.cities.iter().filter(|c|!crate::content::civilizations::huns::city_destinations(game,p,c.position,&[]).is_empty()).map(|c|c.position).collect::<Vec<_>>(),
        "movementLeft":if let GameState::Movement(m)=&game.state {m.movement_actions_left} else {3},
        "barbarianGuide":barbarian_guide,"pirateSpawns":pirate_spawns,"seaRoutes":sea_routes,
        "endTurnTradeWarning":if can_play { turn::trade_warning(game, seat) } else { None },
        "cityActions":actions::cities(game, seat, can_play), "settlers":actions::settlers(game, seat, (can_play && PlayingActionType::MoveUnits.is_available(game,seat).is_ok()) || (moving && seat == active)),
        "stopMovement":if moving && seat == active {Some(Action::Movement(crate::movement::MovementAction::Stop))} else {None},
        "canUndo":seat == active && game.can_undo(),"canRedo":seat == active && game.can_redo(),"canEndTurn":can_play && PlayingActionType::EndTurn.is_available(game, seat).is_ok()})
}

fn movement_notes(
    game: &Game,
    player: &crate::player::Player,
    unit: &crate::unit::Unit,
) -> Vec<&'static str> {
    use crate::movement::MovementRestriction;
    let mut notes = unit
        .movement_restrictions
        .iter()
        .filter(|restriction| {
            **restriction != MovementRestriction::Fertile
                || game.players.iter().any(|opponent| {
                    opponent.index != player.index
                        && game.can_attack_player(player.index, opponent.index)
                        && opponent.cities.iter().any(|city| {
                            city.pieces
                                .wonders
                                .contains(&crate::wonder::Wonder::GreatGardens)
                        })
                })
        })
        .map(|restriction| match restriction {
            MovementRestriction::Forest if player.can_use_advance(Advance::Roads) => {
                "Forest · Attacking requires a Roads route this turn"
            }
            MovementRestriction::Forest => "Forest · Cannot attack this turn",
            MovementRestriction::Mountain if player.can_use_advance(Advance::Roads) => {
                "Mountain · Moving requires a Roads route this turn"
            }
            MovementRestriction::Mountain => "Mountain · Cannot move again this turn",
            MovementRestriction::Battle => "Battle · Cannot move again this turn",
            MovementRestriction::Navigator => "Navigator · Cannot move again this turn",
            MovementRestriction::Fertile => "Cannot attack the Great Gardens this turn",
        })
        .collect::<Vec<_>>();
    if unit.is_army_unit() && !player.can_use_advance(Advance::Tactics) {
        notes.push("Requires Tactics to move");
    }
    if let GameState::Movement(movement) = &game.state {
        if player.index == game.current_player_index
            && unit.is_land_based()
            && movement.current_move.is_none()
            && movement.moved_units.contains(&unit.id)
        {
            notes.push(
                "Already moved in this Move action · Remaining group moves are for other units",
            );
        }
    }
    notes.sort_unstable();
    notes.dedup();
    notes
}

fn advance_description(game: &Game, advance: Advance) -> Value {
    let info = advance.info(game);
    let group =
        game.cache
            .get_advance_groups()
            .iter()
            .enumerate()
            .find_map(|(group_index, group)| {
                group
                    .advances
                    .iter()
                    .position(|a| a.advance == advance)
                    .map(|index| (group.name.clone(), group_index * 10 + index))
            });
    json!({"id":advance,"name":info.name,"description":info.description,
        "group":group.as_ref().map(|g| &g.0),"order":group.as_ref().map(|g| g.1),"required":info.required,
        "bonus":info.bonus.as_ref().map(|bonus| bonus.resources()),"unlocks":info.unlocked_building.map(|building| building.to_string())})
}

// Research conversions replace resources one-for-one or waive their cost. Enumerate
// the small set of possible splits, validating every payment against the engine.
// Keeping unaffordable splits here lets the UI describe the cost independently of stock.
fn advance_payments(cost: &PaymentOptions) -> Vec<ResourcePile> {
    fn visit(
        cost: &PaymentOptions,
        types: &[ResourceType],
        remaining: u8,
        pile: ResourcePile,
        result: &mut Vec<ResourcePile>,
    ) {
        let Some((resource, rest)) = types.split_first() else {
            if cost.is_valid_payment(&pile) {
                result.push(pile);
            }
            return;
        };
        for amount in 0..=remaining {
            let mut next = pile.clone();
            next.add_type(*resource, i32::from(amount));
            visit(cost, rest, remaining - amount, next, result);
        }
    }
    let mut types = cost.possible_resource_types();
    types.sort();
    types.dedup();
    let mut result = Vec::new();
    visit(
        cost,
        &types,
        cost.default_payment().amount(),
        ResourcePile::empty(),
        &mut result,
    );
    result
}

fn exploration_decision(game: &Game, seat: usize) -> Option<Value> {
    let event = game.events.last()?;
    if event.player.index != seat {
        return None;
    }
    let PersistentEventType::ExploreResolution(state) = &event.event_type else {
        return None;
    };
    let handler = game.current_event_handler()?;
    if handler.response.is_some()
        || !matches!(handler.request, PersistentEventRequest::ExploreResolution)
    {
        return None;
    }
    // The engine has already revealed this region and applied forced-placement rules.
    // Only the original and opposite orientations are valid for this request.
    let base = state.block.position.rotation;
    let choices = [base, (base + 3) % 6].map(|rotation| {
        json!({"rotation":rotation,"tiles":state.block.block.tiles(&state.block.position,rotation),
            "action":Action::Response(EventResponse::ExploreResolution(rotation))})
    });
    Some(json!({"start":state.start,"destination":state.destination,"choices":choices}))
}

fn choice_decision(game: &Game, seat: usize) -> Option<Value> {
    use crate::resource::ResourceType;
    use crate::resource_pile::ResourcePile;
    let event = game.events.last()?;
    if event.player.index != seat {
        return None;
    }
    let handler = game.current_event_handler()?;
    if handler.response.is_some() {
        return None;
    }
    match &handler.request {
        PersistentEventRequest::ResourceReward(request)
            if request.reward.payment_options.default.amount() == 1 =>
        {
            let choices = ResourceType::all().into_iter().map(|r| ResourcePile::of(r,1))
                .filter(|pile|request.reward.payment_options.is_valid_payment(pile))
                .map(|pile|json!({"name":pile.to_string(),"pile":pile,"action":Action::Response(EventResponse::ResourceReward(pile.clone()))})).collect::<Vec<_>>();
            Some(json!({"name":request.name,"choices":choices}))
        }
        PersistentEventRequest::BoolRequest(name) => {
            let preview = if handler.origin.name(game) == "Great Mausoleum" {
                match &event.event_type {
                    PersistentEventType::ChooseIncident(_) => game.incidents_discarded.last().map(|id| {
                        let card = game.cache.get_incident(*id);
                        json!({"name":card.name,"rules":card.description(game),
                            "affected":card.targets.iter().map(|t|match t {
                                crate::player_events::IncidentTarget::AllPlayers => "All players",
                                crate::player_events::IncidentTarget::ActivePlayer => "Active player",
                                crate::player_events::IncidentTarget::SelectedPlayer => "Selected player",
                            }).collect::<Vec<_>>().join(" · ")})
                    }),
                    PersistentEventType::ChooseActionCard => game.action_cards_discarded.last().map(|id| {
                        let card = game.cache.get_action_card(*id);
                        json!({"name":card.civil_card.name,"rules":[card.civil_card.description]})
                    }),
                    _ => None,
                }
            } else {
                None
            };
            Some(json!({"name":name,"preview":preview,"choices":[
                {"name":"Yes","action":Action::Response(EventResponse::Bool(true))},
                {"name":"No","action":Action::Response(EventResponse::Bool(false))}]}))
        }
        _ => None,
    }
}

fn objective_decision(game: &Game, seat: usize) -> Option<Value> {
    let event = game.events.last()?;
    if seat != event.player.index {
        return None;
    }
    let PersistentEventType::SelectObjectives(info) = &event.event_type else {
        return None;
    };
    let name = info.shown_objective.as_ref()?;
    let handler = game.current_event_handler()?;
    let PersistentEventRequest::SelectHandCards(request) = &handler.request else {
        return None;
    };
    if handler.response.is_some() || !request.needed.contains(&1) {
        return None;
    }
    let cards = request.choices.iter().filter_map(|card| {
        let HandCard::ObjectiveCard(id) = card else { return None; };
        if *id == 0 || !game.player(seat).objective_cards.contains(id) { return None; }
        let info = game.cache.get_objective_card(*id);
        Some(json!({"id":id,"name":info.name(),"action":Action::Response(EventResponse::SelectHandCards(vec![card.clone()]))}))
    }).collect::<Vec<_>>();
    let skip = request
        .needed
        .contains(&0)
        .then(|| Action::Response(EventResponse::SelectHandCards(vec![])));
    Some(
        json!({"name":name,"description":game.cache.get_objective(name).description,"points":OBJECTIVE_VICTORY_POINTS,"cards":cards,"skip":skip}),
    )
}

pub fn collect_preview(
    game: &Game,
    seat: usize,
    city: Position,
    selections: Vec<PositionCollection>,
) -> Result<Value, String> {
    collect_variant_preview(
        game,
        seat,
        city,
        selections,
        PlayingActionType::Collect,
        false,
    )
}

fn collect_variant_preview(
    game: &Game,
    seat: usize,
    city: Position,
    selections: Vec<PositionCollection>,
    kind: PlayingActionType,
    ballcourts: bool,
) -> Result<Value, String> {
    if seat >= game.players.len() || !game.player(seat).is_human() || seat != game.active_player() {
        return Err("Wait for your turn".to_string());
    }
    if !crate::collect::available_collect_actions(game, seat).contains(&kind) {
        return Err("Collection action unavailable".into());
    }
    let p = game.player(seat);
    let city_data = p.try_get_city(city).ok_or("Choose one of your cities")?;
    if !city_data.can_activate() {
        return Err("This city cannot be activated again this turn".to_string());
    }
    if selections.iter().any(|c| c.times == 0) {
        return Err("Choose at least one resource".to_string());
    }
    let origin = collect_event_origin(&kind, p);
    let collection = crate::collect::get_total_collection_with_ballcourts(
        game,
        seat,
        &origin,
        city,
        &selections,
        CostTrigger::NoModifiers,
        ballcourts,
    )?;
    if ballcourts
        && p.resources.mood_tokens
            < kind
                .payment_options(game, seat)
                .first_valid_payment(&p.resources)
                .map_or(0, |pile| pile.mood_tokens)
                + 1
    {
        return Err("Not enough mood for this action and Ballcourts".into());
    }
    let mut effects = collection
        .info
        .log
        .iter()
        .map(|(origin, description)| json!({"source":origin.name(game),"description":description}))
        .collect::<Vec<_>>();
    let mut total = collection.total;
    let pottery = crate::content::civilizations::japan::pottery_bonus(game, p, total.food);
    if !pottery.is_empty() {
        effects.push(json!({"source":"Pottery","description":format!("Gain {pottery}")}));
        total += pottery;
    }
    let mut after = p.resources.clone() + total.clone();
    let waste = after.apply_resource_limit(&p.resource_limit);
    let mut collect = Collect::new(city, selections, kind);
    collect.ballcourts = ballcourts;
    let action = Action::Playing(PlayingAction::Collect(collect));
    Ok(
        json!({"action":action,"total":total,"effects":effects,"waste":waste,"after":after,"moodWillDecrease":city_data.is_activated()}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{game::GameOptions, game_api, resource_pile::ResourcePile};
    fn game() -> Game {
        game_api::init(2, "web-view-test".into(), GameOptions::default())
    }
    #[test]
    fn spectators_and_waiting_players_have_no_actions() {
        let game = game();
        assert_eq!(view(&game, None)["canPlay"], false);
        assert_eq!(view(&game, Some(99))["cities"], json!([]));
        let waiting = view(&game, Some(1 - game.active_player()));
        assert_eq!(waiting["canPlay"], false);
        assert!(
            waiting["advances"]
                .as_array()
                .unwrap()
                .iter()
                .all(|a| a["action"].is_null())
        );
    }
    #[test]
    fn collect_preview_uses_rules_and_does_not_mutate_state() {
        let game = game();
        let before = serde_json::to_string(&game.cloned_data()).unwrap();
        let seat = game.active_player();
        let city = game.player(seat).cities[0].position;
        let preview = collect_preview(
            &game,
            seat,
            city,
            vec![PositionCollection::new(city, ResourcePile::food(1))],
        )
        .unwrap();
        let action: Action = serde_json::from_value(preview["action"].clone()).unwrap();
        assert!(crate::action::try_execute_action(game.clone(), action, seat).is_ok());
        assert_eq!(before, serde_json::to_string(&game.cloned_data()).unwrap());
        assert!(collect_preview(&game, 1 - seat, city, vec![]).is_err());
        assert!(
            collect_preview(
                &game,
                seat,
                city,
                vec![PositionCollection::new(city, ResourcePile::gold(7))]
            )
            .is_err()
        );
    }
}

fn research_plan(game: &Game, seat: usize, input: &Value) -> Result<Value, String> {
    let action: Action =
        serde_json::from_value(input["action"].clone()).map_err(|e| e.to_string())?;
    let Action::Playing(PlayingAction::Advance(a)) = &action else {
        return Err("Choose a research action".into());
    };
    let p = game.player(seat);
    let eligible = p.can_use_advance(Advance::FreeEducation)
        && a.advance != Advance::FreeEducation
        && (a.payment.ideas > 0 || a.payment.gold > 0);
    if !eligible {
        return Ok(json!({"eligible":false,"affordable":false}));
    }
    let extra_idea = a.advance == Advance::Philosophy
        || (p.can_use_advance(Advance::Philosophy)
            && matches!(
                a.advance,
                Advance::Math | Advance::Astronomy | Advance::Medicine | Advance::Metallurgy
            ));
    if p.resources.ideas.saturating_sub(a.payment.ideas) + u8::from(extra_idea) == 0 {
        return Ok(json!({"eligible":true,"affordable":false}));
    }
    let preview = crate::action::try_execute_action(game.clone(), action, seat)?;
    let matches = preview
        .current_event_handler()
        .is_some_and(|h| h.origin == crate::events::EventOrigin::Advance(Advance::FreeEducation));
    if !matches {
        return Ok(json!({"eligible":true,"affordable":false}));
    }
    let available = decisions::preview(
        &preview,
        seat,
        &json!({"values":[],"payments":[ResourcePile::ideas(1)]}),
    )
    .is_ok();
    Ok(json!({"eligible":true,"affordable":available,"combined":true}))
}
