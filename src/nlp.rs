use crate::filters::Filters;

pub fn parse(q: &str) -> Option<Filters> {
    let q = q.to_lowercase();
    let mut f = Filters::default();
    let mut found_something = false;

    // Gender
    if q.contains("female") || q.contains("woman") || q.contains("females") {
        f.gender = Some("female".into());
        found_something = true;
    } else if q.contains("male") || q.contains("man") || q.contains("males") {
        f.gender = Some("male".into());
        found_something = true;
    }

    // Country
    if q.contains("nigeria") { f.country_id = Some("NG".into()); found_something = true; }
    if q.contains("kenya") { f.country_id = Some("KE".into()); found_something = true; }
    if q.contains("rwanda") { f.country_id = Some("RW".into()); found_something = true; }

    // Age Groups
    if q.contains("young") { f.min_age = Some(18); f.max_age = Some(30); found_something = true; }
    if q.contains("teenager") || q.contains("teen") { f.age_group = Some("teenager".into()); found_something = true; }
    if q.contains("adult") { f.age_group = Some("adult".into()); found_something = true; }
    if q.contains("senior") { f.age_group = Some("senior".into()); found_something = true; }

    // Above/Over X
    if let Some(idx) = q.find("above").or(q.find("over")).or(q.find("older than")) {
        let rest = &q[idx..];
        if let Some(num) = rest.chars().filter(|c| c.is_digit(10)).collect::<String>().parse::<i32>().ok() {
            f.min_age = Some(num + 1);
            found_something = true;
        }
    }

    if found_something { Some(f) } else { None }
}