pub fn format_list(list: &Vec<String>, empty_message: &str) -> String {
    match &list[..] {
        [] => empty_message.to_string(),
        [element] => element.clone(),
        _ => format!(
            "{} and {}",
            &list[..list.len() - 1].join(", "),
            list.last()
                .expect("collection should have at least 2 elements"),
        ),
    }
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

#[cfg(test)]
pub mod tests {
    pub fn eq_f32(a: f32, b: f32) -> bool {
        (a - b).abs() <= f32::EPSILON
    }
}
