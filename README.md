content = """# Intelligence Query API (Rust Implementation)

An API for searching and filtering demographic profiles with Natural Language Processing (NLP) capabilities, built with **Rust**, **Axum**, and **SQLx**.

## Natural Language Parsing Approach

Our parser is implemented as a deterministic **Keyword-Based Extraction** engine in Rust. It utilizes pattern matching and string manipulation to transform unstructured natural language into a structured `Filters` struct.

### Supported Keywords & Mappings
The parser identifies tokens within the query and maps them to database fields as follows:

| Token Category | Rust Logic / Keywords | Database Filter |
| :--- | :--- | :--- |
| **Gender** | `male`, `males`, `man`, `men` / `female`, `females`, `woman`, `women` | `LOWER(gender)` |
| **Country** | `nigeria`, `kenya`, `rwanda` | `UPPER(country_id)` |
| **Age Groups** | `young` (Maps to 18-35 range), `teenager`, `adult`, `senior` | `age_group` or `min_age`/`max_age` |
| **Comparisons** | `above`, `over`, `older than`, `greater than` | `age >= $n` |
| **Comparisons** | `below`, `under`, `less than` | `age <= $n` |
| **Confidence** | `high confidence`, `certain` | `min_gender_probability >= 0.5` |

### Logic Workflow in Rust
1.  **Normalization**: The `String` input is passed through `.to_lowercase()` to ensure case-insensitive processing.
2.  **Stateful Parsing**: We instantiate a `Filters` struct with `Default::default()`. 
3.  **Token Scanning**: Using `.contains()`, the parser checks for fixed-vocabulary keywords (countries, genders, and age groups).
4.  **Value Extraction**: For numerical filters, we use custom helper functions (`extract_after`) that locate a comparison keyword and iterate through the subsequent characters using `.chars().take_while(|c| c.is_ascii_digit())` to parse `i32` values.
5.  **Option Handling**: The parser returns `Option<Filters>`. If no keywords are matched, it returns `None`, which the API handler converts into a `400 Bad Request` "uninterpretable query" response.

## Limitations & Edge Cases

* **Negation Handling**: The parser does not support logical `NOT`. A query like "not male" will incorrectly match "male" because it detects the substring.
* **Conjunctions**: All identified filters are joined via `AND`. We do not support `OR` logic (e.g., "men or women").
* **Geographical Granularity**: Only ISO-3166 alpha-2 country codes are currently mapped. It does not handle cities or regions.
* **Fuzzy Matching**: Rust's standard string matching is exact. There is no typo tolerance or Levenshtein distance calculation, meaning "nigerian" will not match "nigeria" unless explicitly handled.
* **Compound Range Logic**: Queries like "older than 20 but younger than 30" are processed sequentially; the last-found value may overwrite the first if not carefully structured.

## API Usage

### NLP Query Endpoint
`GET /api/profiles/query?q=young+males+from+kenya`

**Response Example:**
```json
{
  "status": "success",
  "data": [...],
  "pagination": {
    "page": 1,
    "limit": 10,
    "total": 5,
    "total_pages": 1
  }
}