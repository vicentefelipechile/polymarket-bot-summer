# AGENTS.md — Polymarket Bot Summer

Operational guide for AI agents (and humans) working in this repository. Read this
**before** writing or moving code. It defines the project's architecture, its **target
folder structure**, and the conventions all code must follow.

---

## 1. What this project is

A Rust **terminal application (TUI)** for experimenting with high-frequency-style trading
on Polymarket. It is a **proof of concept** — not production-tested, handles real money.

- **UI:** full-screen TUI built on `ratatui` + `crossterm`.
- **Config:** stored **encrypted on disk** in `./summer.bot` (AES-256-GCM + Argon2id),
  unlocked by a password prompt. First run launches a **config wizard**. A legacy `.env`
  migration helper exists (`SecureConfig::migrate_from_env`) but is **not wired into the
  startup flow** — `.env`/`dotenvy` is not the live config path; `summer.bot` is.
- **AI:** optional market-analysis + chatbot powered by **Google Gemini**, with selectable
  "personalities" (`Summer`, `Anna`).
- **Persistence:** SQLite via `sqlx` (WAL mode).
- **Trading:** integration with `polymarket-hft` / `polymarket-client-sdk` is **partially
  stubbed**. `main.rs` logs *"Trading integration pending - running in demo mode"*. Do not
  assume orders actually reach the CLOB; mark incomplete paths with `// TODO:` + a reason.

### Entry flow (`src/main.rs`)

1. Init file-only logging to `bot.log` (level from `RUST_LOG`, default `info`). Logging
   never goes to stdout — it would corrupt the TUI.
2. Load config: if `./summer.bot` exists → password prompt (max 3 attempts); else →
   first-time wizard, which saves the encrypted file.
3. `config.validate()`.
4. CLOB `authenticate(&config.private_key)` → `init_database(&config.database_path)`.
5. Build `SpikeDetector` and `ExecutionEngine`.
6. If `ai_enabled` and a Gemini key is set, spawn the periodic AI analysis task.
7. `run_tui(...)` runs the main event loop until quit, then restores the terminal.

Note: the password prompt and wizard (steps 2) drive their own short-lived ratatui
terminal; `authenticate` (step 4) prints status with `println!` — this is fine because it
runs **before** the main TUI owns the terminal (see the logging rule in §4).

---

## 2. Folder structure (this is the law)

The crate is organized into **domains**. Every domain is a directory module whose `mod.rs`
does **wiring only** — declare submodules + re-export public items, no feature logic.

```
src/
├── lib.rs              # domain declarations + curated public re-exports
├── main.rs             # binary entry point: startup orchestration, terminal lifecycle
│
├── config/             # configuration domain
│   ├── mod.rs          # wiring
│   ├── secure_config.rs# SecureConfig + AiPersonality + lifecycle (migrate/validate/load/save)
│   └── crypto.rs       # AES-256-GCM / Argon2id encrypt/decrypt primitives
│
├── trading/            # trading domain
│   ├── mod.rs          # wiring
│   ├── auth.rs         # CLOB authentication -> AuthenticatedClient
│   ├── execution.rs    # ExecutionEngine: order placement (partly stubbed)
│   ├── markets.rs      # MarketService + MarketInfo: Gamma API + DB-backed market data
│   └── spike_detection.rs # SpikeDetector: volume velocity + order-book imbalance
│
├── data/               # data domain
│   ├── mod.rs          # wiring
│   ├── database.rs     # sqlx SQLite pool, schema creation, DbPool type alias
│   └── types.rs        # shared cross-module domain structs
│
├── ai/                 # AI domain
│   ├── mod.rs          # wiring
│   ├── client.rs       # GeminiClient, AiResponse — raw HTTP to Gemini
│   ├── analyzer.rs     # MarketAnalyzer, AiRecommendation, TradeAction
│   ├── chatbot.rs      # AiChatbot, ChatbotResponse, PendingConfirmation
│   └── personality.rs  # PersonalityTrait (+ re-exports AiPersonality from config)
│
└── tui/                # presentation domain
    ├── mod.rs          # wiring + run_tui() + the app event loop (lifecycle only)
    ├── app.rs          # App: central TUI state + handle_event/refresh_data
    ├── ui.rs           # draw(frame, app): pure rendering, NO state mutation / IO
    ├── events.rs       # EventHandler: tick + input polling
    ├── chat.rs         # ChatState: AI chat view state
    ├── config_wizard.rs# ConfigWizard: first-run setup flow
    ├── password_prompt.rs # PasswordPrompt: unlock screen
    └── settings.rs     # SettingsEditor, SettingsAction: in-app config editor
```

