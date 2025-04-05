use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{CivilCardMatch, CivilCardOpportunity};
use crate::content::advances::warfare::draft_cost;
use crate::objective_card::{Objective, objective_is_ready};

pub(crate) fn combat_objectives() -> Vec<Objective> {
    vec![conqueror()]
}

pub(crate) fn conqueror() -> Objective {
    let name = "Conqueror";
    Objective::builder(
        name,
        "You conquered a city with at least 1 Army unit or Fortress this turn.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_end,
        2,
        |game, player, _, e| {
            let c = &e.combat;
            if let Some(winner) = e.result.winner() {
                let p = c.player(winner);
                if p == game.current_player_index && !c.is_sea_battle(game) {
                    CivilCardMatch::new(
                        CivilCardOpportunity::WinLandBattle,
                        Some(c.opponent(p)),
                    )
                    .store(game);
                }
            }
        },
    )
    .build()
}
