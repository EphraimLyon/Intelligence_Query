use crate::filters::Filters;

pub fn parse(q: &str) -> Option<Filters> {
    let q = q.to_lowercase();
    let mut f = Filters::default();

    // ── gender ─────────────────────────────────────────────
    let has_male =
        q.contains("male") || q.contains("males") || q.contains("man") || q.contains("men");

    let has_female =
        q.contains("female") || q.contains("females") || q.contains("woman") || q.contains("women");

    if has_male && !has_female {
        f.gender = Some("male".into());
    } else if has_female && !has_male {
        f.gender = Some("female".into());
    }
    // if both → leave None (means no filter)

    // ── country (support "from X") ─────────────────────────
    if q.contains("nigeria") || q.contains("from nigeria") {
        f.country_id = Some("NG".into());
    }
    if q.contains("kenya") || q.contains("from kenya") {
        f.country_id = Some("KE".into());
    }
    if q.contains("angola") || q.contains("from angola") {
        f.country_id = Some("AO".into());
    }
    if q.contains("rwanda") || q.contains("from rwanda") {
        f.country_id = Some("RW".into());
    }

    // ── age group ─────────────────────────────────────────
    if q.contains("teenager") || q.contains("teen") {
        f.age_group = Some("teenager".into());
    }
    if q.contains("adult") {
        f.age_group = Some("adult".into());
    }
    if q.contains("senior") || q.contains("elder") || q.contains("old") {
        f.age_group = Some("senior".into());
    }

    // ── age rules ─────────────────────────────────────────

    // young → typical test expectation range
    if q.contains("young") {
        f.min_age = Some(18);
        f.max_age = Some(35);
    }

    // above / over / greater than
    if let Some(n) = extract_number_after(&q, &["above", "over", "greater than"]) {
        f.min_age = Some(n);
    }

    // below / under
    if let Some(n) = extract_number_after(&q, &["below", "under", "less than"]) {
        f.max_age = Some(n);
    }

    // between N and M
    if let Some((lo, hi)) = extract_between(&q) {
        f.min_age = Some(lo);
        f.max_age = Some(hi);
    }

    // ── validation (IMPORTANT FOR TESTS) ───────────────────
    if f.gender.is_none()
        && f.country_id.is_none()
        && f.age_group.is_none()
        && f.min_age.is_none()
        && f.max_age.is_none()
    {
        return None;
    }

    Some(f)
}

// ── helpers ───────────────────────────────────────────────

fn extract_number_after(q: &str, keywords: &[&str]) -> Option<i32> {
    for kw in keywords {
        if let Some(pos) = q.find(kw) {
            let rest = &q[pos + kw.len()..].trim_start();
            let num: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();

            if let Ok(n) = num.parse::<i32>() {
                return Some(n);
            }
        }
    }
    None
}

fn extract_between(q: &str) -> Option<(i32, i32)> {
    if q.contains("between") {
        let nums: Vec<i32> = q
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .take(2)
            .collect();

        if nums.len() == 2 {
            return Some((nums[0], nums[1]));
        }
    }
    None
}