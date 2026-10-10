use crate::action::Action;
use crate::city::MoodState;
use crate::city_pieces::BUILDINGS;
use crate::construct::{Construct, can_construct, new_building_positions};
use crate::content::effects::{ConstructEffect, PermanentEffect};
use crate::game::{Game, GameState};
use crate::happiness::{
    IncreaseHappiness, happiness_base_event_origin, happiness_cost_for_cities, lawgiver_city,
};
use crate::map::Terrain;
use crate::movement::{MoveUnits, MovementAction, possible_move_routes};
use crate::player::CostTrigger;
use crate::playing_actions::{PlayingAction, PlayingActionType};
use crate::position::Position;
use crate::recruit::{Recruit, recruit_cost};
use crate::unit::{UnitType, Units};
use serde_json::{Value, json};

pub(super) fn leader_unavailable_reason(
    p: &crate::player::Player,
    leader: crate::leader::Leader,
) -> Option<String> {
    if let Some(unit) = p
        .units
        .iter()
        .find(|u| u.unit_type == UnitType::Leader(leader))
    {
        return Some(format!("Already on board at {}", unit.position));
    }
    if p.recruited_leaders.contains(&leader) {
        return Some("Killed or replaced · Cannot recruit again".into());
    }
    // Guillotine removes the remaining unplayed leaders from the supply.
    (!p.available_leaders.contains(&leader)).then(|| "Unavailable after Guillotine".into())
}

