use crate::content::custom_phase_actions::ResourceRewardRequest;
use crate::incident::{Incident, IncidentBaseEffect, IncidentBuilder};
use crate::payment::PaymentOptions;
use crate::player_events::IncidentTarget;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;

enum GoodYearType {
    ActivePlayer,
    AllPlayers,
    Distribute,
}

pub(crate) fn good_years() -> Vec<Incident> {
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

pub(crate) fn awesome_years() -> Vec<Incident> {
    vec![
        good_year(
            Incident::builder(
                12,
                "An awsome year",
                "You gain 2 food",
                IncidentBaseEffect::BarbariansSpawn,
            ),
            2,
            &GoodYearType::ActivePlayer,
        ),
        good_year(
            Incident::builder(
                13,
                "An awsome year",
                "Every player gains 2 food",
                IncidentBaseEffect::ExhaustedLand,
            ),
            2,
            &GoodYearType::AllPlayers,
        ),
        good_year(
            Incident::builder(
                14,
                "An awsome year",
                "You gain 2 food. Distribute 2 food among other players.",
                IncidentBaseEffect::ExhaustedLand,
            ),
            2,
            &GoodYearType::Distribute,
        ),
    ]
}

pub(crate) fn fantastic_years() -> Vec<Incident> {
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

fn good_year(mut builder: IncidentBuilder, amount: u32, good_year_type: &GoodYearType) -> Incident {
    let n = builder.name.clone();
    let role = match good_year_type {
        GoodYearType::AllPlayers => IncidentTarget::AllPlayers,
        GoodYearType::Distribute | GoodYearType::ActivePlayer => IncidentTarget::ActivePlayer,
    };

    builder = builder.add_incident_resource_request(
        role,
        10,
        move |_game, _player_index, _incident| {
            Some(ResourceRewardRequest::new(
                PaymentOptions::sum(amount, &[ResourceType::Food]),
                "-".to_string(),
            ))
        },
        move |_game, s| vec![format!("{} gained {} from {}", s.player_name, s.choice, n,)],
    );

    if matches!(good_year_type, GoodYearType::Distribute) {
        for i in 0..amount {
            let n = builder.name.clone();

            builder = builder.add_incident_player_request(
                "Select a player to gain 1 food",
                |p, _| p.resources.food < p.resource_limit.food,
                i as i32,
                move |game, c| {
                    game.add_info_log_item(&format!(
                        "{} gained 1 food from {}",
                        game.player_name(c.choice),
                        n.clone(),
                    ));
                    game.get_player_mut(c.choice)
                        .gain_resources(ResourcePile::food(1));
                },
            );
        }
    }

    builder.build()
}
