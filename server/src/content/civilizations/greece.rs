use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::civilization::Civilization;
use crate::player::gain_resources;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};

pub(crate) fn greece() -> Civilization {
    Civilization::new("Greece", vec![study(), sparta()], vec![])
}

fn study() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::Study,
        SpecialAdvanceRequirement::Advance(Advance::PublicEducation),
        "Study",
        "Gain 1 idea when recruiting in a city with an Academy.",
    )
        .add_simple_persistent_event_listener(
            |event| &mut event.recruit,
            3,
            |game, player_index, _player_name, r| {
                if game.get_any_city(r.city_position).pieces.academy.is_some() {
                    gain_resources(
                        game,
                        player_index,
                        ResourcePile::ideas(1),
                        |name, pile| format!("{name} gained {pile} for Study"),
                    );
                }
            },
        )
    .build()
}

fn sparta() -> SpecialAdvanceInfo {
    // todo You may pay Draft with culture tokens instead of mood tokens. \
    // todo In land battles with fewer units than your enemy: Your ememy may nut play tactics cards.
    SpecialAdvanceInfo::builder(
        SpecialAdvance::Sparta,
        SpecialAdvanceRequirement::Advance(Advance::Draft),
        "Sparta",
        "You may pay Draft with culture tokens instead of mood tokens. \
        In land battles with fewer units than your enemy: Your enemy may nut play tactics cards.",
    )
    .build()
}