fn recruit_cost_options(game: &Game, cost: &crate::player_events::CostInfo) -> Vec<String> {
    let mut names = Vec::new();
    for (origin, _) in &cost.info.log {
        let name = origin.name(game);
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

pub fn recruit_extended(game: &Game, seat: usize, input: &Value) -> Result<Value, String> {
    if seat != game.active_player() {
        return Err("Wait for your turn".into());
    }
    PlayingActionType::Recruit.is_available(game, seat)?;
    let units: Units = serde_json::from_value(input["units"].clone()).map_err(|e| e.to_string())?;
    let position: Position =
        serde_json::from_value(input["city"].clone()).map_err(|e| e.to_string())?;
    let replaced: Vec<u32> =
        serde_json::from_value(input["replaced"].clone()).map_err(|e| e.to_string())?;
    let p = game.player(seat);
    let city = p.try_get_city(position).ok_or("Choose your city")?;
    let draft_card = input["draftCard"].as_bool().unwrap_or(false);
    if units.is_empty() && !draft_card {
        return Err("Choose units to recruit".into());
    }
    if units
        .leader
        .is_some_and(|l| !p.available_leaders.contains(&l))
    {
        return Err("Leader unavailable".into());
    }
    if replaced
        .iter()
        .any(|id| !p.units.iter().any(|u| u.id == *id))
        || replaced
            .iter()
            .enumerate()
            .any(|(i, id)| replaced[..i].contains(id))
    {
        return Err("Choose units to replace".into());
    }
    let ballcourts = input["ballcourts"].as_bool().unwrap_or(false);
    let cost = crate::recruit::recruit_cost_with_options(
        game,
        p,
        &units,
        position,
        &replaced,
        CostTrigger::NoModifiers,
        ballcourts,
        draft_card,
    )?;
    let payment = cost
        .cost
        .first_valid_payment(&p.resources)
        .ok_or("Not enough resources")?;
    let mut recruit =
        Recruit::new(&units, position, payment.clone()).with_replaced_units(&replaced);
    recruit.ballcourts = ballcourts;
    recruit.draft_card = draft_card;
    recruit.attack_pirates = input["attackPirates"].as_bool().unwrap_or(false);
    if let Some(payment) = input.get("payment").filter(|p| !p.is_null()) {
        let payment: crate::resource_pile::ResourcePile =
            serde_json::from_value(payment.clone()).map_err(|e| e.to_string())?;
        if !p.resources.has_at_least(&payment) || !cost.cost.is_valid_payment(&payment) {
            return Err("Choose a valid recruitment payment".into());
        }
        recruit.payment = payment;
    }
    crate::content::civilizations::carthage::validate_recruit(game, p, &recruit)?;
    let payments = super::decisions::payment_choices(&cost.cost, &p.resources, false, false);
    Ok(json!({"payment":recruit.payment,"payments":payments,
            "basePayment":units.clone().to_vec().iter().map(UnitType::cost).sum::<crate::resource_pile::ResourcePile>(),
            "costOptions":recruit_cost_options(game, &cost),
            "moodWillDecrease":city.is_activated(),"action":Action::Playing(PlayingAction::Recruit(recruit))}))
}

pub fn happiness_preview(game: &Game, seat: usize, input: &Value) -> Result<Value, String> {
    if seat != game.active_player() {
        return Err("Wait for your turn".into());
    }
    let kind: PlayingActionType =
        serde_json::from_value(input["variant"].clone()).map_err(|e| e.to_string())?;
    if !crate::happiness::available_happiness_actions(game, seat).contains(&kind) {
        return Err("Happiness action unavailable".into());
    }
    let selections: Vec<(Position, u8)> =
        serde_json::from_value(input["cities"].clone()).map_err(|e| e.to_string())?;
    let p = game.player(seat);
    let restriction = crate::happiness::happiness_city_restriction(p, &kind);
    if selections.is_empty() {
        return Err("Choose cities to improve".into());
    }
    for (i, (pos, n)) in selections.iter().enumerate() {
        let city = p.try_get_city(*pos).ok_or("Choose your city")?;
        let max = match city.mood_state {
            MoodState::Happy => 0,
            MoodState::Neutral => 1,
            MoodState::Angry => 2,
        };
        if *n == 0
            || *n > max
            || selections[..i].iter().any(|(p, _)| p == pos)
            || restriction.is_some_and(|p| p != *pos)
        {
            return Err("Choose valid happiness increases".into());
        }
    }
    let origin = crate::happiness::happiness_event_origin(&kind, p);
    let lawgiver = input["lawgiver"].as_bool().unwrap_or(false);
    let cost = happiness_cost_for_cities(
        game,
        seat,
        &selections,
        lawgiver,
        CostTrigger::NoModifiers,
        &kind,
        &origin,
    )?;
    let affordable = cost.cost.first_valid_payment(&p.resources);
    let payment = affordable
        .clone()
        .unwrap_or_else(|| cost.cost.default_payment());
    let action = affordable.map(|payment| {
        let mut increase = IncreaseHappiness::new(selections, payment, kind);
        increase.lawgiver = lawgiver;
        Action::Playing(PlayingAction::IncreaseHappiness(increase))
    });
    Ok(json!({
        "payment":payment,
        "reason":action.is_none().then_some("Not enough resources"),
        "action":action
    }))
}

fn terrain_notes(
    game: &Game,
    player: &crate::player::Player,
    units: &[u32],
    destination: Position,
) -> Vec<&'static str> {
    let mut notes = vec![];
    match game.map.get(destination) {
        Some(Terrain::Mountain) => {
            if crate::content::civilizations::carthage::ignores_mountains(player, units) {
                notes.push("Hannibal with elephants: ignores the mountain movement stop.");
            } else if player.has_special_advance(crate::special_advance::SpecialAdvance::Terracing)
            {
                notes.push("Terracing: settlers and leaders ignore the mountain movement stop.");
                if units.iter().any(|id| {
                    let u = player.get_unit(*id);
                    !u.is_settler() && !u.is_leader()
                }) {
                    notes.push("Other units must stop here for the rest of the turn.");
                }
            } else {
                notes.push("After entering: these units cannot move again this turn.");
            }
        }
        Some(Terrain::Forest) => {
            notes.push(
                if units.iter().any(|id| player.get_unit(*id).is_army_unit()) {
                    "After this move: army units may move again, but cannot make a later attack this turn."
                } else {
                    "No forest movement restriction for these units."
                },
            );
        }
        _ => return notes,
    }
    if player.can_use_advance(crate::advance::Advance::Roads) {
        notes.push("A Roads route can bypass terrain movement restrictions.");
    }
    notes
}

pub fn movement(game: &Game, seat: usize, units: Vec<u32>) -> Result<Value, String> {
    let p = game.player(seat);
    if seat != game.active_player() || !game.events.is_empty() {
        return Err("Finish the current decision first".into());
    }
    if !matches!(game.state, GameState::Movement(_)) {
        PlayingActionType::MoveUnits.is_available(game, seat)?;
    }
    if units.is_empty() {
        return Ok(json!({"destinations":[]}));
    }
    if units.iter().any(|id| !p.units.iter().any(|u| u.id == *id))
        || units.iter().collect::<std::collections::HashSet<_>>().len() != units.len()
    {
        return Err("Choose your units".into());
    }
    let start = p.get_unit(units[0]).position;
    let carriers = std::iter::once(None).chain(
        p.units
            .iter()
            .filter(|u| u.is_ship() && !units.contains(&u.id))
            .map(|u| Some(u.id)),
    );
    let mut destinations = vec![];
    for carrier in carriers {
        for route in possible_move_routes(p, game, &units, start, carrier).unwrap_or_default() {
            if let Some(payment) = route.cost.first_valid_payment(&p.resources) {
                let breaks_diplomacy =
                    route
                        .cost
                        .modifiers
                        .contains(&crate::events::EventOrigin::Incident(
                            crate::content::incidents::great_diplomat::DIPLOMAT_ID,
                        ));
                let offer = json!({"position":route.destination,"terrain":game.map.get(route.destination),"terrainNotes":terrain_notes(game,p,&units,route.destination),"payment":payment,"carrier":carrier,"attack":game.enemy_player(seat,route.destination).is_some(),"breaksDiplomacy":breaks_diplomacy,
                    "action":Action::Movement(MovementAction::Move(MoveUnits::new(units.clone(),route.destination,carrier,payment)))});
                if !destinations.contains(&offer) {
                    destinations.push(offer);
                }
            }
        }
    }
    if p.has_special_advance(crate::special_advance::SpecialAdvance::PirateAllies) {
        let pirates = crate::content::civilizations::carthage::pirate_player(game);
        if units.iter().all(|id| !p.get_unit(*id).is_ship()) {
            for ship in &game.player(pirates).units {
                let mut preview = game.clone();
                let carrier = crate::content::civilizations::carthage::claim_pirate(
                    &mut preview,
                    seat,
                    ship.id,
                );
                for route in possible_move_routes(
                    preview.player(seat),
                    &preview,
                    &units,
                    start,
                    Some(carrier),
                )
                .unwrap_or_default()
                {
                    if route.destination != ship.position {
                        continue;
                    }
                    if let Some(payment) = route.cost.first_valid_payment(&p.resources) {
                        let mut action =
                            MoveUnits::new(units.clone(), route.destination, None, payment.clone());
                        action.embark_pirate = Some(ship.id);
                        destinations.push(json!({"position":route.destination,"terrain":game.map.get(route.destination),"payment":payment,"carrier":null,"pirateCarrier":ship.id,"label":"Board allied pirate","attack":false,"action":Action::Movement(MovementAction::Move(action))}));
                    }
                }
            }
        } else if units
            .iter()
            .all(|id| p.get_unit(*id).is_ship() && !p.get_unit(*id).pirate)
        {
            let targets = game
                .player(pirates)
                .units
                .iter()
                .map(|u| u.position)
                .chain(p.units.iter().filter(|u| u.pirate).map(|u| u.position))
                .collect::<std::collections::BTreeSet<_>>();
            for target in targets {
                let mut preview = game.clone();
                let mut action = MoveUnits::new(
                    units.clone(),
                    target,
                    None,
                    crate::resource_pile::ResourcePile::empty(),
                );
                action.attack_pirates = true;
                if crate::content::civilizations::carthage::prepare_move(
                    &mut preview,
                    seat,
                    &mut action,
                )
                .is_err()
                {
                    continue;
                }
                if let Some(route) =
                    possible_move_routes(preview.player(seat), &preview, &units, start, None)
                        .unwrap_or_default()
                        .into_iter()
                        .find(|r| r.destination == target)
                {
                    if let Some(payment) = route.cost.first_valid_payment(&p.resources) {
                        action.payment = payment.clone();
                        destinations.push(json!({"position":target,"terrain":game.map.get(target),"payment":payment,"carrier":null,"label":"Attack pirates","attack":true,"action":Action::Movement(MovementAction::Move(action))}));
                    }
                }
            }
        }
    }
    destinations.sort_by_key(|d| d["position"].as_str().unwrap_or_default().to_owned());
    Ok(json!({"destinations":destinations}))
}

pub fn cards(game: &Game, seat: usize, can_play: bool) -> Vec<Value> {
    game.player(seat).action_cards.iter().filter(|id|**id!=0).map(|id| {
        let c = game.cache.get_action_card(*id);
        let reason = action_reason(game,seat,can_play,PlayingActionType::ActionCard(*id));
        json!({"id":id,"name":c.civil_card.name,"description":c.civil_card.description,"free":PlayingActionType::ActionCard(*id).cost(game,seat).free,
            "cost":PlayingActionType::ActionCard(*id).payment_options(game,seat).default,
            "tactics":c.tactics_card.as_ref().map(|t|json!({"name":t.name,"description":t.description})),
            "reason":reason,"action":reason.is_none().then(||Action::Playing(PlayingAction::ActionCard(*id)))})
    }).collect()
}

pub fn special(game: &Game, seat: usize, can_play: bool) -> Vec<Value> {
    use crate::content::custom_actions::{CustomAction, SpecialActionExecution};
    game.player(seat).special_actions.values().flat_map(|info| {
        let SpecialActionExecution::Action(execution) = &info.execution else { return vec![]; };
        let mut reason = if seat != game.active_player() {
            Some("Wait for your turn".to_string())
        } else if !can_play {
            Some("Finish the current decision first".to_string())
        } else {
            info.action.playing_action_type().is_available(game, seat).err()
        };
        let mut cities: Vec<Option<Position>> = if info.city_bound().is_some() {
            game.player(seat).cities.iter().filter(|c|info.is_city_available(game,c)).map(|c|Some(c.position)).collect()
        } else { vec![None] };
        if cities.is_empty() {
            reason.get_or_insert_with(|| "Custom action cannot be played".to_string());
            cities.push(None);
        }
        cities.into_iter().map(|city|json!({"name":info.event_origin.name(game),"description":execution.ability.description,"position":city,"reason":reason,
            "cost":info.cost.cost.payment_options(game.player(seat),info.event_origin.clone()).default_payment(),
            "free":info.cost.cost.free,
            "activatesCity":(info.custom_action_type() == crate::content::custom_actions::CustomActionType::GoldenAge)
                .then(|| crate::leader::leader_position(game.player(seat))),
            "action":Action::Playing(PlayingAction::Custom(CustomAction::new(info.custom_action_type(),city)))})).collect::<Vec<_>>()
    }).collect()
}

pub fn influence(game: &Game, seat: usize, can_play: bool) -> Vec<Value> {
    use crate::cultural_influence::{
        InfluenceCultureAttempt, available_influence_actions, available_influence_culture,
        influence_culture_boost_cost_from, influence_start_positions,
    };
    if !can_play {
        return vec![];
    }
    let mut offers: Vec<Value> = available_influence_actions(game,seat).into_iter().flat_map(|kind| {
        available_influence_culture(game,seat,&kind).into_iter().filter_map(|(s,r)| {
            let info = r.ok()?;
            let origins = influence_start_positions(game,game.player(seat)).into_iter().map(|(p,_)|p)
                .chain(std::iter::once(s.position))
                .collect::<std::collections::BTreeSet<_>>().into_iter().filter_map(|origin| {
                    let from = influence_culture_boost_cost_from(game,seat,&s,&kind,true,false,game.get_any_city(s.position).player_index,Some(origin)).ok()?;
                    let mut attempt = InfluenceCultureAttempt::new(s.clone(),kind.clone());
                    attempt.starting_position = Some(origin);
                    Some(json!({"position":origin,"settlers":game.try_get_any_city(origin).is_none(),
                        "reroll":crate::content::civilizations::india::buddhism_available(game,seat,&from),
                        "free":kind.cost(game,seat).free,"actionPayment":kind.cost(game,seat).payment_options(game.player(seat), kind.origin(game.player(seat))).default_payment(),"rollBonus":from.roll_boost,"preventBoost":from.prevent_boost,"payment":from.range_boost_cost.default_payment(),
                        "action":Action::Playing(PlayingAction::InfluenceCultureAttempt(attempt))}))
                }).collect::<Vec<_>>();
            Some(json!({"name":super::decisions::structure_name(&s.structure),"position":s.position,"origin":info.starting_city_position,"variant":kind.origin(game.player(seat)).name(game),"free":kind.cost(game,seat).free,"actionPayment":kind.cost(game,seat).payment_options(game.player(seat), kind.origin(game.player(seat))).default_payment(),"rollBonus":info.roll_boost,"preventBoost":info.prevent_boost,"payment":info.range_boost_cost.default_payment(),
                "origins":origins,
                "action":Action::Playing(PlayingAction::InfluenceCultureAttempt(InfluenceCultureAttempt::new(s,kind.clone())))}))
        }).collect::<Vec<_>>()
    }).collect();
    if game
        .player(seat)
        .has_special_advance(crate::special_advance::SpecialAdvance::Zoroastrianism)
    {
        for kind in available_influence_actions(game, seat) {
            for owner in &game.players {
                for unit in &owner.units {
                    let mut attempt = InfluenceCultureAttempt::new(
                        crate::content::persistent_events::SelectedStructure::new(
                            unit.position,
                            crate::structure::Structure::CityCenter,
                        ),
                        kind.clone(),
                    );
                    attempt.target_unit = Some(crate::cultural_influence::InfluenceUnit {
                        player: owner.index,
                        unit: unit.id,
                    });
                    let Ok(info) =
                        crate::cultural_influence::unit_influence_cost(game, seat, &attempt, true)
                    else {
                        continue;
                    };
                    let origins=influence_start_positions(game,game.player(seat)).into_iter().filter_map(|(pos,_)| {
                    let mut selected=attempt.clone();selected.starting_position=Some(pos);
                    let from=crate::cultural_influence::unit_influence_cost(game,seat,&selected,true).ok()?;
                    Some(json!({"position":pos,"settlers":false,"reroll":false,"free":kind.cost(game,seat).free,"actionPayment":kind.cost(game,seat).payment_options(game.player(seat), kind.origin(game.player(seat))).default_payment(),"rollBonus":from.roll_boost,"preventBoost":from.prevent_boost,"payment":from.range_boost_cost.default_payment(),"action":Action::Playing(PlayingAction::InfluenceCultureAttempt(selected))}))
                }).collect::<Vec<_>>();
                    offers.push(json!({"name":format!("{} · {} #{}",owner.civilization.name,unit.unit_type.non_leader_name(),unit.id+1),"position":unit.position,"origin":info.starting_city_position,"variant":"Zoroastrianism","free":kind.cost(game,seat).free,"actionPayment":kind.cost(game,seat).payment_options(game.player(seat), kind.origin(game.player(seat))).default_payment(),"rollBonus":info.roll_boost,"preventBoost":info.prevent_boost,"payment":info.range_boost_cost.default_payment(),"origins":origins,"action":Action::Playing(PlayingAction::InfluenceCultureAttempt(attempt))}));
                }
            }
        }
    }
    offers
}

fn action_reason(
    game: &Game,
    seat: usize,
    can_play: bool,
    kind: PlayingActionType,
) -> Option<String> {
    if !can_play {
        Some("Wait for your turn or finish the current decision".into())
    } else {
        kind.is_available(game, seat).err()
    }
}

pub fn cities(game: &Game, seat: usize, can_play: bool) -> Vec<Value> {
    let p = game.player(seat);
    // These cards grant an action to offset the following Construct command.
    // Present the extra build's net cost, not that internal action debit.
    let construction_source = (seat == game.active_player())
        .then(|| {
            game.permanent_effects
                .iter()
                .find_map(|effect| match effect {
                    PermanentEffect::Construct(ConstructEffect::GreatEngineer) => {
                        Some("Great Engineer")
                    }
                    PermanentEffect::Construct(ConstructEffect::CityDevelopment) => {
                        Some("City Development")
                    }
                    _ => None,
                })
        })
        .flatten();
    let free_construction =
        construction_source.is_some() || PlayingActionType::Construct.cost(game, seat).free;
    p.cities.iter().map(|city| {
        let buildings = BUILDINGS.into_iter().map(|building| {
            let cost = p.building_cost_in_city(game, building, city.position, CostTrigger::NoModifiers);
            let payment = cost.cost.first_valid_payment(&p.resources).unwrap_or_else(|| cost.cost.default_payment());
            let positions = new_building_positions(game, building, city);
            let reason = action_reason(game, seat, can_play, PlayingActionType::Construct)
                .or_else(|| can_construct(city, building, p, game, CostTrigger::NoModifiers, &[]).err())
                .or_else(|| positions.is_empty().then(|| "Needs an adjacent sea tile".to_string()));
            let choices = if reason.is_none() {positions.into_iter().map(|port| json!({"position":port,
                "action":Action::Playing(PlayingAction::Construct(Construct::new(city.position, building, payment.clone()).with_port_position(port)))})).collect::<Vec<_>>()} else {vec![]};
            json!({"name":building.to_string(),"owned":!city.pieces.can_add_building(building),
                "required":game.cache.get_building_advance(building).name(game),"payment":payment,
                "payments":super::decisions::payment_choices(&cost.cost,&p.resources,false,false),"reason":reason,"choices":choices,
                "free":free_construction,"activateCity":cost.activate_city,"source":construction_source,
                "moodWillDecrease":cost.activate_city && city.is_activated()})
        }).collect::<Vec<_>>();
        let recruits = [UnitType::Settler,UnitType::Infantry,UnitType::Cavalry,UnitType::Elephant,UnitType::Ship].into_iter().map(|unit| {
            let mut units = Units::empty(); units += &unit;
            let result = crate::recruit::recruit_cost_without_replaced(game,p,&units,city.position,CostTrigger::NoModifiers);
            let reason = action_reason(game,seat,can_play,PlayingActionType::Recruit).or_else(||result.as_ref().err().cloned());
            let cost_options = result.as_ref().map(|c| recruit_cost_options(game, c)).unwrap_or_default();
            let payment = result.map(|c|c.cost.first_valid_payment(&p.resources).unwrap_or_else(||c.cost.default_payment())).unwrap_or_else(|_|unit.cost());
            json!({"type":unit,"payment":payment,"basePayment":unit.cost(),"costOptions":cost_options,
                "reason":reason,"available":p.available_units().get_amount(&unit),"limit":p.unit_limit().get_amount(&unit)})
        }).collect::<Vec<_>>();
        let max_steps = match city.mood_state {MoodState::Happy=>0, MoodState::Neutral=>1, MoodState::Angry=>2};
        let happiness = (1..=max_steps).map(|steps| (steps, false))
            .chain((max_steps > 0 && lawgiver_city(p) == Some(city.position)).then_some((max_steps, true)))
            .map(|(steps, lawgiver)| {
                let kind = PlayingActionType::IncreaseHappiness;
                let cost = happiness_cost_for_cities(game, seat, &[(city.position,steps)], lawgiver, CostTrigger::NoModifiers, &kind, &happiness_base_event_origin()).expect("valid happiness target");
                let payment = cost.cost.first_valid_payment(&p.resources);
                let reason = action_reason(game,seat,can_play,kind.clone()).or_else(||payment.is_none().then(||"Not enough resources".into()));
                let payment = payment.unwrap_or_else(||cost.cost.default_payment());
                let action = reason.is_none().then(|| {
                    let mut increase = IncreaseHappiness::new(vec![(city.position,steps)],payment.clone(),kind);
                    increase.lawgiver = lawgiver;
                    Action::Playing(PlayingAction::IncreaseHappiness(increase))
                });
                json!({"steps":steps,"lawgiver":lawgiver,"mood":if steps==max_steps {"Happy"} else {"Neutral"},"payment":payment,"reason":reason,"action":action})
            }).collect::<Vec<_>>();
        let leaders = p.civilization.leaders.iter().map(|info| {
            let l = &info.leader;
            let mut units = Units::empty();
            units += &UnitType::Leader(*l);
            let result = crate::recruit::recruit_cost_without_replaced(game, p, &units, city.position, CostTrigger::NoModifiers);
            let reason = leader_unavailable_reason(p, *l)
                .or_else(|| action_reason(game, seat, can_play, PlayingActionType::Recruit))
                .or_else(|| result.as_ref().err().cloned());
            let payment = result.map(|c| c.cost.first_valid_payment(&p.resources).unwrap_or_else(|| c.cost.default_payment()))
                .unwrap_or_else(|_| UnitType::Leader(*l).cost());
            json!({"id":l,"name":l.name(game),"payment":payment,"reason":reason,
                "abilities":game.cache.get_leader(l).abilities.iter().map(|a|json!({"name":a.name,"description":a.description})).collect::<Vec<_>>(),
                "description":game.cache.get_leader(l).abilities.iter().map(|a|format!("{}: {}",a.name,a.description)).collect::<Vec<_>>().join("\n")})
        }).collect::<Vec<_>>();
        json!({"position":city.position,"buildings":buildings,"recruits":recruits,"happiness":happiness,"leaders":leaders})
    }).collect()
}

pub fn recruit_preview(
    game: &Game,
    seat: usize,
    city: Position,
    units: Units,
) -> Result<Value, String> {
    if seat >= game.players.len() || seat != game.active_player() || !game.player(seat).is_human() {
        return Err("Wait for your turn".into());
    }
    PlayingActionType::Recruit.is_available(game, seat)?;
    if units.is_empty() {
        return Err("Choose at least one unit".into());
    }
    let p = game.player(seat);
    let city_data = p.try_get_city(city).ok_or("Choose one of your cities")?;
    let cost = recruit_cost(game, p, &units, city, &[], CostTrigger::NoModifiers)?;
    let payment = cost
        .cost
        .first_valid_payment(&p.resources)
        .ok_or("Not enough resources")?;
    Ok(
        json!({"payment":payment,"moodWillDecrease":city_data.is_activated(),
        "action":Action::Playing(PlayingAction::Recruit(Recruit::new(&units,city,payment.clone())))}),
    )
}

pub fn settlers(game: &Game, seat: usize, can_move: bool) -> Vec<Value> {
    let p = game.player(seat);
    p.units.iter().filter(|u|u.is_settler() && !u.is_transported()).map(|unit| {
        let mut destinations = if can_move {possible_move_routes(p,game,&[unit.id],unit.position,None).unwrap_or_default().into_iter()
            .filter(|route|(game.map.is_land(route.destination) || game.map.get(route.destination)==Some(&Terrain::Unexplored)) && game.enemy_player(seat,route.destination).is_none())
            .filter_map(|route|route.cost.first_valid_payment(&p.resources).map(|payment|json!({
                "position":route.destination,"terrain":game.map.get(route.destination),"payment":payment,
                "action":Action::Movement(MovementAction::Move(MoveUnits::new(vec![unit.id],route.destination,None,payment.clone())))}))).collect::<Vec<_>>() } else {vec![]};
        destinations.sort_by_key(|d|d["position"].as_str().unwrap_or_default().to_string());
        let founder = p.active_leader()==Some(crate::leader::Leader::QueenDido) && crate::leader::leader_position(p)==unit.position;
        let found_kind=if founder {crate::content::custom_actions::CustomActionType::Founder.playing_action_type()} else {PlayingActionType::FoundCity};
        let found_reason = action_reason(game,seat,game.state==GameState::Playing && seat==game.active_player(),found_kind)
            .or_else(||unit.found_city_blocker(game).map(Into::into));
        json!({"id":unit.id,"position":unit.position,"destinations":destinations,"foundReason":found_reason,
            "foundFree":founder,"foundAction":found_reason.is_none().then(||if founder {Action::Playing(PlayingAction::Custom(crate::content::custom_actions::CustomAction::new(crate::content::custom_actions::CustomActionType::Founder,Some(unit.position))))}else{Action::Playing(PlayingAction::FoundCity{settler:unit.id})})})
    }).collect()
}

pub(crate) fn nomad_movement(
    game: &Game,
    seat: usize,
    city: Position,
    units: Vec<u32>,
) -> Result<Value, String> {
    if seat != game.active_player() {
        return Err("Wait for your turn".into());
    }
    let destinations=crate::content::civilizations::huns::city_destinations(game,game.player(seat),city,&units).into_iter().map(|to|{
        let mut action=MoveUnits::new(units.clone(),to,None,crate::resource_pile::ResourcePile::empty());action.city=Some(city);
        json!({"position":to,"terrain":game.map.get(to),"payment":{},"carrier":null,"attack":false,"action":Action::Movement(MovementAction::Move(action))})
    }).collect::<Vec<_>>();
    Ok(json!({"destinations":destinations}))
}
