use crate::ability_initializer::AbilityInitializerSetup;
use crate::collect::{CollectInfo, PositionCollection};
use crate::content::builtin::Builtin;
use crate::game::Game;
use crate::player::Player;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use itertools::Itertools;
use std::collections::HashSet;
use std::mem;


pub(crate) fn possible_collections(info: &CollectInfo) -> Vec<Vec<PositionCollection>> {
    let choices = info.choices.clone();

    let mut list = choices.iter().collect_vec();
    let tiles = list.len(); // production focus is not considered
    let mut max_tiles = tiles;

    if info.max_range2_tiles > 0 {
        // avoid husbandry
        list.sort_by_key(|(pos, _)| (pos.distance(info.city), (*pos).clone()));

        let range2_tiles = list
            .iter()
            .filter(|(pos, _)| pos.distance(info.city) > 1)
            .count();
        max_tiles -= range2_tiles.saturating_sub(info.max_range2_tiles as usize);
    }
    let take = (info.max_selection as usize).min(max_tiles);

    let combinations = generate_all_combinations(tiles, take);

    combinations
        .into_iter()
        .flat_map(|c| {
            let mut result = vec![vec![]];
            add_combination(&c, &list, &mut result);
            result
        })
        .filter(|c| {
            info.max_range2_tiles == 0
                || c.iter()
                    .filter(|p| p.position.distance(info.city) > 1)
                    .count()
                    <= info.max_range2_tiles as usize
        })
        .unique_by(total_collect)
        .collect_vec()
}

fn add_combination(
    combo: &[usize],
    choices: &[(&Position, &HashSet<ResourcePile>)],
    result: &mut Vec<Vec<PositionCollection>>,
) {
    if combo.is_empty() {
        return;
    }

    let (pos, pile) = choices[combo[0]];

    if pile.len() > 1 {
        let old = mem::take(result);
        for j in pile {
            for r in &old.clone() {
                let mut r = r.clone();
                r.push(PositionCollection::new(*pos, j.clone()));
                result.push(r);
            }
        }
    } else {
        let n = pile.iter().next().expect("pile empty");
        for r in &mut *result {
            r.push(PositionCollection::new(*pos, n.clone()));
        }
    }
    add_combination(&combo[1..], choices, result);
}

/// Generates all possible combinations of `k` numbers out of `0...n-1`.
///
/// # Arguments
///
/// * `n` - The upper limit of the range (`0` to `n-1`).
/// * `k` - The number of elements in each combination.
///
/// # Returns
///
/// A `Result` containing a vector with all possible combinations of `k` numbers out of `0...n-1`,
/// or a `CombinationError` if the input is invalid.
pub fn generate_all_combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    if n == 0 && k > 0 {
        panic!("Invalid input: n is 0 and k is greater than 0");
    }

    if k > n {
        panic!("Invalid input: k is greater than n");
    }

    let mut combinations = vec![];
    let mut current = vec![0; k];
    backtrack(0, n, k, 0, &mut current, &mut combinations);
    combinations
}

/// Helper function to generate combinations recursively.
///
/// # Arguments
///
/// * `start` - The current number to start the combination with.
/// * `n` - The upper limit of the range (`0` to `n-1`).
/// * `k` - The number of elements left to complete the combination.
/// * `index` - The current index being filled in the combination.
/// * `current` - A mutable reference to the current combination being constructed.
/// * `combinations` - A mutable reference to the vector holding all combinations.
fn backtrack(
    start: usize,
    n: usize,
    k: usize,
    index: usize,
    current: &mut Vec<usize>,
    combinations: &mut Vec<Vec<usize>>,
) {
    if index == k {
        combinations.push(current.clone());
        return;
    }

    for num in start..=(n - k + index) {
        current[index] = num;
        backtrack(num + 1, n, k, index + 1, current, combinations);
    }
}

pub(crate) fn total_collect(r: &Vec<PositionCollection>) -> ResourcePile {
    let mut total = ResourcePile::empty();
    for c in r {
        total += c.total();
    }
    total
}

pub(crate) fn invalidate_collect_cache() -> Builtin {
    Builtin::builder("InvalidateCollectCache", "-")
        .add_simple_persistent_event_listener(
            |event| &mut event.found_city,
            1,
            |game, _player, _, p| {
                reset_collect_within_range_for_all(game, *p, 1);
            },
        )
        .add_simple_persistent_event_listener(
            |event| &mut event.combat_end,
            0,
            |game, player, _, p| {
                let range = if game.player(player).civilization.is_pirates() {
                    2 // pirates zone of control can influence a city 2 tiles away
                } else {
                    1
                };
                reset_collect_within_range_for_all(game, p.combat.defender_position, range);
            },
        )
        .build()
}

pub(crate) fn reset_collect_within_range(
    player: usize,
    position: Position,
    game: &mut Game,
    range: u8,
) {
    let p = game.player_mut(player);

    // husbandry is not used yet by the AI
    //     if is_land && p.has_advance(Advance::Husbandry) {
    //     2
    // } else {
    //     1
    // };
    for c in &mut p.cities {
        if c.position.distance(position) <= range as u32 {
            c.possible_collections.clear();
        }
    }
}

pub(crate) fn reset_collect_within_range_for_all(game: &mut Game, pos: Position, range: u8) {
    for p in 0..game.human_players_count() {
        reset_collect_within_range(p, pos, game, range);
    }
}

