pub fn join_with_final<T>(separator: &str, final_separator: &str, strings: Vec<T>) -> String
where
    T: std::string::ToString,
{
    if strings.is_empty() {
        return "".to_string();
    }
    if strings.len() == 1 {
        return strings[0].to_string();
    }
    let mut result = strings[0].to_string();
    for s in strings[1..strings.len() - 1].iter() {
        result.push_str(separator);
        result.push_str(&s.to_string());
    }
    result.push_str(final_separator);
    result.push_str(&strings[strings.len() - 1].to_string());
    result
}
