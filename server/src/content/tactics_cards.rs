use crate::tactics_card::{FighterRequirement, TacticsCard, TacticsCardTarget};
use itertools::Itertools;

#[must_use]
pub(crate) fn get_all() -> Vec<TacticsCard> {
    let all: Vec<TacticsCard> = vec![vec![peltasts()]].into_iter().flatten().collect_vec();
    assert_eq!(
        all.iter().unique_by(|i| &i.name).count(),
        all.len(),
        "action card ids are not unique"
    );
    all
}

///
/// # Panics
/// Panics if action card does not exist
#[must_use]
pub fn get_tactics_card(name: &str) -> TacticsCard {
    get_all()
        .into_iter()
        .find(|c| c.name == name)
        .expect("action card not found")
}

pub(crate) fn peltasts() -> TacticsCard {
    TacticsCard::builder(
        "Peltasts",
        "On reveal: Roll a die for each of your Army units. \
        If you rolled a 5 or 6, ignore 1 hit",
        TacticsCardTarget::ActivePlayer,
        FighterRequirement::Army,
    )
    .add_reveal_listener(0, |player, game, combat, s| {
        for _ in &combat.fighting_units(game, player) {
            let roll = game.get_next_dice_roll().value;
            if roll >= 5 {
                s.roll_log
                    .push(format!("Peltasts rolled a {roll} and ignored a hit",));
                s.hit_cancels += 1;
                return;
            }
        }
        s.roll_log.push("Pelts rolled no 5 or 6".to_string());
    })
    .build()
}