pub(crate) fn reset_collect_within_range_for_all_except(
    game: &mut Game,
    pos: Position,
    player: usize,
) {
    for p in 0..game.human_players_count() {
        if p == player {
            continue;
        }
        reset_collect_within_range(p, pos, game, 1);
    }
}

pub(crate) fn reset_collection_stats(p: &mut Player) {
    for c in &mut p.cities {
        c.possible_collections.clear();
    }
}

#[cfg(test)]
mod tests {
    use crate::ai_collect::{possible_collections, total_collect};
    use crate::collect::{CollectInfo, PositionCollection};
    use crate::player_events::ActionInfo;
    use crate::position::Position;
    use crate::resource_pile::ResourcePile;
    use itertools::Itertools;
    use rustc_hash::FxBuildHasher;
    use std::collections::{HashMap, HashSet};

    fn info(max: u8, choices: HashMap<Position, HashSet<ResourcePile>>) -> CollectInfo {
        CollectInfo {
            total: ResourcePile::empty(),
            city: Position::from_offset("D3"),
            modifiers: Vec::new(),
            choices,
            max_selection: max,
            max_per_tile: 0,
            max_range2_tiles: 0,
            info: ActionInfo {
                player: 0,
                info: HashMap::new(),
                log: Vec::new(),
            },
        }
    }

    fn assert_total(result: &Vec<Vec<PositionCollection>>, expected: Vec<ResourcePile>) {
        let got = result.iter().map(|r| total_collect(r)).collect::<Vec<_>>();
        let want: HashSet<&ResourcePile, FxBuildHasher> = HashSet::from_iter(expected.iter());
        let got: HashSet<&ResourcePile, FxBuildHasher> = HashSet::from_iter(got.iter());
        assert_eq!(got, want, "Total mismatch");
    }

    #[test]
    fn test_possible_collections() {
        let mut info = info(
            1,
            HashMap::from([
                (
                    Position::from_offset("D2"),
                    HashSet::from([ResourcePile::food(1)]),
                ),
                (
                    Position::from_offset("E2"),
                    HashSet::from([ResourcePile::wood(1)]),
                ),
                (
                    Position::from_offset("E3"),
                    HashSet::from([ResourcePile::ore(1)]),
                ),
                (
                    Position::from_offset("D4"),
                    HashSet::from([ResourcePile::ore(1)]),
                ),
            ]),
        );
        assert_total(&possible_collections(&info), vec![
            ResourcePile::ore(1),
            ResourcePile::food(1),
            ResourcePile::wood(1),
        ]);

        info.max_selection = 2;
        assert_total(&possible_collections(&info), vec![
            ResourcePile::ore(2),
            ResourcePile::food(1) + ResourcePile::wood(1),
            ResourcePile::food(1) + ResourcePile::ore(1),
            ResourcePile::wood(1) + ResourcePile::ore(1),
        ]);
    }

    // todo test port

    #[test]
    fn test_husbandry() {
        let mut info = info(
            1,
            HashMap::from([
                (
                    Position::from_offset("D4"), // prefer this
                    HashSet::from([ResourcePile::food(1)]),
                ),
                (
                    Position::from_offset("D1"), // over the further away food
                    HashSet::from([ResourcePile::food(1)]),
                ),
                (
                    Position::from_offset("C5"),
                    HashSet::from([ResourcePile::gold(1)]),
                ),
                (
                    Position::from_offset("D5"),
                    HashSet::from([ResourcePile::gold(1)]),
                ),
            ]),
        );
        info.max_range2_tiles = 1;
        let collections = possible_collections(&info);

        assert_total(&collections, vec![
            ResourcePile::gold(1),
            ResourcePile::food(1),
        ]);

        let positions = collections
            .iter()
            .map(|c| c.iter().map(|p| p.position).collect_vec())
            .collect_vec();

        assert_eq!(positions, vec![
            vec![Position::from_offset("D4")], // don't take the further away food
            vec![Position::from_offset("C5")],
        ]);

        info.max_range2_tiles = 2;
        info.max_selection = 3;
        assert_total(&possible_collections(&info), vec![
            ResourcePile::gold(2) + ResourcePile::food(1),
            ResourcePile::gold(1) + ResourcePile::food(2),
        ]);
    }

    #[test]
    fn test_husbandry_not_enough_tiles() {
        let mut info = info(
            2,
            HashMap::from([
                (
                    Position::from_offset("C5"),
                    HashSet::from([ResourcePile::gold(1)]),
                ),
                (
                    Position::from_offset("D5"),
                    HashSet::from([ResourcePile::gold(1)]),
                ),
            ]),
        );
        info.max_range2_tiles = 1;

        assert_total(&possible_collections(&info), vec![ResourcePile::gold(1)]);
    }

    #[test]
    fn test_port() {
        let info = info(
            2,
            HashMap::from([
                (
                    Position::from_offset("D3"),
                    HashSet::from([ResourcePile::gold(1), ResourcePile::mood_tokens(1)]),
                ),
                (
                    Position::from_offset("D4"),
                    HashSet::from([ResourcePile::gold(1)]),
                ),
            ]),
        );
        assert_total(&possible_collections(&info), vec![
            ResourcePile::gold(2),
            ResourcePile::gold(1) + ResourcePile::mood_tokens(1),
        ]);
    }
}
