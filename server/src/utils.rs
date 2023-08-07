pub fn format_collection(collection: &Vec<String>, empty_format: &str) -> String {
    match &collection[..] {
        [] => empty_format.to_string(),
        [element] => element.clone(),
        _ => format!(
            "{} and {}",
            &collection[..collection.len() - 1].join(", "),
            collection
                .last()
                .expect("collection should have at least 2 elements"),
        ),
    }
}

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
