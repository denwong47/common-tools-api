# common-tools-api

Small async Rust clients for third-party web APIs. You pick the providers you
need with Cargo features, and only those are compiled.

Every provider works the same way. You create a `Session` and call its methods
with typed queries, and you get back typed results. Each session rate limits
its own requests to stay within that provider's limits.

## Providers

| Provider | Feature             | APIs                                                                                  | Authentication         | Default rate limit |
| -------- | ------------------- | ------------------------------------------------------------------------------------- | ---------------------- | ------------------ |
| KeyCDN   | `provider-keycdn`   | [IP Location Finder](https://tools.keycdn.com/geo#ip-location-finder-api)             | Your site's URL        | 1 request / 350 ms |
| Geoapify | `provider-geoapify` | [Reverse Geocoding](https://apidocs.geoapify.com/docs/geocoding/reverse-geocoding/)   | API key                | 5 requests / s     |

No provider is enabled by default.

## Installation

The crate is not published on crates.io. Add it as a git dependency and
enable the providers you need. You also need [Tokio](https://tokio.rs/),
because requests are made through `reqwest`:

```toml
[dependencies]
keycdn-api = { git = "https://github.com/denwong47/common-tools-api", features = ["provider-geoapify"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The package is still named `keycdn-api` from when it only supported KeyCDN,
so you import it as `keycdn_api`.

### Features

| Feature                | Default | Description                                                                 |
| ---------------------- | ------- | --------------------------------------------------------------------------- |
| `provider-keycdn`      | no      | The KeyCDN provider.                                                        |
| `provider-geoapify`    | no      | The Geoapify provider.                                                      |
| `rustls`               | yes     | Use `rustls` for TLS.                                                       |
| `native-tls`           | no      | Use the platform's native TLS instead. Disable default features to use it on its own. |
| `preserve-query-order` | no      | Send query parameters in the order their struct fields are declared, rather than alphabetically. |
| `test-integration`     | no      | Enable tests that call the live APIs.                                       |

## Usage

### KeyCDN: IP Location Finder

```rust
use keycdn_api::providers::keycdn::Session;

#[tokio::main]
async fn main() -> keycdn_api::Result<()> {
    let session = Session::new("https://www.example.com")?;

    let location = session.location_lookup("8.8.8.8").await?;
    println!("{} is in {:?}", location.ip, location.country_name);

    Ok(())
}
```

KeyCDN requires every request to say which site is making it, in a
`User-Agent` header of the form `keycdn-tools:https://www.example.com`. The URL
you pass to `Session::new` is used for that header. KeyCDN drops requests
without a valid header and does not send a response. You can change the URL
later with `Session::set_url`.

`location_lookup` returns a `LocationLookupResult` with the IP's country,
region, city, postal code, continent, coordinates, time zone, ASN and ISP. The
data comes from MaxMind GeoLite, so many addresses have no city or region.
KeyCDN does not document which fields can be `null`, so every field apart from
`host` and `ip` is an `Option`.

### Geoapify: Reverse Geocoding

```rust
use keycdn_api::providers::geoapify::{ReverseGeocodingQuery, Session};
use keycdn_api::providers::geoapify::models::reverse_geocoding::LocationType;

#[tokio::main]
async fn main() -> keycdn_api::Result<()> {
    let session = Session::new(std::env::var("GEOAPIFY_API_KEY").unwrap())?;

    // The place nearest to the coordinates.
    let query = ReverseGeocodingQuery::new(51.8749, 14.0082);
    let response = session.reverse_geocode(&query).await?;
    if let Some(place) = response.results.first() {
        println!("{:?}", place.formatted);
    }

    // Up to 3 cities, in German, limited to Germany.
    let query = ReverseGeocodingQuery::new(51.8749, 14.0082)
        .with_type(LocationType::City)
        .with_lang("de")
        .with_limit(3)
        .with_country_codes(["de"]);
    for city in session.reverse_geocode(&query).await?.results {
        println!("{:?}", city.city);
    }

    Ok(())
}
```

The API key is sent in the `x-api-key` header rather than as a query
parameter, so it does not show up in logged URLs. It is also hidden when the
session's headers are printed with `{:?}`. You can change it later with
`Session::set_api_key`.

`reverse_geocode` returns a `ReverseGeocodingResponse`:

- `results` lists the matching places, best first. If nothing is found, it is
  empty rather than an error.
- `query` is the request as Geoapify understood it, including the coordinates'
  plus code.

Each result is a `GeocodingResult`. It has the place's name, formatted
address, coordinates, distance from the requested point, result type and
category, plus its time zone, rank and data source. It also has the common
address components, from `country` down to `housenumber`. Geoapify documents
around 40 more address components that depend on the country, such as
`province` and `arrondissement`. Those, and any fields Geoapify adds in future,
are kept as raw JSON in `other`. Only `lat` and `lon` are guaranteed to be
present.

The query always asks for `format=json`. Geoapify's default GeoJSON format has
a different shape. Coordinates and other values are not checked locally;
Geoapify responds with HTTP 400 if they are out of range.

Several `with_country_codes` codes are sent as repeated `countrycodes`
parameters. Geoapify does not document how to send more than one, so this has
not been confirmed against the live API.

## Sessions and rate limiting

Each `Session` has its own rate limiter, set to that provider's default limit
(see [Providers](#providers)). The limiter starts counting each request's
window when the request *finishes*, so uneven network latency cannot push more
requests into one window on the provider's side.

All requests made through one session share its limiter. To make concurrent
requests, share a single session (by reference or in an `Arc`) instead of
creating several:

```rust
use std::sync::Arc;
use keycdn_api::providers::keycdn::Session;

let session = Arc::new(Session::new("https://www.example.com")?);

let handles: Vec<_> = ["1.1.1.1", "9.9.9.9", "example.com"]
    .into_iter()
    .map(|host| {
        let session = Arc::clone(&session);
        tokio::spawn(async move { session.location_lookup(host).await })
    })
    .collect();

for handle in handles {
    match handle.await.expect("task panicked") {
        Ok(location) => println!("{}: {:?}", location.host, location.city),
        Err(err) => eprintln!("lookup failed: {err}"),
    }
}
```

To use a different limit, for example on a paid Geoapify plan, pass a
`RateLimit` to `Session::new_with_rate_limit`:

```rust
use std::num::NonZeroU32;
use keycdn_api::models::rate_limit::RateLimit;
use keycdn_api::providers::geoapify::Session;

let session = Session::new_with_rate_limit(
    api_key,
    RateLimit::new_per_second(NonZeroU32::new(30).unwrap()),
)?;
```

`RateLimit::new(max_requests, duration)`, `RateLimit::new_per_minute` and
`RateLimit::new_per_hour` are also available.

What happens when you go over a limit depends on the provider:

- **KeyCDN** documents its limit as "3r/s". It does not send an HTTP 429.
  It closes the connection, and that shows up as `Error::Http`, which looks the
  same as a network failure. The default of one request every 350 ms keeps
  requests from being queued or dropped. The doc comment on the KeyCDN
  session's `rate_limit` explains how this value was chosen.
- **Geoapify** responds with HTTP 429, which shows up as
  `Error::UnexpectedResponse`.

## Serialization

Result types implement `serde::Serialize` and `Deserialize`. They deserialize
from the provider's field names and serialize to `camelCase`, leaving out
`None` fields. A few Geoapify names are mapped by hand: `NUTS_3` becomes
`nuts3`, `offset_STD` becomes `offsetStd`, and `iso3166_2` is kept as it is.
The keys in `GeocodingResult::other` are left as Geoapify returns them.

Query types only implement `Serialize`. They are turned into query parameters
by the `CanExportQuery` trait in `keycdn_api::traits::query`. Arrays become
repeated parameters, and `None` fields are left out.

## Errors

Every function returns `keycdn_api::Result<T>`. The error type is
`keycdn_api::Error`:

| Variant              | Cause                                                                  |
| -------------------- | ---------------------------------------------------------------------- |
| `InvalidSiteId`      | KeyCDN: the site URL could not be parsed, or (in `set_url`) has no host or is not HTTP(S). |
| `InvalidApiKey`      | Geoapify: the API key is empty, or cannot be used as a header value.   |
| `InvalidHeaderValue` | KeyCDN: the site URL cannot be used in a `User-Agent` header.          |
| `InvalidQueryValue`  | A query could not be turned into query parameters.                     |
| `InvalidUrl`         | An endpoint URL could not be built.                                    |
| `InvalidRequest`     | A body was given for a `GET` or `HEAD` request.                        |
| `Http`               | Connection error or timeout. For KeyCDN, this includes going over the rate limit. |
| `UnexpectedResponse` | The provider responded with a non-2xx status. For Geoapify, `body` holds its JSON error, e.g. `{"statusCode":401,"error":"Unauthorized","message":"Invalid apiKey"}`. |
| `Json`               | The response body was not the expected JSON.                           |
| `Api`                | KeyCDN reported an error, e.g. `"Hostname or IP is not valid."`        |
| `MissingData`        | KeyCDN reported success but returned no data.                          |

`Error` is `#[non_exhaustive]`, so `match` statements on it need a wildcard
arm.

## Development

```sh
cargo test --features provider-keycdn,provider-geoapify                     # unit tests only
cargo test --features provider-keycdn,provider-geoapify,test-integration    # also call the live APIs
```

The Geoapify integration tests read an API key from `GEOAPIFY_API_KEY`. If it
is not set, the tests that need it print a message and pass without calling
the API. The test that checks an invalid key is rejected runs either way.

The rate limiter tests run on Tokio's paused clock, so they finish instantly
and give the same result every run. Set `RUST_LOG` to adjust the trace output
from tests.

CI runs `cargo fmt --check`, `cargo clippy -D warnings` and `cargo test` with
`--all-features`, so it also runs the integration tests against the live APIs.

### Adding a provider

1. Add a `provider-<name>` feature in `Cargo.toml`, and gate a
   `pub mod <name>` on it in `src/providers/mod.rs`.
2. Write `src/providers/<name>/session.rs`: a `Session` that implements
   `models::provider::Provider` (its name and default `RateLimit`) and sends
   requests through its `RateLimiter`. The KeyCDN and Geoapify sessions can be
   used as templates.
3. Put request and response types in `models/`. Query types implement
   `CanExportQuery`.
4. Add one module per API under `api/`, each adding a method to `Session`.

## AI/LLM Involvement Declaration

The crate's design, including the provider layout, the sessions, the rate
limiter and the `CanExportQuery` trait, was designed and coded by hand,
except for the following:

- **KeyCDN**
    - The `LocationLookup*` structs and the `Error` model were created by an
      LLM to save time. Both were then adjusted by hand.
    - An LLM reviewed the code and applied some fixes to `rate_limit.rs`.
- **Geoapify** (written by an LLM, from a hand-written session skeleton and
  `CanExportQuery`)
    - The data models in `models/geocoding.rs` and
      `models/reverse_geocoding.rs`, based on Geoapify's documentation and
      OpenAPI spec. They were then adjusted by hand.
    - The Reverse Geocoding route in `api/reverse_geocoding.rs`.
    - The changes to `session.rs` that send the API key in the `x-api-key`
      header and fix the base URL, and the session's unit tests.
- This README was rewritten by an LLM for the multi-provider crate.
