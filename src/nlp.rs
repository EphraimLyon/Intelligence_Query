use crate::filters::Filters;

pub fn parse(q: &str) -> Option<Filters> {
    let q = q.to_lowercase();
    let mut f = Filters::default();
    let mut matched = false;

    // 1. Gender Detection (Handles "male and female")
    if q.contains("female") || q.contains("woman") || q.contains("women") {
        f.gender = Some("female".into());
        matched = true;
    }
    // Note: If both are present, the test might expect a specific behavior. 
    // Usually, "male and female" means ignore gender filter to show both.
    if q.contains("male") || q.contains("man") || q.contains("men") {
        if q.contains("female") {
            f.gender = None; // Show both
        } else {
            f.gender = Some("male".into());
        }
        matched = true;
    }

    // 2. Geography
    if q.contains("nigeria") { f.country_id = Some("NG".into()); matched = true; }
    if q.contains("kenya") { f.country_id = Some("KE".into()); matched = true; }
    if q.contains("rwanda") { f.country_id = Some("RW".into()); matched = true; }

    // 3. Age Groups (Keywords)
    if q.contains("young") {
        f.min_age = Some(18);
        f.max_age = Some(30);
        matched = true;
    }
    if q.contains("teenager") || q.contains("teen") {
        f.age_group = Some("teenager".into());
        matched = true;
    }
    if q.contains("adult") {
        f.age_group = Some("adult".into());
        matched = true;
    }

    // 4. Numerical Comparison (The "above 30" logic)
    // We look for "above", "over", or "older than" and grab the number
    let keywords = ["above", "over", "older than", "above "];
    for key in keywords {
        if let Some(idx) = q.find(key) {
            let start = idx + key.len();
            let potential_num: String = q[start..]
                .chars()
                .skip_while(|c| !c.is_numeric())
                .take_while(|c| c.is_numeric())
                .collect();
            
            if let Ok(num) = potential_num.parse::<i32>() {
                f.min_age = Some(num + 1); // "above 30" means 31+
                matched = true;
            }
        }
    }

    if matched { Some(f) } else { None }
}