### Structure rules (non-negotiable)

- **Every domain is a directory with `mod.rs`.** Never put feature logic in a `mod.rs`; it
  only declares `pub mod` and re-exports. (The sole exception is `tui/mod.rs`, which owns
  `run_tui` + the event loop — keep that to lifecycle, not feature logic.)
- **No flat feature files directly under `src/`.** Only `lib.rs` and `main.rs` live at the
  top level. A new piece of functionality goes inside the domain it belongs to, or a new
  domain directory if it doesn't fit one.
- **One responsibility per file.** Split a type's data from heavy logic when it clarifies
  (e.g. `config/`: the `SecureConfig` model lives in `secure_config.rs`, the crypto
  primitives in `crypto.rs`). **Never split a single type's struct and its `impl` across
  files just by name** — that was the old `crypto.rs`/`config.rs` anti-pattern; don't
  recreate it.
- **Cross-module types go in `data/types.rs`.** If two domains need the same struct, it
  lives there. A type used by only one domain stays in that domain (e.g. `MarketInfo` for
  rich market data lives in `trading/markets.rs`, since trading owns it).
- **`tui/ui.rs` is render-only.** It reads `App` state and draws. State changes happen in
  `app.rs` (`handle_event`, `refresh_data`). No IO or mutation in the render path.
- **Everything must be wired in.** A file not declared through a `mod.rs` / `lib.rs` chain
  is dead code — delete it or wire it. (The removed `onboarding.rs` and
  `main_wizard_helper.rs` were exactly this.)

### Import path convention

Refer to items by their domain path: `crate::config::SecureConfig`,
`crate::data::DbPool`, `crate::trading::markets::MarketInfo`, `crate::trading::ExecutionEngine`.
Each domain's `mod.rs` re-exports its public items, so prefer `crate::<domain>::Item` over
deep paths unless you need to disambiguate.

---

## 3. Code style (enforced on every new/edited file)

This project follows Vicente's universal code-style conventions. Bring any file you touch
into line; don't leave new code below standard.

### File declaration order (top to bottom, no mixing)

1. File header block (`//!` module doc describing purpose)
2. `use` imports
3. Constants (`const` / `static`)
4. Types (`struct`, `enum`, `type` aliases)
5. Private/helper functions
6. Public functions / `impl` blocks
7. `#[cfg(test)] mod tests` at the bottom

### Section separators

Each logical section gets a **105-character** separator (count from `//` to the last `=`):

```rust
// =========================================================================================================
// Section Name
// =========================================================================================================
```

Use `//` for separators. Reserve `///` for item docs and `//!` for module docs.

### Imports — grouped, blank line between groups

1. `std::*`
2. External crates (`anyhow`, `tokio`, `sqlx`, `ratatui`, ...)
3. Internal (`crate::*`)

Named imports over globs. The ratatui prelude (`use ratatui::prelude::*;`) in existing TUI
code is a tolerated idiom — don't introduce new wildcard imports elsewhere. No unused
imports, ever.

### Naming (Rust idiomatic)

- Types/enums/traits: `PascalCase` · functions/methods/vars: `snake_case`
- Constants/statics: `UPPER_SNAKE_CASE` · modules/files: `snake_case`

### Language

- **All code artifacts are in English**: identifiers, comments, doc strings, log/error
  messages. Some legacy user-facing strings are still Spanish; migrate them to English when
  you touch them.
- Conversational replies to the user can be Spanish; the code stays English.

