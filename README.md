# lavalink-rs

A `lavalink` API wrapping library for every `tokio` discord crate.

## To-Do

- [x] `native_tls` backend
- [x] Player queues
- [x] Readbale player queues
- [ ] Streamable queue reader
- [x] Search engine helpers
- [x] Write basic cocumentation
- [ ] Improve documentation with examples, better formatting, and fill in missing data
- [ ] Expand event logging
- [ ] Examples
- [x] Songbird utilities
- [x] Serenity utilities
- [x] Twilight utilities
- [ ] Lavasnek (PyO3)
- [ ] Tests
- [ ] Implement abstractions for ease of use
- [ ] Implement more node selection methods:
 - [ ] Round-Robin
 - [ ] Region based
 - [x] Load balancer
 - [ ] Main and fallback



## Links to download stuff you will need

- [Lavalink repository](https://github.com/lavalink-devs/Lavalink) (V4)
- [Java download](https://adoptium.net/) (JRE 17)

## How to use

Install the version from crates.io:

```toml
lavalink-rs = "0.10.0-alpha"

# or

[dependencies.lavalink-rs]
version = "0.10.0-alpha"
```

Or the development release:

```toml
lavalink-rs = { git = "https://gitlab.com/vicky5124/lavalink-rs/", branch = "main"}

# or

[dependencies.lavalink-rs]
git = "https://gitlab.com/vicky5124/lavalink-rs/"
branch = "main"
```

If you wish to use a development version of songbird, add the following to the Cargo.toml:

```toml
[patch.crates-io.serenity]
git = "https://github.com/serenity-rs/songbird"
branch = "next"
```

### Features

- `rustls`: default feature. Use rustls.
- `native-tls`: default feature. Use the system native tls.
- `serenity-rustls`: for (serenity)[https://lib.rs/crates/serenity] with rustls support.
- `serenity-native`: for (serenity)[https://lib.rs/crates/serenity] with native-tls support.
- `songbird`: for (songbird)[https://lib.rs/crates/songbird] support.
- `twilight`: for (twilight-model)[https://lib.rs/crates/twilight-model] support.
