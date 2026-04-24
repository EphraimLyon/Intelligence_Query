# Intelligence Query API

A Rust/Axum REST API for querying demographic profile data stored in PostgreSQL.

---

## Base URL

```
https://intelligencequery-production.up.railway.app
```

---

## Endpoints

### `GET /api/profiles`

List profiles with optional filtering, sorting, and pagination.

**Query parameters**

| Parameter               | Type    | Description                                                     |
|-------------------------|---------|-----------------------------------------------------------------|
| `name`                  | string  | Case-insensitive partial match on name                          |
| `gender`                | string  | `male` or `female` (case-insensitive)                           |
| `country`               | string  | Filter by country name (case-insensitive exact match)           |
| `country_id`            | string  | ISO 3166-1 alpha-2 code (e.g. `NG`, `KE`)                      |
| `age_group`             | string  | `child`, `teenager`, `young_adult`, `adult`, `senior`           |
| `min_age`               | integer | Minimum age (inclusive)                                         |
| `max_age`               | integer | Maximum age (inclusive)                                         |
| `min_gender_probability`| float   | Minimum gender prediction confidence (0.0–1.0)                  |
| `min_country_probability`| float  | Minimum country prediction confidence (0.0–1.0)                 |
| `sort_by`               | string  | `age`, `name`, `country_name`, `gender`, `created_at`, `gender_probability`, `country_probability` |
| `order`                 | string  | `asc` or `desc` (default `desc`)                                |
| `page`                  | integer | Page number (default `1`)                                       |
| `limit`                 | integer | Results per page (default `10`, max `100`)                      |

**Response envelope**

```json
{
  "status": "success",
  "count": 142,
  "page": 1,
  "limit": 10,
  "data": [ ... ]
}
```

**Error response (e.g. invalid `sort_by`)**

```json
{
  "status": "error",
  "message": "Invalid sort_by value 'foo'. Must be one of: age, name, ..."
}
```

---

### `POST /api/profiles`

Create a new profile.

**Request body (JSON)**

```json
{
  "name": "Amara Osei",
  "gender": "female",
  "gender_probability": 0.97,
  "age": 28,
  "age_group": "young_adult",
  "country_id": "GH",
  "country_name": "Ghana",
  "country_probability": 0.88
}
```

**Response**

```json
{ "status": "created", "id": "01927f..." }
```

---

### `GET /api/profiles/search`

Search profiles by name, gender, country, or age group.

**Query parameters**: `search`, `gender`, `country`, `age_group`, `page`, `limit`

---

### `GET /api/profiles/query`

Natural-language profile search.

**Query parameters**

| Parameter | Description                              |
|-----------|------------------------------------------|
| `q`       | Free-text query (required)               |
| `page`    | Page number (default `1`)                |
| `limit`   | Results per page (default `10`, max `100`)|

**Supported query patterns**

| Query example                        | Parsed as                              |
|--------------------------------------|----------------------------------------|
| `young males`                        | gender=male, age 18–35                 |
| `females above 30`                   | gender=female, min_age=30              |
| `people from Nigeria`                | country_id=NG                          |
| `adult males from Kenya`             | gender=male, age_group=adult, country_id=KE |
| `Male and female teenagers above 17` | age_group=teenager, min_age=17         |

**Response**

```json
{
  "status": "success",
  "query": "young males",
  "parsed": {
    "gender": "male",
    "age_group": null,
    "min_age": 18,
    "max_age": 35,
    "country_id": null
  },
  "count": 87,
  "page": 1,
  "limit": 10,
  "data": [ ... ]
}
```

---

### `GET /api/profiles/{id}`

Retrieve a single profile by UUID.

---

### `DELETE /api/profiles/{id}`

Delete a profile by UUID.

---

## Profile object

```json
{
  "id": "01927f3e-...",
  "name": "Chisom Eze",
  "gender": "female",
  "gender_probability": 0.95,
  "age": 24,
  "age_group": "young_adult",
  "country_id": "NG",
  "country_name": "Nigeria",
  "country_probability": 0.91,
  "created_at": "2025-01-15T10:30:00Z"
}
```

---

## Validation rules

- `sort_by` must be one of the allowed column names; otherwise a `400` error is returned.
- `limit` is capped at **100** regardless of what is supplied.
- `country_id` and `gender` comparisons are case-insensitive.

---

## Stack

- **Language**: Rust (edition 2021)
- **Framework**: Axum 0.7
- **Database**: PostgreSQL (via sqlx)
- **Deployment**: Railway