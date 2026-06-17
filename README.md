<div align="center">

# fluxa-core

  [![Contributors][contributors-shield]][contributors-url]
  [![Forks][forks-shield]][forks-url]
  [![Stars][stars-shield]][stars-url]
  [![Issues][issues-shield]][issues-url]
  [![License: GPL v3][license-shield]][license-url]

  <p>
    The platform-agnostic Rust core behind <a href="https://github.com/KhooLy/Fluxa">Fluxa</a>, a media-streaming app.<br/>
    State management · Stream policy · Addon protocol · Effect-driven I/O
  </p>

</div>

---

## What is fluxa-core?

`fluxa-core` is a headless Rust library holding all of Fluxa's domain logic: content
discovery, stream selection, playback state, profiles, library, calendar, and external
sync (Trakt, Simkl). It contains no platform-specific code and never performs I/O itself.

**Rust never calls the network, touches disk, or talks to a player.** Instead, the engine
takes an action and returns state plus a list of typed *effects* describing what needs to
happen. The host platform executes those effects and reports results back through
`completeEffect`. The same Rust codebase runs unmodified on Android, desktop, and
(via WASM) the web — each platform supplies a thin shell that drives the loop.

```
Host  →  dispatch(action_json)
      ←  { state, effects: [{ id, type, payload }] }
Host  →  executes each effect (HTTP / storage / player / ...)
      →  completeEffect({ effectId, result })
      ←  { state, effects: [...] }
```

This repo also contains a companion crate, **`fluxa-streaming-engine/`**, which handles
the runtime streaming side: torrent download (via `librqbit`), local HTTP proxying, and
Dolby Vision / HDR10+ stream rewriting.

## Who uses this

| Platform | Repo | How it links |
|---|---|---|
| Android (mobile + TV) | [Fluxa](https://github.com/KhooLy/Fluxa) | JNI (primary, ~157 functions) + a small UniFFI surface |
| Desktop (Linux/macOS/Windows) | [FluxaDesktop](https://github.com/KhooLy/FluxaDesktop) | Plain Rust dependency — calls `FluxaCore`/`core_invoke` directly, no FFI marshaling |
| iOS / tvOS | not in this workspace | UniFFI (`bindings/uniffi.rs`) |
| webOS | not in this workspace | WASM (`bindings/wasm.rs`, `wasm` feature) |

See [`docs/integrating.md`](docs/integrating.md) for how each platform actually wires
this crate in, including how to add a new capability for a given platform.

## Architecture

- **`headless_engine/`** — the primary state machine. State is a typed `EngineState`
  struct made of per-feature sub-structs (home, detail, player, library, search, ...);
  cross-module writes go through `pub(super)` setters, never raw field access.
- **`app_state.rs`** — a second, simpler engine for overlapping concerns, used by
  Android via UniFFI. The split is intentional, not duplication to be cleaned up.
- Three uncoordinated exposure mechanisms, one per platform's needs: `core_api::FluxaCore`
  (minimal, desktop-only), `ffi::core_invoke` (string-routed dispatcher, desktop + Swift),
  and `bindings/jni.rs` (Android, no equivalent elsewhere).

Full architecture notes, the effect catalog, and the wire-format reference live in
[`docs/`](docs/):

- [`docs/overview.md`](docs/overview.md) — architecture, state engines, module map
- [`docs/effect-loop.md`](docs/effect-loop.md) — the dispatch/effect/completeEffect cycle
- [`docs/effects.md`](docs/effects.md) — every `EffectKind` and its payload shape
- [`docs/integrating.md`](docs/integrating.md) — per-platform integration guide
- [`docs/building.md`](docs/building.md) — features, commands, cross-compilation

## Building

```bash
cargo build                  # default (native) features — what Android uses
cargo test --lib             # ~190 tests, fast
cargo check --no-default-features --features wasm   # sanity-check the webOS/WASM path
```

Android cross-compilation needs the NDK targets:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
```

The companion streaming crate builds independently:

```bash
cd fluxa-streaming-engine && cargo build
```

See [`docs/building.md`](docs/building.md) for the full feature matrix, UniFFI binding
generation, and release-build details.

## Repo layout

```
src/                    domain logic, headless_engine, FFI bindings
fluxa-streaming-engine/ torrent + Dolby Vision/HDR10+ stream rewriting (separate crate)
fuzz/                   cargo-fuzz targets for parsers (episode matching, manifests, percent-decode)
docs/                   architecture, effects reference, integration guide
```

## License

GPL-3.0 — see [LICENSE](LICENSE).

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
