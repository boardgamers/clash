use crate::city::MoodState;
use crate::content::incidents_famine::decrease_mood_incident_city;
use crate::content::incidents_population_boom::select_player_to_gain_settler;
use crate::incident::{Incident, IncidentBaseEffect};
use crate::player::Player;
use crate::player_events::IncidentTarget;
use crate::position::Position;
use itertools::Itertools;

pub(crate) fn migrations() -> Vec<Incident> {
    vec![migration(34), migration(35)]
}

fn migration(id: u8) -> Incident {
    let mut b = Incident::builder(
        id,
        "Migration",
        "Select a player to gain 1 settler in one of their cities. Decrease the mood in one of your cities.",
        IncidentBaseEffect::GoldDeposits,
    );
    b = select_player_to_gain_settler(b);
    b = b.add_myths_payment(IncidentTarget::ActivePlayer, |_game, p| {
        u32::from(!non_angry_cites(p).is_empty())
    });
    decrease_mood_incident_city(b, IncidentTarget::ActivePlayer, 0, |game, player_index| {
        non_angry_cites(game.get_player(player_index))
    })
    .build()
}

fn non_angry_cites(p: &Player) -> Vec<Position> {
    p.cities
        .iter()
        .filter(|c| !matches!(c.mood_state, MoodState::Angry))
        .map(|c| c.position)
        .collect_vec()
}