### Comments & errors

- Document **why**, not **what**. Inline comments go **above** the line, never to the right.
- Use `anyhow::Result` + `?` + `.context("...")` for app errors. **Never silently swallow
  an error** — at minimum log via `tracing`. Return early on error; don't nest success
  inside `if ok { ... }`.
- `thiserror` is a dependency but currently unused; prefer it for a typed, public error
  enum if/when a module needs callers to match on error variants. Don't reach for it just
  to wrap an internal error — `anyhow` is the default.

---

## 4. Project-specific conventions

- **Logging:** use `tracing` (`info!`/`warn!`/`error!`); it writes to `bot.log` only.
  **Never** `println!`/`eprintln!` while the TUI owns the terminal — it corrupts the
  display. Startup code that runs *before* `run_tui` (e.g. `trading::auth::authenticate`)
  may print, but new TUI-time code must not.
- **Terminal lifecycle:** any code entering raw mode / alternate screen MUST restore it on
  every exit path (see `cleanup_terminal` in `main.rs`). A panic mid-TUI without restore
  leaves the user's terminal broken.
- **Secrets:** the private key and Gemini API key live in `SecureConfig` as plain `String`
  fields — they are **not** zeroized in memory today (only the Argon2-derived encryption
  key in `config/crypto.rs` uses `Zeroizing`). Treat that as a known gap: never log secret
  values, never write them unencrypted, and don't widen their exposure. Hardening
  `SecureConfig`'s secret fields (e.g. `secrecy`/`zeroize`) is a welcome improvement.
- **Adding a config setting:** extend the `SecureConfig` struct in
  `config/secure_config.rs`, then add a default in `migrate_from_env`, a `validate()` rule
  if constrained, and wiring in `ConfigWizard` / `SettingsEditor`. Old `summer.bot` files
  must still deserialize or migrate.
- **Database:** schema lives in `data/database.rs::create_schema` (idempotent
  `CREATE TABLE IF NOT EXISTS`). WAL mode required. Use `sqlx::query`/`query_as`.
- **AI calls:** go through `GeminiClient` (`ai/client.rs`); rate-limited with `governor`.
  No ad-hoc HTTP to Gemini from feature code.

---

## 5. Build, run, test

**The cargo target directory is `C:\rust_builds`** (set via `CARGO_TARGET_DIR`), not the
local `./target`. Build artifacts and `cargo` lock contention live there.

```bash
cargo build              # debug build
cargo build --release    # optimized (LTO, opt-level 3) — used for real runs
cargo run                # launches the TUI (password prompt or first-run wizard)
cargo test               # unit tests (config validation, crypto roundtrip, ai, spike)
cargo clippy             # lint — keep clean on touched files
cargo fmt                # rustfmt — run before committing
```

- First run with no `./summer.bot` triggers the wizard; later runs ask for the password.
  Delete `./summer.bot` to reset onboarding.
- Set log verbosity with `RUST_LOG` (`trace`/`debug`/`info`/`warn`/`error`, default
  `info`). Logs go to `./bot.log`; tail it to debug TUI-time behavior.
- The TUI needs a real terminal — it won't behave correctly without a tty.
- **Runtime artifacts are git-ignored and must stay that way:** `summer.bot` (encrypted
  config holding the private key), `bot.log`, and the SQLite DB (`*.db`/`-wal`/`-shm`).
  Never commit them; never add a fixture that contains a real key.

---

## 6. Checklist for a new file

- [ ] `//!` module header describing its purpose
- [ ] Lives inside a domain directory (never flat under `src/`)
- [ ] Sections in canonical order with 105-char separators
- [ ] Imports grouped std → external → internal, no unused
- [ ] No magic numbers/strings (extract to `const`)
- [ ] Cross-module types in `data/types.rs`; domain-local types in the domain
- [ ] Errors handled, never silently swallowed; English messages
- [ ] Declared through its domain `mod.rs` and (if public) re-exported in `lib.rs`
- [ ] `cargo fmt` + `cargo clippy` clean
