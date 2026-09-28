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
            let result = recruit_cost(game,p,&units,city.position,&[],CostTrigger::NoModifiers);
            let reason = action_reason(game,seat,can_play,PlayingActionType::Recruit).or_else(||result.as_ref().err().cloned());
            let payment = result.map(|c|c.cost.first_valid_payment(&p.resources).unwrap_or_else(||c.cost.default_payment())).unwrap_or_else(|_|unit.cost());
            json!({"type":unit,"payment":payment,"reason":reason,"available":p.available_units().get_amount(&unit)})
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
        json!({"position":city.position,"buildings":buildings,"recruits":recruits,"happiness":happiness})
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
