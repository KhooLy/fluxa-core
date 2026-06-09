<div align="center">

# fluxa-core

  [![Contributors][contributors-shield]][contributors-url]
  [![Forks][forks-shield]][forks-url]
  [![Stars][stars-shield]][stars-url]
  [![Issues][issues-shield]][issues-url]
  [![License][license-shield]][license-url]

  <p>
    The platform-agnostic Rust brain behind <a href="https://github.com/KhooLy/Fluxa">Fluxa</a>.<br/>
    State management · Stream policy · Addon protocol · Effect-driven I/O
  </p>

</div>

---

## What is fluxa-core?

`fluxa-core` is a headless Rust library that contains all of Fluxa's domain logic. It handles content discovery, stream selection, playback state, user profiles, library management, calendar tracking, and external integrations (Trakt, Simkl) — with no platform-specific code inside Rust.

**Rust never calls the network.** Instead, the engine emits typed effects that the host platform (Kotlin on Android, or a native Rust runtime on desktop) executes and reports back. This keeps the same crate portable across Android (JNI), desktop (UniFFI), and future targets (WASM).

---

## Architecture

### The Effect Loop

```
Host (Kotlin / native)  →  dispatch(action_json)
                        ←  { state, effects: [{ id, type, payload }] }
Host                    →  executes each effect (HTTP / storage / player / ...)
                        →  completeEffect({ effectId, result })
                        ←  { state, effects: [...] }
```

The host owns all I/O. The Rust engine owns all decisions.

### Binding Layers

```
┌─────────────────────────────────────────────┐
│              fluxa-core (Rust)              │
│                                             │
│  headless_engine  ←  domain modules         │
│         ↓                                   │
│   ┌─────────────┬──────────────┐            │
│   │  JNI (jni)  │ UniFFI       │            │
│   │  Android    │ Desktop/WASM │            │
│   └─────────────┴──────────────┘            │
└─────────────────────────────────────────────┘
```

JNI bindings are compiled under the `native` feature (default). UniFFI bindings are always compiled and are the long-term cross-platform target.

---

## What's Inside

| Module | Responsibility |
|--------|----------------|
| `headless_engine` | Central state machine — dispatches actions, owns pending effects |
| `addon_protocol` | Manifest URL normalisation, resource URL construction, manifest merging |
| `stream_policy` | Stream selection, magnet building, audio/subtitle preference matching |
| `player_policy` | Backend selection (ExoPlayer / MPV / external), track state |
| `player_flow` | Playback state machine (load → select → play → scrobble) |
| `content_identity` | ID parsing and normalisation (IMDB, TMDB, Kitsu, episode locators) |
| `home_ranking` | Billboard and shelf ordering, continue-watching deduplication |
| `library_state` | Continue-watching badge computation, next-episode resolution |
| `profile_contract` | Profile activation, auth token merging, settings migration |
| `profile_prefs` | Typed read of user preferences from raw profile JSON |
| `search_plan` | Search query planning, result grouping, discover sort |
| `calendar_plan` | Calendar item filtering, widget rows, release notifications |
| `external_sync` | Trakt and Simkl API response parsing and history mapping |
| `addon_store` | Addon search policy, CloudStream repo URL normalisation |
| `addon_resource` | Addon HTTP response classification (success / empty / error) |
| `intro_segments` | introdb.app and AniSkip segment parsing, deduplication |
| `dolby_vision_rpu` | Dolby Vision RPU metadata extraction for HDR stream selection |
| `platform_plan` | Season/episode navigation planning for the detail screen |
| `tmdb_plan` | TMDB ID resolution hints, trailer mapping |
| `discovery_plan` | Discover catalog request planning |
| `watchlist_plan` | Watchlist toggle, offline grouping, progress merging |

---

## Building

### Prerequisites

- Rust toolchain (stable) — install via [rustup](https://rustup.rs/)
- For Android targets, the NDK and cross-compilation targets:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
```

### Library

```bash
cargo build --release
```

### Tests

```bash
cargo test
```

### Android (via Fluxa)

The Android app builds this crate as part of its Gradle build. See the [Fluxa](https://github.com/KhooLy/Fluxa) repo for the full build setup.

### UniFFI bindings (desktop)

```bash
cargo build --release --features uniffi-cli
cargo run --features uniffi-cli --bin uniffi-bindgen generate \
  --library target/release/libfluxa_core.so \
  --language kotlin \
  --out-dir bindings/
```

---

## Used By

- [Fluxa](https://github.com/KhooLy/Fluxa) — Android media hub (Kotlin + Jetpack Compose)
- [FluxaDesktop](https://github.com/KhooLy/FluxaDesktop) — Desktop media hub (Rust + Tauri)

---

<!-- MARKDOWN LINKS -->
[contributors-shield]: https://img.shields.io/github/contributors/KhooLy/fluxa-core.svg?style=for-the-badge
[contributors-url]: https://github.com/KhooLy/fluxa-core/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/KhooLy/fluxa-core.svg?style=for-the-badge
[forks-url]: https://github.com/KhooLy/fluxa-core/network/members
[stars-shield]: https://img.shields.io/github/stars/KhooLy/fluxa-core.svg?style=for-the-badge
[stars-url]: https://github.com/KhooLy/fluxa-core/stargazers
[issues-shield]: https://img.shields.io/github/issues/KhooLy/fluxa-core.svg?style=for-the-badge
[issues-url]: https://github.com/KhooLy/fluxa-core/issues
[license-shield]: https://img.shields.io/github/license/KhooLy/fluxa-core.svg?style=for-the-badge
[license-url]: https://github.com/KhooLy/fluxa-core/blob/master/LICENSE
