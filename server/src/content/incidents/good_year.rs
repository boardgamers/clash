use crate::content::persistent_events::{PositionRequest, ResourceRewardRequest};
use crate::incident::{Incident, IncidentBaseEffect, IncidentBuilder};
use crate::player::gain_unit;
use crate::player_events::IncidentTarget;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::unit::UnitType;

enum GoodYearType {
    ActivePlayer,
    AllPlayers,
    Distribute,
}

pub(crate) fn good_years_incidents() -> Vec<Incident> {
    let mut r = good_years();
    r.extend(awesome_years());
    r.extend(fantastic_years());
    r.extend(population_booms());
    r.push(successful_year());
    r
}

fn good_years() -> Vec<Incident> {
    vec![
        good_year(
            Incident::builder(
                9,
                "A good year",
                "Every player gains 1 food",
                IncidentBaseEffect::BarbariansSpawn,
            ),
            1,
            &GoodYearType::AllPlayers,
        ),
        good_year(
            Incident::builder(
                10,
                "A good year",
                "You gain 1 food",
                IncidentBaseEffect::BarbariansSpawn,
            ),
            1,
            &GoodYearType::ActivePlayer,
        ),
        good_year(
            Incident::builder(
                11,
                "A good year",
                "You gain 1 food. Select another player to gain 1 food.",
                IncidentBaseEffect::BarbariansSpawn,
            ),
            1,
            &GoodYearType::Distribute,
        ),
    ]
}

fn awesome_years() -> Vec<Incident> {
    vec![
        good_year(
            Incident::builder(
                12,
                "An awesome year",
                "You gain 2 food",
                IncidentBaseEffect::BarbariansSpawn,
            ),
            2,
            &GoodYearType::ActivePlayer,
        ),
        good_year(
            Incident::builder(
                13,
                "An awesome year",
                "Every player gains 2 food",
                IncidentBaseEffect::ExhaustedLand,
            ),
            2,
            &GoodYearType::AllPlayers,
        ),
        good_year(
            Incident::builder(
                14,
                "An awesome year",
                "You gain 2 food. Distribute 2 food among other players.",
                IncidentBaseEffect::ExhaustedLand,
            ),
            2,
            &GoodYearType::Distribute,
        ),
    ]
}

fn fantastic_years() -> Vec<Incident> {
    vec![
        good_year(
            Incident::builder(
                15,
                "A fantastic year",
                "Every player gains 3 food",
                IncidentBaseEffect::BarbariansSpawn,
            ),
            3,
            &GoodYearType::AllPlayers,
        ),
        good_year(
            Incident::builder(
                16,
                "A fantastic year",
                "You gain 3 food. Distribute 3 food among other players.",
                IncidentBaseEffect::BarbariansSpawn,
            ),
            3,
            &GoodYearType::Distribute,
        ),
        good_year(
            Incident::builder(
                17,
                "A fantastic year",
                "You gain 3 food",
                IncidentBaseEffect::BarbariansSpawn,
            ),
            2,
            &GoodYearType::ActivePlayer,
        ),
    ]
}

fn good_year(mut builder: IncidentBuilder, amount: u8, good_year_type: &GoodYearType) -> Incident {
    let role = match good_year_type {
        GoodYearType::AllPlayers => IncidentTarget::AllPlayers,
        GoodYearType::Distribute | GoodYearType::ActivePlayer => IncidentTarget::ActivePlayer,
    };

    builder = builder.add_incident_resource_request(role, 10, move |_game, p, _incident| {
        Some(ResourceRewardRequest::new(
            p.reward_options().sum(amount, &[ResourceType::Food]),
            "-".to_string(),
        ))
    });

    if matches!(good_year_type, GoodYearType::Distribute) {
        for i in 0..amount {
            builder = builder.add_incident_player_request(
                IncidentTarget::ActivePlayer,
                "Select a player to gain 1 food",
                |p, _, _| p.resources.food < p.resource_limit.food,
                i as i32,
                move |game, s, _| {
                    s.other_player(s.choice, game)
                        .gain_resources(game, ResourcePile::food(1));
                },
            );
        }
    }

    builder.build()
}

fn population_booms() -> Vec<Incident> {
    vec![
        population_boom(27, IncidentBaseEffect::BarbariansSpawn),
        population_boom(28, IncidentBaseEffect::BarbariansMove),
    ]
}

fn population_boom(id: u8, effect: IncidentBaseEffect) -> Incident {
    let mut b = Incident::builder(
        id,
        "Population Boom",
        "Gain 1 settler in one of your cities. \
            Select another player to gain 1 settler on one of their cities.",
        effect,
    );
    b = select_settler(b, 13, IncidentTarget::ActivePlayer);
    select_player_to_gain_settler(b).build()
}

pub(crate) fn select_player_to_gain_settler(mut b: IncidentBuilder) -> IncidentBuilder {
    b = b.add_incident_player_request(
        IncidentTarget::ActivePlayer,
        "Select another player to gain 1 settler on one of their cities",
        |p, _, _| p.available_units().settlers > 0 && !p.cities.is_empty(),
        12,
        |game, c, i| {
            c.log(
                game,
                &format!(
                    "{} was selected to gain 1 settler.",
                    game.player_name(c.choice)
                ),
            );
            i.selected_player = Some(c.choice);
        },
    );
    select_settler(b, 11, IncidentTarget::SelectedPlayer)
}

fn select_settler(b: IncidentBuilder, priority: i32, target: IncidentTarget) -> IncidentBuilder {
    b.add_incident_position_request(
        target,
        priority,
        move |game, p, _| {
            let p = p.get(game);
            if p.available_units().settlers > 0 {
                let choices = p.cities.iter().map(|c| c.position).collect();
                let needed = 1..=1;
                Some(PositionRequest::new(
                    choices,
                    needed,
                    "Select a city to gain 1 settler",
                ))
            } else {
                None
            }
        },
        |game, s, _| {
            gain_unit(game, &s.player(), s.choice[0], UnitType::Settler);
        },
    )
}

pub(crate) fn successful_year() -> Incident {
    Incident::builder(
        54,
        "A successful year",
        "All players with the fewest cities gain 1 food for every city \
        they have less than the player with the most cities. \
        If everyone has the same number of cities, all players gain 1 food.",
        IncidentBaseEffect::PiratesSpawnAndRaid,
    )
    .add_incident_resource_request(IncidentTarget::AllPlayers, 0, |game, p, _incident| {
        let player_to_city_num = game
            .players
            .iter()
            .filter_map(|p| p.is_human().then_some(p.cities.len()))
            .collect::<Vec<_>>();

        let min_cities = player_to_city_num.iter().min().unwrap_or(&0);
        let max_cities = player_to_city_num.iter().max().unwrap_or(&0);
        if min_cities == max_cities {
            return Some(ResourceRewardRequest::new(
                p.reward_options().sum(1, &[ResourceType::Food]),
                "-".to_string(),
            ));
        }

        let cities = p.get(game).cities.len();
        if cities == *min_cities {
            Some(ResourceRewardRequest::new(
                p.reward_options()
                    .sum((max_cities - min_cities) as u8, &[ResourceType::Food]),
                "-".to_string(),
            ))
        } else {
            None
        }
    })
    .build()
}
