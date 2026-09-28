use crate::action::Action;
use crate::city::MoodState;
use crate::city_pieces::BUILDINGS;
use crate::construct::{Construct, can_construct, new_building_positions};
use crate::game::{Game, GameState};
use crate::happiness::{IncreaseHappiness, happiness_base_event_origin, happiness_cost};
use crate::map::Terrain;
use crate::movement::{MoveUnits, MovementAction, possible_move_routes};
use crate::player::CostTrigger;
use crate::playing_actions::{PlayingAction, PlayingActionType};
use crate::position::Position;
use crate::recruit::{Recruit, recruit_cost};
use crate::unit::{UnitType, Units};
use serde_json::{Value, json};

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
    if units.is_empty() {
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
    let cost = recruit_cost(
        game,
        p,
        &units,
        position,
        &replaced,
        CostTrigger::NoModifiers,
    )?;
    let payment = cost
        .cost
        .first_valid_payment(&p.resources)
        .ok_or("Not enough resources")?;
    Ok(
        json!({"payment":payment,"moodWillDecrease":city.is_activated(),"action":Action::Playing(PlayingAction::Recruit(Recruit::new(&units,position,payment.clone()).with_replaced_units(&replaced)))}),
    )
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
    let mut steps = 0;
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
        steps += city.size() as u8 * n;
    }
    let origin = crate::happiness::happiness_event_origin(&kind, p);
    let cost = happiness_cost(seat, steps, CostTrigger::NoModifiers, &kind, game, &origin);
    let payment = cost
        .cost
        .first_valid_payment(&p.resources)
        .ok_or("Not enough resources")?;
    Ok(
        json!({"payment":payment,"action":Action::Playing(PlayingAction::IncreaseHappiness(IncreaseHappiness::new(selections,payment.clone(),kind)))}),
    )
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
                let offer = json!({"position":route.destination,"terrain":game.map.get(route.destination),"payment":payment,"carrier":carrier,"attack":game.enemy_player(seat,route.destination).is_some(),
                    "action":Action::Movement(MovementAction::Move(MoveUnits::new(units.clone(),route.destination,carrier,payment)))});
                if !destinations.contains(&offer) {
                    destinations.push(offer);
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
        json!({"id":id,"name":c.civil_card.name,"description":c.civil_card.description,"free":c.civil_card.action_type.free,
            "tactics":c.tactics_card.as_ref().map(|t|json!({"name":t.name,"description":t.description})),
            "reason":reason,"action":reason.is_none().then(||Action::Playing(PlayingAction::ActionCard(*id)))})
    }).collect()
}

pub fn special(game: &Game, seat: usize, can_play: bool) -> Vec<Value> {
    use crate::content::custom_actions::{CustomAction, CustomActionExecution};
    if !can_play {
        return vec![];
    }
    game.available_custom_actions(seat).iter().flat_map(|info| {
        let CustomActionExecution::Action(execution) = &info.execution else { return vec![]; };
        let cities = if info.city_bound().is_some() {
            game.player(seat).cities.iter().filter(|c|info.is_city_available(game,c)).map(|c|Some(c.position)).collect()
        } else { vec![None] };
        cities.into_iter().map(|city|json!({"name":info.event_origin.name(game),"description":execution.ability.description,"position":city,
            "action":Action::Playing(PlayingAction::Custom(CustomAction::new(info.action,city)))})).collect::<Vec<_>>()
    }).collect()
}

