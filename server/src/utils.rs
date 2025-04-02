use itertools::Itertools;

#[must_use]
pub(crate) fn a_or_an(word: &str) -> String {
    if word.is_empty() {
        return "an".to_string();
    }
    let first_char = word.chars().next().expect("empty word");
    if first_char.is_ascii_alphabetic() && "aeiou".contains(first_char.to_ascii_lowercase()) {
        format!("an {word}")
    } else {
        format!("a {word}")
    }
}

#[must_use]
pub(crate) fn format_and<S: AsRef<str>>(list: &[S], empty_message: &str) -> String {
    format_list(list, empty_message, "and")
}

#[must_use]
pub(crate) fn format_list<S: AsRef<str>>(
    list: &[S],
    empty_message: &str,
    conjunction: &str,
) -> String {
    match list {
        [] => empty_message.to_string(),
        [element] => element.as_ref().to_string(),
        _ => format!(
            "{} {} {}",
            &list[..list.len() - 1].iter().map(AsRef::as_ref).join(", "),
            conjunction,
            list.last()
                .expect("collection should have at least 2 elements")
                .as_ref(),
        ),
    }
}

#[must_use]
pub fn is_false(value: &bool) -> bool {
    !value
}

pub fn remove_element<T>(list: &mut Vec<T>, element: &T) -> Option<T>
where
    T: PartialEq,
{
    let index = list
        .iter()
        .position(|other_element| other_element == element);
    if let Some(index) = index {
        return Some(list.remove(index));
    }
    None
}

pub fn remove_element_by<F, T>(list: &mut Vec<T>, f: F) -> Option<T>
where
    F: Fn(&T) -> bool,
{
    list.iter().position(f).map(|index| list.remove(index))
}

pub fn remove_and_map_element_by<F, T, U>(list: &mut Vec<T>, f: F) -> Option<U>
where
    F: Fn(&T) -> Option<U>,
{
    remove_element_by(list, |e| f(e).is_some()).and_then(|e| f(&e))
}

#[must_use]
pub fn ordinal_number(value: u32) -> String {
    format!(
        "{value}{}",
        match value % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    )
}

#[derive(Clone, Default)]
pub struct Rng {
    pub(crate) seed: u128,
}

impl Rng {
    #[must_use]
    pub(crate) fn from_seed(seed: u128) -> Self {
        Self { seed }
    }

    #[must_use]
    pub(crate) fn from_seed_string(seed: &str) -> Self {
        let seed = if seed.is_empty() {
            0
        } else {
            seed.parse().expect("seed should be a number")
        };
        Self { seed }
    }

    pub(crate) fn range(&mut self, start: usize, end: usize) -> usize {
        self.seed = next_seed(self.seed);
        let range = (end - start) as u128;
        let random_value = self.seed % range;
        start + random_value as usize
    }
}

fn next_seed(seed: u128) -> u128 {
    const XOR: u128 = 295_990_755_076_957_304_699_390_954_000_840_642_031;
    const ROTATE: u32 = 37;
    const MULTIPLIER: u128 = 6_364_136_223_846_793_005;
    const INCREMENT: u128 = 1;

    let new_seed = seed ^ XOR;
    new_seed
        .rotate_left(ROTATE)
        .wrapping_mul(MULTIPLIER)
        .wrapping_add(INCREMENT)
}

pub(crate) trait Shuffle {
    #[must_use]
    fn shuffled(self, rng: &mut Rng) -> Self;
    fn shuffle(&mut self, rng: &mut Rng);
}

impl<T> Shuffle for Vec<T> {
    fn shuffled(mut self, rng: &mut Rng) -> Self {
        self.shuffle(rng);
        self
    }

    fn shuffle(&mut self, rng: &mut Rng) {
        let mut new = Vec::new();
        let length = self.len();
        for _ in 0..length {
            let index = rng.range(0, self.len());
            new.push(self.remove(index));
        }
        *self = new;
    }
}

#[cfg(test)]
pub mod tests {
    use std::{
        collections::HashMap,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use itertools::Itertools;

    use crate::utils::Rng;

    pub trait FloatEq {
        fn eq(self, other: Self) -> bool;
        fn assert_eq(self, expected: Self);
    }

    impl FloatEq for f32 {
        fn eq(self, other: Self) -> bool {
            (self - other).abs() <= Self::EPSILON
        }

        fn assert_eq(self, expected: Self) {
            let same = self.eq(expected);
            assert!(same, "expected: {expected}, got: {self}",);
        }
    }

    fn get_current_time() -> Duration {
        let start = SystemTime::now();
        start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
    }

    #[test]
    fn test_rng() {
        const ITERATIONS: usize = 10_000;
        const MODULO: usize = 12;
        const TOLERANCE_PERCENTAGE: f32 = 15.;

        const EXPECTED_OCCURRENCES: usize = ITERATIONS / MODULO;
        const TOLERANCE: usize =
            (EXPECTED_OCCURRENCES as f32 * TOLERANCE_PERCENTAGE * 0.01) as usize;

        let initial_seed = get_current_time().as_nanos();
        let mut rng = Rng::from_seed(initial_seed);
        let mut results = HashMap::new();

        for _ in 0..ITERATIONS {
            let result = rng.range(0, MODULO);
            *results.entry(result).or_insert(0) += 1;
        }

        for count in results.values() {
            if (*count as isize - EXPECTED_OCCURRENCES as isize).unsigned_abs() < TOLERANCE {
                continue;
            }

            panic!(
                "random number generator does not create an even distribution of seeds with modulo 12 on initial seed {initial_seed}.\nHere is the actual distribution (expected count: {EXPECTED_OCCURRENCES}, acceptable range: {} - {}):\nvalue | count\n{}",
                EXPECTED_OCCURRENCES - TOLERANCE,
                EXPECTED_OCCURRENCES + TOLERANCE,
                results
                    .into_iter()
                    .sorted_by_key(|(value, _)| *value)
                    .map(|(value, count)| format!(
                        "{}{value} | {}{count}{}",
                        " ".repeat(5 - value.to_string().len()),
                        " ".repeat(5 - count.to_string().len()),
                        if (count as isize - EXPECTED_OCCURRENCES as isize).unsigned_abs()
                            > TOLERANCE
                        {
                            " <-- outside of range"
                        } else {
                            ""
                        }
                    ))
                    .collect::<Vec<String>>()
                    .join("\n")
            )
        }
    }
}
