# Natural Language Query Parser

## Overview

The parser takes a plain English query string and converts it into a structured `Filters` object used to query the profiles API. It performs simple keyword matching — no ML, no NLP libraries, just `contains()` checks on a lowercased input string.

---

## 1. Parsing Approach

### How It Works

1. The input string is **lowercased** so all matching is case-insensitive.
2. A blank `Filters` struct is created using `Filters::default()`.
3. The lowercased string is scanned for known keywords using `.contains()`.
4. Matching keywords set the corresponding filter fields.
5. The populated `Filters` object is returned wrapped in `Some(...)`.

> Note: The function always returns `Some(f)` — even if no keywords matched and all fields remain `None`.

---

## 2. Supported Keywords & Filter Mappings

### Gender

| Keyword | Maps To |
|---------|---------|
| `male` (without `female`) | `gender = "male"` |
| `female` | `gender = "female"` |

**Logic:** The parser checks for `male` first, but only sets it if `female` is not also present in the string. This prevents `"female"` from accidentally matching the substring `"male"` inside it.

**Example queries:**
```
"show me male profiles"       → gender = male
"list all female users"       → gender = female
"male and female"             → gender = female  ⚠️ (see Limitations)
```

---

### Country

| Keyword | Maps To |
|---------|---------|
| `nigeria` | `country_id = "NG"` |
| `kenya` | `country_id = "KE"` |
| `angola` | `country_id = "AO"` |

**Logic:** Independent `if` blocks — not `else if`. If multiple country names appear in the query, the **last matched one wins** (Angola would override Nigeria if both appear).

**Example queries:**
```
"profiles from nigeria"       → country_id = NG
"users in kenya"              → country_id = KE
"angola profiles"             → country_id = AO
```

---

### Age Group

| Keyword | Maps To |
|---------|---------|
| `teenager` | `age_group = "teenager"` |
| `adult` | `age_group = "adult"` |

**Logic:** Independent `if` blocks. If both keywords appear, `adult` wins as it is checked last.

**Example queries:**
```
"find a teenager"             → age_group = teenager
"adult users only"            → age_group = adult
```

---

### Age Range Rules

| Keyword | Maps To |
|---------|---------|
| `young` | `min_age = 16`, `max_age = 24` |
| `above 30` | `min_age = 30` |

**Logic:** The `young` rule sets both `min_age` and `max_age` as a range. The `above 30` rule sets only `min_age`, with no upper bound.

**Example queries:**
```
"show young users"            → min_age = 16, max_age = 24
"users above 30"              → min_age = 30
"young adults above 30"       → min_age = 30, max_age = 24  ⚠️ (see Limitations)
```

---

## 3. Limitations & Edge Cases

### Always Returns `Some`

The function always returns `Some(f)` even when no keywords matched and every field is `None`. Callers cannot distinguish between "no filters matched" and "a valid empty filter was requested."

---

### No Number Extraction

Only the hardcoded phrase `"above 30"` is supported. The parser cannot understand:
```
"above 40"     → not handled
"older than 25"→ not handled
"under 18"     → not handled
"between 20 and 35" → not handled
```
Any age phrasing other than the exact string `"above 30"` is silently ignored.

---

### Last-Write Wins for Country

Country filters use independent `if` blocks, not `else if`. A query like `"nigeria and kenya"` would set `country_id = KE` because Kenya is checked last, silently discarding Nigeria.

---

### Conflicting Age Rules

If both `young` and `above 30` appear in a query, both blocks run and partially overwrite each other:
```
"young users above 30"
→ young sets:     min_age = 16, max_age = 24
→ above 30 sets:  min_age = 30  (overwrites 16)
→ result:         min_age = 30, max_age = 24  ← impossible range
```
This produces a filter where `min_age > max_age`, which will return zero results.

---

### Limited Country Coverage

Only Nigeria, Kenya, and Angola are supported. Queries mentioning any other African country or any country outside Africa are silently ignored with no `country_id` set.

---

### No Partial Age Group Coverage

Only `teenager` and `adult` are recognised. The `senior` age group (which exists in the data) has no keyword mapping. A query like `"show seniors"` or `"elderly users"` returns no age group filter.

---

### No Negation Support

The parser has no concept of negation. Phrases like `"not male"`, `"exclude nigeria"`, or `"non-adult"` are not handled and will either be ignored or mismatched.

---

### No Confidence or Error Feedback

There is no way to communicate back to the caller which parts of the query were understood, which were ignored, or whether the query was ambiguous. All parsing is silent.