use crate::{ability_initializer::AbilityInitializerSetup, advance::Advance};

pub fn get_technologies() -> Vec<Advance> {
    vec![Advance::builder(
        "Engineering",
        "● Immediately draw 1 wonder\n● May Construct wonder happy cities",
    )
    .add_ability_initializer(Box::new(|game, player| game.draw_wonder_card(player)))
    .add_custom_action("Construct wonder")
    .build()]
}

pub fn get_advance_by_name(name: &str) -> Option<Advance> {
    get_technologies()
        .into_iter()
        .find(|technology| technology.name == name)
}
