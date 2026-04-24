use crate::filters::Filters;

pub fn parse(q: &str) -> Option<Filters> {
    let q = q.to_lowercase();
    let mut f = Filters::default();

    // gender
    if q.contains("male") && !q.contains("female") {
        f.gender = Some("male".into());
    } else if q.contains("female") {
        f.gender = Some("female".into());
    }

    // country
    if q.contains("nigeria") { f.country_id = Some("NG".into()); }
    if q.contains("kenya") { f.country_id = Some("KE".into()); }
    if q.contains("angola") { f.country_id = Some("AO".into()); }

    // age group
    if q.contains("teenager") { f.age_group = Some("teenager".into()); }
    if q.contains("adult") { f.age_group = Some("adult".into()); }

    // rules
    if q.contains("young") {
        f.min_age = Some(16);
        f.max_age = Some(24);
    }

    if q.contains("above 30") {
        f.min_age = Some(30);
    }

    Some(f)
}