pub fn influence(game: &Game, seat: usize, can_play: bool) -> Vec<Value> {
    use crate::cultural_influence::{
        InfluenceCultureAttempt, available_influence_actions, available_influence_culture,
    };
    if !can_play {
        return vec![];
    }
    available_influence_actions(game,seat).into_iter().flat_map(|kind| {
        available_influence_culture(game,seat,&kind).into_iter().filter_map(|(s,r)| {
            let info = r.ok()?;
            Some(json!({"name":super::decisions::structure_name(&s.structure),"position":s.position,"origin":info.starting_city_position,"variant":kind.origin(game.player(seat)).name(game),"payment":info.range_boost_cost.default_payment(),
                "action":Action::Playing(PlayingAction::InfluenceCultureAttempt(InfluenceCultureAttempt::new(s,kind.clone())))}))
        }).collect::<Vec<_>>()
    }).collect()
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
    p.cities.iter().map(|city| {
        let buildings = BUILDINGS.into_iter().map(|building| {
            let cost = p.building_cost(game, building, CostTrigger::NoModifiers);
            let payment = cost.cost.first_valid_payment(&p.resources).unwrap_or_else(|| cost.cost.default_payment());
            let positions = new_building_positions(game, building, city);
            let reason = action_reason(game, seat, can_play, PlayingActionType::Construct)
                .or_else(|| can_construct(city, building, p, game, CostTrigger::NoModifiers, &[]).err())
                .or_else(|| positions.is_empty().then(|| "Needs an adjacent sea tile".to_string()));
            let choices = if reason.is_none() {positions.into_iter().map(|port| json!({"position":port,
                "action":Action::Playing(PlayingAction::Construct(Construct::new(city.position, building, payment.clone()).with_port_position(port)))})).collect::<Vec<_>>()} else {vec![]};
            json!({"name":building.to_string(),"owned":!city.pieces.can_add_building(building),
                "required":game.cache.get_building_advance(building).name(game),"payment":payment,"reason":reason,"choices":choices,
                "moodWillDecrease":cost.activate_city && city.is_activated()})
        }).collect::<Vec<_>>();
        let recruits = [UnitType::Settler,UnitType::Infantry,UnitType::Cavalry,UnitType::Elephant,UnitType::Ship].into_iter().map(|unit| {
            let mut units = Units::empty(); units += &unit;
            let result = crate::recruit::recruit_cost_without_replaced(game,p,&units,city.position,CostTrigger::NoModifiers);
            let reason = action_reason(game,seat,can_play,PlayingActionType::Recruit).or_else(||result.as_ref().err().cloned());
            let payment = result.map(|c|c.cost.first_valid_payment(&p.resources).unwrap_or_else(||c.cost.default_payment())).unwrap_or_else(|_|unit.cost());
            json!({"type":unit,"payment":payment,"reason":reason,"available":p.available_units().get_amount(&unit),"limit":p.unit_limit().get_amount(&unit)})
        }).collect::<Vec<_>>();
        let max_steps = match city.mood_state {MoodState::Happy=>0, MoodState::Neutral=>1, MoodState::Angry=>2};
        let happiness = (1..=max_steps).map(|steps| {
            let kind = PlayingActionType::IncreaseHappiness;
            let cost = happiness_cost(seat,city.size() as u8 * steps,CostTrigger::NoModifiers,&kind,game,&happiness_base_event_origin());
            let payment = cost.cost.first_valid_payment(&p.resources);
            let reason = action_reason(game,seat,can_play,kind.clone()).or_else(||payment.is_none().then(||"Not enough resources".into()));
            let payment = payment.unwrap_or_else(||cost.cost.default_payment());
            let action = reason.is_none().then(||Action::Playing(PlayingAction::IncreaseHappiness(IncreaseHappiness::new(vec![(city.position,steps)],payment.clone(),kind))));
            json!({"steps":steps,"mood":if steps==max_steps {"Happy"} else {"Neutral"},"payment":payment,"reason":reason,"action":action})
        }).collect::<Vec<_>>();
        let leaders = p.available_leaders.iter().map(|l|json!({"id":l,"name":l.name(game),"description":game.cache.get_leader(l).abilities.iter().map(|a|format!("{}: {}",a.name,a.description)).collect::<Vec<_>>().join("\n")})).collect::<Vec<_>>();
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
        let found_reason = action_reason(game,seat,game.state==GameState::Playing && seat==game.active_player(),PlayingActionType::FoundCity)
            .or_else(||(!unit.can_found_city(game)).then(||"Move to an empty land tile to found a city".into()));
        json!({"id":unit.id,"position":unit.position,"destinations":destinations,"foundReason":found_reason,
            "foundAction":found_reason.is_none().then(||Action::Playing(PlayingAction::FoundCity{settler:unit.id}))})
    }).collect()
}
