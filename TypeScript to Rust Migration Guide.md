# TypeScript to Rust Migration Guide

A guide for developers with **advanced TypeScript** experience moving to Rust. It skips the basics and focuses on how TS concepts map to Rust, and where the TS mental model breaks.

> [!note] How this repo uses this guide
> This file is the reference for how a TypeScript idea is written in Rust here (`AGENTS.md`, *Code style*). Write the code with the forms this guide shows. Use a Rust feature that is not here only when the code really needs it, and then add it to this guide, with its TypeScript equivalent, in the same commit. Every section has the TS side first and the Rust side after it.

> [!tip] Core mindset shift
> In TypeScript, the type system is a _description_ layered over a dynamic runtime (structural, erased, unsound by design). In Rust, the type system _is_ the program: types are nominal, exist at compile time to drive codegen, and are sound. Plus a new axis you never had to think about: **who owns each value, and for how long**.

---

## 1. Tooling map

| TypeScript              | Rust                                      |
| ----------------------- | ----------------------------------------- |
| `npm` / `pnpm`          | `cargo`                                   |
| `package.json`          | `Cargo.toml`                              |
| `package-lock.json`     | `Cargo.lock`                              |
| npm registry            | [crates.io](https://crates.io)            |
| `tsc --noEmit`          | `cargo check`                             |
| `eslint`                | `cargo clippy`                            |
| `prettier`              | `cargo fmt`                               |
| `vitest` / `jest`       | `cargo test` (built-in) / `cargo nextest` |
| `c8` / `istanbul`       | `cargo llvm-cov`                          |
| `tsx watch` / `nodemon` | `cargo watch -x run`                      |
| `npx foo`               | `cargo install foo` then `foo`            |
| `npm install foo`       | `cargo add foo`                           |
| monorepo workspaces     | Cargo workspaces (`[workspace]`)          |
| `tsconfig` `strict`     | always on — there is no "loose" mode      |
| TypeDoc                 | `cargo doc --open` (`///` doc comments)   |
| `engines.node`          | `rust-version` in `Cargo.toml` (MSRV)     |

Install via `rustup` (preferred over distro packages): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`.

### Optional dependencies → cargo features

In npm an optional part of a package is a `peerDependency` or a runtime `if (hasRedis)`. In Rust it is a **feature**: a compile-time switch in `Cargo.toml`, and the code behind it is not compiled when the feature is off.

```ts
// package.json
{ "peerDependencies": { "ioredis": "^5" }, "peerDependenciesMeta": { "ioredis": { "optional": true } } }

// index.ts
if (config.type === "redis" && !hasIoredis) {
  throw new DoyenError("DOYEN_UNSUPPORTED", "install ioredis to use the redis type");
}
```

```toml
# Cargo.toml
[features]
default = ["redis", "mongodb"]  # all on unless the user opts out
redis = []
mongodb = []
```

```rust
#[cfg(feature = "redis")]
mod redis;                      // the module exists only when the feature is on
                                // (the adapter changes add such modules)

impl DatabaseKind {
    fn is_built_in(self) -> bool {
        match self {
            DatabaseKind::Redis => cfg!(feature = "redis"),        // a plain `bool`
            DatabaseKind::Mongodb => cfg!(feature = "mongodb"),
        }
    }
}

fn check_feature(database: DatabaseKind) -> Result<(), Error> {
    if database.is_built_in() {
        return Ok(());
    }
    let name = database.name();
    Err(Error::new(
        ErrorCode::Unsupported,
        format!("the {name} type needs the cargo feature \"{name}\", which is off in this build"),
    ))
}
```

The user picks: `cargo add doyen --no-default-features --features redis`. Tests run with `--all-features` and with `--no-default-features` to cover both.

Two forms of the switch:

- `#[cfg(feature = "redis")]` **removes the item** from the build (a `mod`, a `fn`, a `struct`). It is like a bundler that drops a file.
- `cfg!(feature = "redis")` is an **expression**: `true` or `false`, known when the crate is built (≈ a `const hasRedis = false` that the bundler fills in). Both branches of the `if` or `match` are still compiled and type-checked, so the run-time check above works with every feature combination and needs no second function behind `#[cfg(not(...))]`.

Use `cfg!` when the code only needs the yes or no (here: to return an error), and `#[cfg]` when the code behind the switch must not exist.

---

## 2. Types: structural → nominal

TS is **structural**: any object with the right shape fits. Rust is **nominal**: two structs with identical fields are different types.

```ts
interface User {
  id: number;
  name: string;
}
const u: User = { id: 1, name: "Ana" }; // any matching shape works
```

```rust
#[derive(Debug, Clone, PartialEq)]
struct User {
    id: u64,
    name: String,
}

let u = User { id: 1, name: "Ana".to_owned() };
```

Notes:

- `number` splits into `i8..i128`, `u8..u128`, `isize/usize`, `f32/f64`. No implicit conversion — use `as` (lossy) or `TryFrom` (checked).
- `string` splits into `String` (owned, heap) and `&str` (borrowed view). See [Strings](#4-strings).
- `#[derive(...)]` replaces things you got "for free" in JS: debug printing, cloning, equality, hashing, serialization.
- No `any`. The closest escape hatches are generics, trait objects (`Box<dyn Trait>`), or `serde_json::Value` for truly dynamic data.

### Type aliases vs. newtypes

```ts
type UserId = string & { __brand: "UserId" }; // branded type hack
type Meters = number;                          // plain alias
```

```rust
struct UserId(String); // newtype: zero-cost, truly distinct type
type Meters = f64;     // plain alias, NOT distinct (like TS `type`)
```

### Numbers that mean something: `Duration` and `Instant`

In TS a time is a `number` of milliseconds and you keep the unit in the name (`timeoutMs`). Rust has types for it, so the unit cannot be mixed up.

```ts
const timeoutMs = 10_000;
const startedAt = performance.now();            // monotonic, not wall clock
const elapsedMs = performance.now() - startedAt;
if (elapsedMs > timeoutMs) { /* expired */ }
```

```rust
use std::time::Duration;
use tokio::time::Instant;                        // monotonic, not wall clock

let timeout = Duration::from_millis(10_000);
let started_at = Instant::now();
let elapsed: Duration = started_at.elapsed();
if elapsed > timeout { /* expired */ }
```

- Keep the public config in `u64` milliseconds (`timeout_ms`), as the TS API does, and turn it into a `Duration` once, when the config is checked.
- Integer math does not overflow silently: `a.checked_add(b)` returns `Option` (≈ `Number.isSafeInteger` check), `a.saturating_sub(b)` stops at 0 (≈ `Math.max(0, a - b)`).

### Module-level constants

```ts
export const DEFAULT_TIMEOUT_MS = 10_000;
const CODES = ["DOYEN_AUTH", "DOYEN_LEFT"] as const;
```

```rust
pub const DEFAULT_TIMEOUT_MS: u64 = 10_000;
const CODES: [&str; 2] = ["DOYEN_AUTH", "DOYEN_LEFT"];
```

`const` is inlined; `static` is one value in memory (use it for a global that must have one address, rare).

### A constant that belongs to a type: associated `const`

```ts
class Backoff {
  static readonly FIRST_DELAY_MS = 100;
  private nextDelayMs = Math.min(Backoff.FIRST_DELAY_MS, this.longestDelayMs);
}
```

```rust
impl Backoff {
    const FIRST_DELAY: Duration = Duration::from_millis(100);

    pub(super) fn new(longest_delay: Duration) -> Backoff {
        Backoff {
            next_delay: Backoff::FIRST_DELAY.min(longest_delay),
            longest_delay,
        }
    }
}
```

An associated `const` lives in an `impl` block and is read with the type name, like a `static readonly` member. It is private unless marked `pub`.

---

## 3. Discriminated unions → `enum`

This is where Rust will feel like home. TS discriminated unions are Rust enums, but with exhaustive `match` enforced by the compiler.

```ts
type Shape =
  | { kind: "circle"; radius: number }
  | { kind: "rect"; w: number; h: number };

function area(s: Shape): number {
  switch (s.kind) {
    case "circle":
      return Math.PI * s.radius ** 2;
    case "rect":
      return s.w * s.h;
  }
}
```

```rust
enum Shape {
    Circle { radius: f64 },
    Rect { w: f64, h: f64 },
}

fn area(s: &Shape) -> f64 {
    match s {
        Shape::Circle { radius } => std::f64::consts::PI * radius.powi(2),
        Shape::Rect { w, h } => w * h,
    }
}
```

- No `never`-check trick needed — non-exhaustive `match` is a compile error.
- `match` supports guards (`x if x > 0 =>`), ranges (`1..=9`), nested destructuring, and bindings (`n @ 1..=9`).

### One variant only: `if let`, `let ... else`, `matches!`

```ts
// early return when it is not the variant we want
if (s.kind !== "circle") {
  return err("not a circle");
}
const { radius } = s;

// just a boolean
const isCircle = s.kind === "circle";
```

```rust
let Shape::Circle { radius } = s else {
    return Err("not a circle");
};

let is_circle = matches!(s, Shape::Circle { .. });
```

### String unions → unit enums

A TS string union (`"master" | "slave" | "offline"`) is a Rust enum with no data. Give it a `Display` so logs and bindings get the TS name back.

```ts
type State = "master" | "slave" | "offline";
console.log(state); // prints "master"
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Master,
    Slave,
    Offline,
}

impl State {
    pub fn as_str(&self) -> &'static str {
        match self {
            State::Master => "master",
            State::Slave => "slave",
            State::Offline => "offline",
        }
    }
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
```

This repo uses both forms. `ErrorCode` is written as above: `as_str` holds the `match`, and `Display` calls it (use this form when other code, such as a binding, needs the text too). `OfflineReason` has the `match` inside `fmt`, because nothing else needs the text:

```rust
impl fmt::Display for OfflineReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            OfflineReason::NotJoined => "notJoined",
            OfflineReason::Connecting => "connecting",
            OfflineReason::ConnectionLost => "connectionLost",
            OfflineReason::Reconnecting => "reconnecting",
            OfflineReason::Left => "left",
        };
        formatter.write_str(name)
    }
}
```

`Status` has no `Display`. The text of its state comes from a plain function, `state_name(status: &Status) -> &'static str`, in `engine/state.rs`.

### An enum that may grow: `#[non_exhaustive]`

In TS, adding a member to a union breaks every exhaustive `switch` of every user. Rust can mark a public enum or struct so that users outside the crate must keep a `_ =>` arm (and cannot build the struct with a literal). Add it to a public enum or struct that the specs say will grow in a minor version: `ErrorCode`, and the config types `DatabaseConfig`, `RedisConfig` and `MongodbConfig` (a new setting is a new field). `Status` and `OfflineReason` do not have it.

```ts
// TS has no equivalent; users write a `default:` by convention
switch (code) {
  case "DOYEN_AUTH": ...
  default: ...       // needed when the library adds a code later
}
```

```rust
#[non_exhaustive]
pub enum ErrorCode {
    InvalidConfig,
    Auth,
}
// outside the crate: `match code { ErrorCode::Auth => ..., _ => ... }` is required
```

---

## 4. Strings

| TS               | Rust                  | When                              |
| ---------------- | --------------------- | --------------------------------- |
| `string` (param) | `&str`                | read-only input                   |
| `string` (owned) | `String`              | you need to store / mutate it     |
| string literal   | `&'static str`        | lives for the whole program       |
| template literal | `format!("{a} {b}")`  |                                   |
| `.length`        | `.len()` (**bytes**!) | use `.chars().count()` for chars  |
| `s[0]`           | not allowed           | `s.chars().next()` / byte slicing |

Rule of thumb: **accept `&str`, return `String`**.

```ts
function greet(name: string): string {
  return `Hello, ${name}`;
}

greet("Ana");
greet(user.name);

const copy = name; // strings are values in JS: no copy to think about
```

```rust
fn greet(name: &str) -> String {
    format!("Hello, {name}")
}

greet("Ana");                    // &'static str
greet(&my_string);               // &String auto-derefs to &str

let copy: String = name.to_owned();   // explicit copy from &str to String
```

- `to_owned()`, `to_string()`, `String::from(s)` and `.into()` can all make a `String` from a `&str`. This repo picks by meaning:
  - `to_owned()` for a plain copy of a `&str` (`"Ana".to_owned()`, `group.to_owned()`);
  - `to_string()` for a value shown with `Display`, such as a number, an enum or an error;
  - `.into()` only for a parameter typed `impl Into<String>` (`url.into()` in `RedisConfig::new`), where the caller may give a `&str` or a `String`.
- A struct field is `String` (owned) unless there is a strong reason; a `&str` field forces a lifetime on the struct (see [Lifetimes](#lifetimes)).

### Borrowed or owned, decided at run time: `Cow`

```ts
function decode(raw: string): string {
  return raw.includes("%") ? decodeURIComponent(raw) : raw;   // the same string, or a new one
}
```

In JS the caller cannot tell the two cases apart and does not care. In Rust a function sometimes returns the input as it is (a `&str`) and sometimes a new `String`. `Cow<'a, str>` ("clone on write") is one type for both: it holds either a borrow or an owned `String`. The library gives it to you; you rarely write it. `.into_owned()` turns it into a `String` (it copies only when it was a borrow).

```rust
// `query_pairs()` of the `url` crate gives `Cow<str>` for the name and the value.
.map(|(name, value)| {
    let value = if holds_secret(&name) {
        HIDDEN.to_owned()
    } else {
        value.into_owned()          // Cow<str> -> String
    };
    (name.into_owned(), value)
})

let host_name = gethostname::gethostname().to_string_lossy().into_owned();
```

---

## 5. Ownership & borrowing (the new part)

There is no GC. Every value has exactly **one owner**; when the owner goes out of scope, the value is dropped.

```ts
const a = { name: "hi" };
const b = a;          // both names point to the same object
a.name = "changed";   // b.name is "changed" too
```

```rust
let a = String::from("hi");
let b = a;            // MOVE: `a` is no longer usable
// println!("{a}");   // compile error: use of moved value
```

In TS, `const b = a` copies a reference and both point to the same object. In Rust, that's a **move** for heap types. Your options:

| You want                         | Rust                               | TS analogy                  |
| -------------------------------- | ---------------------------------- | --------------------------- |
| Read without taking ownership    | `&T` (shared borrow)               | passing a `Readonly<T>`     |
| Mutate without taking ownership  | `&mut T` (exclusive borrow)        | passing an object to mutate |
| A separate copy                  | `.clone()` (on an `Arc`: see [closures](#11-closures)) | `structuredClone(x)` |
| Shared ownership (single thread) | `Rc<T>`                            | normal JS references        |
| Shared ownership (multi thread)  | `Arc<T>`                           | normal JS references        |
| Shared + mutable                 | `Rc<RefCell<T>>` / `Arc<Mutex<T>>` | normal JS objects           |

**The borrowing rule:** at any moment you can have _either_ many `&T` _or_ exactly one `&mut T`, never both. This eliminates data races and iterator invalidation at compile time.

```ts
function addTag(tags: string[], t: string): void {
  tags.push(t);
}

const tags: string[] = [];
addTag(tags, "rust");
```

```rust
fn add_tag(tags: &mut Vec<String>, t: &str) {
    tags.push(t.to_owned());
}

let mut tags = vec![];
add_tag(&mut tags, "rust");
```

> [!warning] Don't fight the borrow checker with JS habits
>
> - Avoid "object graph with back-references" designs (parent ↔ child). Use IDs/indices into a `Vec` or a `HashMap` instead.
> - `.clone()` is fine while learning. Optimize later.
> - If you reach for `Rc<RefCell<_>>` everywhere, you're writing JS in Rust — rethink the data flow.

### Shared state between tasks: `Arc<Mutex<T>>`

In JS one object is shared by every closure that holds it, and no lock is needed because there is one thread. In Rust the shared object is an `Arc` (shared ownership, counted) and the mutable part is inside a `Mutex`. The lock returns a **guard**; the lock is held while the guard lives and released when the guard is dropped (end of the block).

```ts
class Shared {
  private state = {
    phase: "notJoined" as Phase,                 // where the object is in its life
    lastPublished: offlineNotJoined as Status,   // the last status sent to the subscribers
    subscribers: [] as Subscriber[],
  };

  status(): Status {
    return this.publishChange();   // compute the status from the phase, send it if it is new
  }

  cancelJoin(): void {
    if (this.state.phase === "connecting") {
      this.state.phase = "notJoined";
      this.publishChange();
    }
  }
}
```

```rust
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

struct State {                                      // shortened: the real struct has more fields
    phase: Phase,                                   // NotJoined, Connecting, Holding(Place), ...
    last_published: Status,                         // the last status sent to the subscribers
    subscribers: Vec<mpsc::UnboundedSender<Status>>,
}

struct Shared {                                     // shortened
    clock: Arc<dyn Clock>,
    state: Mutex<State>,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        // A panic while the lock is held cannot leave the state half written (each change
        // is one assignment), so a poisoned lock is still safe to read. `status()` must
        // never panic.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn status(&self) -> Status {
        let now = self.clock.now();
        self.lock().publish_change(now)      // the guard is dropped at the end of this line
    }

    pub(super) fn cancel_join(&self) {
        let now = self.clock.now();
        let mut state = self.lock();
        if matches!(state.phase, Phase::Connecting) {
            state.phase = Phase::NotJoined;
            state.publish_change(now);
        }
    }                                       // lock released here
}
```

`State` has no `status` field and no `set_status`: the status is computed from the `phase` (and the clock) each time, and `last_published` remembers what the subscribers have seen, so only a new state or reason is sent. (This `State` is a private struct of `engine/state.rs`, not the `State` enum of [section 3](#string-unions--unit-enums).)

There is no `State::default()`: the struct has no `Default`, so `Shared::new` builds it field by field (`State { phase: Phase::NotJoined, last_published: not_joined, ... }`). The object is shared with `let shared = shared.clone();` (an `Arc` clone; see [closures](#11-closures)).

In test code the lock is shorter, because a poisoned lock there means a test already failed:

```rust
self.state.lock().expect("the in-memory database lock is poisoned")
```

```rust
let connection = state.connection.clone();
drop(state);                    // release the lock now, before the code that follows
self.leaving.send_replace(true);
```

`drop(guard)` ends the lock before the end of the block (≈ leaving a `using` block early). Use it when the next lines must not run with the lock held.

- Keep the guard short: never hold it across an `.await` (use `tokio::sync::Mutex` if you must).
- A single shared flag or counter does not need a `Mutex`: `Arc<AtomicBool>` / `AtomicU64` with `load(Ordering::SeqCst)` and `store(..)` (≈ a shared `boolean` that any closure may flip; `SeqCst` is the simple, always-correct ordering).

### Cleanup when a value goes away: `Drop`

JS has `Symbol.dispose` with `using` (TS 5.2) and finalizers; Rust has `Drop`, and it runs at a known time: at the end of the scope, on `?` and on an early `return`, when a panic unwinds, when a future that holds the value is dropped, and when the last `Arc` owner goes.

It does **not** run when the value is leaked: `std::mem::forget`, a cycle of `Rc` or `Arc` that never frees, `std::process::exit`, an abort (a panic with `panic = "abort"`, or a kill signal). Do not use `Drop` for something that must happen even then.

```ts
class CancelJoinOnDrop implements Disposable {
  constructor(private readonly shared: Shared) {}
  [Symbol.dispose]() {
    this.shared.cancelJoin();      // does nothing unless the phase is still "connecting"
  }
}

{
  using cancelOnDrop = new CancelJoinOnDrop(shared);
  await firstJoin();               // if this throws, dispose still runs
}                                  // disposed here: after a place, `cancelJoin` changes nothing
```

```rust
/// Puts the status back to `notJoined` when a `join` future is dropped before it finished
/// (rust-api, "Cancel a join by dropping it"). After a place, a failure or a `leave` it
/// does nothing.
struct CancelJoinOnDrop {
    shared: Arc<Shared>,
}

impl Drop for CancelJoinOnDrop {
    fn drop(&mut self) {
        self.shared.cancel_join();
    }
}

// in `Doyen::join` (shortened)
let shared = &self.handle.shared;
shared.start_join(&connection)?;
let _cancel_on_drop = CancelJoinOnDrop {
    shared: shared.clone(),
};
// ... the first join; if the caller drops the future here, `drop` runs
```

There is no "armed" flag. The guard always drops, and `cancel_join` looks at the phase (see `Shared` above): once `join` has a place the phase is `Holding`, so the call changes nothing. A TS `await` cannot be stopped halfway, but a Rust future can be dropped at any `.await` (for example by `tokio::time::timeout`), and `Drop` is what cleans up.

> [!warning] `let _guard = …` is not `let _ = …`
> A name that starts with `_` is a normal variable: the guard lives to the end of the block. A bare `_` is not a variable: the value is dropped on that line, and the guard does nothing.
>
> ```ts
> using _guard = new CancelJoinOnDrop(shared);   // lives to the end of the block
> ```
>
> ```rust
> let _cancel_on_drop = CancelJoinOnDrop { shared: shared.clone() };  // kept
> let _ = CancelJoinOnDrop { shared: shared.clone() };                // dropped at once
> let _ = mutex.lock();                                               // the same pitfall: unlocked at once
> ```

A `Drop` is also how a background task is stopped when the object is dropped: the struct holds the sending half of a channel, dropping it closes the channel, and the task sees it (see [Channels](#channels-the-event-emitter-of-rust)).

### Lifetimes

Lifetimes are annotations that tell the compiler how long borrows are valid. Mostly inferred; you write them when a function returns a reference derived from its inputs:

```ts
function longest(a: string, b: string): string {
  return a.length > b.length ? a : b;   // no question of "how long is the result valid"
}
```

```rust
fn longest<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() > b.len() { a } else { b }
}
```

Think of `'a` as a _generic parameter over scopes_. If a struct holds a reference, it needs one too (`struct Parser<'a> { input: &'a str }`). Early on, prefer owned fields (`String`) to avoid this.

### A struct that borrows: a named lifetime

```ts
interface SharedSettings {
  url: string;                 // a string is a value: no question of how long it lives
  group: string;
  timeoutMs?: number;
}
```

```rust
/// The settings every database shares, borrowed from a config struct.
struct SharedSettings<'config> {
    database: DatabaseKind,
    url: &'config str,
    group: &'config str,
    timeout_ms: Option<u64>,
    renew_interval_ms: Option<u64>,
    instance_name: Option<&'config str>,
}

impl RedisConfig {
    fn shared_settings(&self) -> SharedSettings<'_> {
        SharedSettings {
            database: DatabaseKind::Redis,
            url: &self.url,
            group: &self.group,
            // ...
            instance_name: self.instance_name.as_deref(),
        }
    }
}
```

`'config` is a name for "as long as the config that I borrow from"; the compiler refuses a `SharedSettings` that would outlive its config. `SharedSettings<'_>` in the `impl` means "the lifetime of `&self`", filled in by the compiler. Use a named lifetime only for a short-lived view like this one (the checks and the `Debug` output only read the text, so no copy is needed). A struct that is stored, or sent to a task, has owned `String` fields.

The lifetimes you will see in this repo:

- `'static`: "lives for the whole program". A string literal is `&'static str`; a trait bound `T: 'static` means "`T` holds no borrow", which every value sent to a background task must satisfy (the task may outlive the caller). It is like the rule that a callback passed to `setTimeout` must not use a variable the caller will free.
- `'_`: "the compiler fills it in" (`MutexGuard<'_, State>`, `Formatter<'_>`). Write it when the type asks for a lifetime and you do not care which.
- A named lifetime such as `'config` (above), only where the code needs it and a comment says why.

---

## 6. `null`/`undefined` → `Option<T>`

```ts
function find(id: number): User | undefined { ... }
const name = find(1)?.name ?? "anon";
```

```rust
fn find(id: u64) -> Option<User> { ... }

let name = find(1).map(|u| u.name).unwrap_or_else(|| "anon".to_owned());
```

| TS                        | Rust                                       |
| ------------------------- | ------------------------------------------ |
| `x?.y`                    | `x.map(\|v\| v.y)` / `x.as_ref().map(...)` |
| `x ?? d`                  | `x.unwrap_or(d)` / `unwrap_or_else(f)`     |
| `x!`                      | `x.unwrap()` / `x.expect("msg")` (panics)  |
| `if (x) {...}`            | `if let Some(v) = x {...}`                 |
| optional param `a?:`      | `a: Option<T>`                             |
| `x ?? throw err`          | `x.ok_or(err)?`                            |
| — (no equivalent)         | `x.as_deref()`: borrow the text inside an `Option<String>` as `Option<&str>` |
| `const v = x; x = undefined` | `let v = x.take();`                     |
| `x?.y?.z` (each step may be missing) | `x.and_then(\|v\| v.y)` (the closure returns an `Option`) |

Inside a function returning `Option`, `?` short-circuits on `None` — like `?.` but for the whole function.

More `Option` helpers that the code uses:

```ts
const highest = places.at(-1) ?? 0;                                // last item, or 0
const present = [viewA, viewB].filter((view) => view !== undefined); // drop the missing ones
```

```rust
let highest_place = self.places.keys().last().copied().unwrap_or(0);   // Option<&u64> -> Option<u64>
for view in [&view_a, &view_b].into_iter().flatten() { /* only the Some */ }
let fault = faults.get(&operation).cloned();                          // Option<&Fault> -> Option<Fault>
let reading = stream.next().now_or_never();                           // Option<Option<()>>
let ready = reading.flatten();                                        // Option<()>
```

- `.copied()` and `.cloned()` turn an `Option<&T>` (a borrow, as `map.get(k)` and `iter().last()` give) into an `Option<T>`: `.copied()` for a type that is `Copy` (a number), `.cloned()` for a type that is `Clone` (it makes a copy).
- `.flatten()` on an `Option<Option<T>>` gives an `Option<T>`; on an iterator of `Option<T>` it skips the `None` items.
- `.and_then(f)` is `.map(f)` when `f` itself returns an `Option`: no `Option<Option<_>>`.

```ts
// read a field and clear it in one step (a one-shot handle)
const stop = this.driverStop;
this.driverStop = undefined;
```

```rust
let stop: Option<oneshot::Receiver<()>> = state.driver_stop.take();
```

---

## 7. Exceptions → `Result<T, E>`

There is no `throw`/`try/catch` for recoverable errors. Errors are **values** in the return type.

```ts
async function load(path: string): Promise<Config> {
  const raw = await fs.readFile(path, "utf8"); // may throw
  return JSON.parse(raw); // may throw
}
```

```rust
use anyhow::{Context, Result};

fn load(path: &str) -> Result<Config> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading {path}"))?;
    let cfg = serde_json::from_str(&raw)?;
    Ok(cfg)
}
```

- `?` = "if `Err`, return it early (converting via `From`), else unwrap". This replaces 90% of try/catch.
- `panic!` exists but is for bugs/invariants, not control flow (like an uncaught `throw` that kills the thread).
- Error crates:
  - **`anyhow`** — applications. One `anyhow::Error` type, easy context. ≈ `throw new Error(msg)`.
  - **`thiserror`** — libraries. Typed error enums, so callers can `match` on them.

### A library error with a code (what doyen does)

```ts
class DoyenError extends Error {
  constructor(readonly code: ErrorCode, message: string) {
    super(`${code}: ${message}`);
  }
}

try {
  await doyen.join(config);
} catch (e) {
  if (e instanceof DoyenError && e.code === "DOYEN_INVALID_CONFIG") { ... }
}
```

```rust
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}: {message}")]
pub struct Error {
    code: ErrorCode,
    message: String,
}

impl Error {
    /// Make an error. Crate only: users get errors, they do not make them.
    pub(crate) fn new(code: ErrorCode, message: impl Into<String>) -> Error {
        Error {
            code,
            message: message.into(),
        }
    }

    pub fn code(&self) -> ErrorCode { self.code }

    pub fn message(&self) -> &str { &self.message }
}

match doyen.join(config).await {
    Ok(()) => {}
    Err(error) if error.code() == ErrorCode::InvalidConfig => { ... }
    Err(error) => return Err(error),
}
```

`#[error("...")]` writes the `Display` impl (what `.to_string()` gives); `#[from]` on a field writes the `From` impl that `?` uses to convert:

```rust
#[derive(Debug, thiserror::Error)]
enum ApiError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
}
```

### Change the error on the way: `map_err`, `ok_or`

```ts
function checkUrl(url: string): void {
  try { new URL(url); } catch { throw invalidConfig("url is not a valid URL"); }
}
const token = doyen.token() ?? throwLeft();
```

```rust
fn invalid_config(message: impl Into<String>) -> Error {          // a free function in config.rs
    Error::new(ErrorCode::InvalidConfig, message)
}

let Ok(parsed) = Url::parse(url) else {
    return Err(invalid_config("url is not a valid URL"));
};
// or with `map_err`: change the error type on the way
let parsed = Url::parse(url).map_err(|_| invalid_config("url is not a valid URL"))?;

let token = doyen.token().ok_or_else(left_error)?;               // `left_error()` is a free function in engine/state.rs
```

`ok_or(error)` builds the error even when it is not needed; `ok_or_else(f)` calls `f` only on `None`. A function name can be the closure (see [A function name as the closure](#a-function-name-as-the-closure)).

### Ignore an error on purpose

```ts
await promise.catch(() => {});        // the error cannot matter here
```

```rust
sender.send(status).ok();                              // `Err` means the reader is gone
let _ = self.leaving.wait_for(|leaving| *leaving).await;   // an error means the sender is gone
```

`.ok()` turns the `Result` into an `Option` that is dropped; `let _ = …;` drops the `Result` itself. This repo writes `.ok()` after a short call such as `send`, and `let _ = fut.await;` for a longer expression, with a comment that says why the error cannot matter (see `stop.rs`). Do not write `let _ =` for a guard (see [`let _guard`](#cleanup-when-a-value-goes-away-drop)).

---

## 8. Interfaces → traits

Traits are like interfaces, but implemented **separately** from the type and can be implemented for types you don't own (with the orphan rule: either the trait or the type must be yours).

```ts
interface Describe {
  describe(): string;
}
class Dog implements Describe {
  describe() {
    return "dog";
  }
}
```

```rust
trait Describe {
    fn describe(&self) -> String;
    fn shout(&self) -> String {            // default method
        self.describe().to_uppercase()
    }
}

struct Dog;
impl Describe for Dog {
    fn describe(&self) -> String { "dog".to_owned() }
}
```

Key standard traits (learn these early): `Debug`, `Display`, `Clone`, `Copy`, `PartialEq`/`Eq`, `Hash`, `Default`, `From`/`Into`, `TryFrom`, `Iterator`, `Deref`, `Drop`, `Send`/`Sync`.

### No classes, no inheritance

- Data → `struct`; behavior → `impl` blocks; polymorphism → traits.
- Constructors are just associated functions by convention: `User::new(...)`.
- Instead of `extends`, use composition + traits. Trait inheritance (`trait B: A`) only means "implementing B requires A".
- Private by default; `pub` to export (module-level, not class-level).

```ts
class User {
  id = 0;
  constructor(public name: string) {}
  rename(name: string): void {
    this.name = name;
  }
}
```

```rust
impl User {
    pub fn new(name: impl Into<String>) -> Self {
        Self { id: 0, name: name.into() }
    }
    pub fn rename(&mut self, name: &str) { self.name = name.to_owned(); }
}
```

`&self` is a method that reads; `&mut self` is a method that changes the object (the caller needs a `mut` binding); `self` consumes it (rare: builders, `into_*`).

### An interface with async methods: `#[async_trait]`

A TS interface can have `Promise` methods. A Rust trait can have `async fn` too, but then it cannot be used as a `dyn` object (a value typed by the interface). The `async-trait` crate fixes that with one attribute on the trait and on each `impl`.

```ts
interface Backend {                                  // shortened: join, renew, leave and watch are alike
  connect(): Promise<Warning[]>;
  join(group: string, id: string, tag: bigint, ttlMs: number): Promise<[number, View]>;
  watch(group: string): Promise<AsyncIterable<void> | undefined>;
}
```

```rust
use async_trait::async_trait;

#[async_trait]
pub(crate) trait Backend: Send + Sync + 'static {
    async fn connect(&self) -> Result<Vec<Warning>>;
    async fn join(&self, group: &str, id: &str, tag: u128, ttl: Duration) -> Result<(u64, View)>;
    async fn renew(&self, group: &str, me: &Membership, ttl: Duration) -> Result<Option<View>>;
    async fn leave(&self, group: &str, me: &Membership) -> Result<()>;
    async fn watch(&self, group: &str) -> Result<Option<BoxStream<'static, ()>>>;
}

#[async_trait]
impl Backend for MemoryBackend { ... }
```

Here `Result` is the alias of the `backend` module. `Membership` is the struct `{ number: u64, tag: u128 }` that the engine keeps for its place; `renew` and `leave` take it.

`Send + Sync + 'static` after the trait name: every implementation can be shared between tasks and holds no borrow. Without it the engine could not keep a `dyn Backend` inside a `tokio::spawn`.

### Async methods without `dyn`: a native `async fn` in a trait

The rule: a trait used as a `dyn` object needs `#[async_trait]`; a trait used only through generics can use a plain `async fn`, with no crate. The conformance suite is the second case. Its trait also has an **associated type**: a type that each implementation chooses.

```ts
interface TestDatabase<A extends Backend> {
  newAdapter(): Promise<A>;            // the type A is fixed by each implementation
  groupName(name: string): string;
}
```

```rust
pub(crate) trait TestDatabase {
    /// The adapter under test.
    type Adapter: Backend;

    /// A new adapter on this database: one more app instance.
    async fn new_adapter(&self) -> Self::Adapter;

    /// A group name for one test.
    fn group_name(&self, name: &str) -> String;
    // ... more hooks
}

impl TestDatabase for MemoryDatabase {
    type Adapter = MemoryBackend;

    async fn new_adapter(&self) -> MemoryBackend {
        self.backend()
    }
    // ...
}

pub(crate) async fn renew_after_expiry(database: &impl TestDatabase) { ... }   // generic, never `dyn`
```

`type Adapter: Backend;` says "each implementation names a type that is a `Backend`"; `Self::Adapter` is that type. The suite uses the trait only through a generic parameter (`&impl TestDatabase`), never as `dyn`, so the compiler knows the concrete type and a plain `async fn` is enough.

### A value typed by the interface: `dyn Trait`

```ts
class Doyen {
  private backend: Backend;             // any implementation
  constructor(backend: Backend, clock: Clock) { this.backend = backend; }
}
```

```rust
#[derive(Clone)]
pub(super) struct Connection {
    pub(super) backend: Arc<dyn Backend>,   // any implementation, shared, chosen at run time
    pub(super) settings: Arc<Validated>,
}

impl Doyen {
    /// An object on the adapter and the clock of a test, in place of the adapter of the
    /// config and the real clock.
    #[cfg(test)]
    pub(crate) fn with_backend(backend: Arc<dyn Backend>, clock: Arc<dyn Clock>) -> Doyen {
        Doyen::with_parts(clock, Some(backend))
    }
}
```

- `Arc<dyn Trait>` when it is shared (this repo's choice for `Backend` and `Clock`); `Box<dyn Trait>` when one owner holds it.
- `impl Trait` in a parameter (`fn run(backend: impl Backend)`) is the static form: one copy of the function per type, no pointer. Use `dyn` when the type is chosen at run time (the config says `redis` or `mongodb`).

### A callback stored in a field

```ts
type LivenessCheck = () => boolean;
let check: LivenessCheck | undefined;

function setLiveness(doyen: Doyen, check: LivenessCheck): void { ... }
```

```rust
pub(super) type LivenessCheck = Arc<dyn Fn() -> bool + Send + Sync>;
// in `State`:  liveness_check: Option<LivenessCheck>,

pub fn set_liveness(doyen: &Doyen, check: impl Fn() -> bool + Send + Sync + 'static) {
    doyen.handle.shared.set_liveness_check(Arc::new(check));
}
```

The parameter takes any closure (`impl Fn`); the field needs one fixed type, so the closure goes behind `Arc<dyn Fn>` (or `Box<dyn Fn>` when not shared).

---

## 9. Generics

Similar syntax, but constraints are **trait bounds** and generics are **monomorphized** (a copy per concrete type — zero runtime cost).

```ts
function maxBy<T>(xs: T[], key: (x: T) => number): T | undefined;
```

```rust
fn max_by<T, F>(xs: &[T], key: F) -> Option<&T>
where
    F: Fn(&T) -> i64,
{
    xs.iter().max_by_key(|x| key(x))
}
```

| TS                              | Rust                                                       |
| ------------------------------- | ---------------------------------------------------------- |
| `<T extends Foo>`               | `<T: Foo>` or `where T: Foo`                               |
| `T extends A & B`               | `T: A + B`                                                 |
| interface-typed param           | `impl Trait` (static) / `&dyn Trait` (dynamic)             |
| interface-typed return          | `-> impl Trait` (the caller sees only the interface)       |
| heterogeneous array `Foo[]`     | `Vec<Box<dyn Foo>>`                                        |
| conditional / mapped types      | ❌ no equivalent — use traits, associated types, or macros |
| `keyof`, template literal types | ❌ — use enums or macros                                   |

> [!note] Type-level programming
> Rust's type system is less "expressive-by-tricks" than TS's. Much of what you'd do with mapped/conditional types is done with **derive macros** (e.g. `serde`) or `macro_rules!`.

### A generic helper around a promise

```ts
async function callWithTimeout<T>(connection: Connection, call: Promise<T>): Promise<T> {
  return Promise.race([call, rejectAfter(connection.renewIntervalMs)]);
}
```

```rust
pub(super) async fn call_with_timeout<T>(
    connection: &Connection,
    call: impl Future<Output = backend::Result<T>>,
) -> backend::Result<T> {
    let timeout = connection.renew_interval();
    match tokio::time::timeout(timeout, call).await {
        Ok(answer) => answer,
        Err(_) => Err(BackendError::Network(format!(
            "the database did not answer within {} ms",
            timeout.as_millis()
        ))),
    }
}
```

`impl Future<Output = T>` is the parameter type of "a promise of `T`". A generic `<T>` stays when the caller must see the type again in the return.

### An explicit type argument: the turbofish `::<>`

```ts
const none = new Array<number>();      // f<T>(): the type argument written out
const names = items.map(String);
```

```rust
let empty = Vec::<u64>::new();                       // the type argument goes after `::`
let names = users.iter().map(|u| u.name.as_str()).collect::<Vec<_>>();   // `_`: the compiler works out the rest
let wait_forever = std::future::pending::<()>();
```

In a function call or a method call, `::<T>` is the same as `<T>` in TS; the `::` only keeps the parser from reading `<` as "less than". Write it when no `let x: Type` can give the type; `let names: Vec<&str> = …collect();` is the other way and is often easier to read.

---

## 10. Collections & iterators

| TS                  | Rust                                   |
| ------------------- | -------------------------------------- |
| `T[]`               | `Vec<T>` (owned), `&[T]` (slice view)  |
| `[A, B]` tuple      | `(A, B)`                               |
| `Map<K, V>`         | `HashMap<K, V>` / `BTreeMap` (ordered) |
| `Set<T>`            | `HashSet<T>` / `BTreeSet`              |
| `Record<string, T>` | `HashMap<String, T>`                   |

Iterators are **lazy** (like generator chains), and nothing happens until consumed with `collect`, `sum`, `for`, etc.

```ts
const names = users.filter((u) => u.active).map((u) => u.name);
```

```rust
let names: Vec<&str> = users
    .iter()                    // borrow items: &User
    .filter(|u| u.active)
    .map(|u| u.name.as_str())
    .collect();
```

- `.iter()` → `&T`, `.iter_mut()` → `&mut T`, `.into_iter()` → `T` (consumes the collection).
- `collect()` is driven by the target type: can build `Vec`, `HashMap`, `String`, even `Result<Vec<_>, E>` (fails on first error — like `Promise.all`).
- Useful: `enumerate`, `zip`, `flat_map`, `filter_map`, `take_while`, `fold`, `any`, `all`, `find`, `position`, `chunks`, `windows`. Crate `itertools` for more.

### Change a list in place

```ts
subscribers = subscribers.filter((s) => s.send(status));   // drop the closed ones
members.sort((a, b) => a.number - b.number);
```

```rust
subscribers.retain(|s| s.send(status.clone()).is_ok());    // same list, in place
members.sort_by_key(|m| m.number);
```

### Get or create a value in a map: `entry`

```ts
let data = groups.get(group);
if (data === undefined) {
  data = new GroupData();
  groups.set(group, data);
}
```

```rust
let data = state
    .groups
    .entry(group.to_owned())          // the key is moved in, so copy a `&str` first
    .or_insert_with(GroupData::new);  // runs `GroupData::new` only when the key is missing

state.watchers.entry(group.to_owned()).or_default().push(sender);   // `Default::default()` (an empty `Vec`)
```

Both give a `&mut` to the value in the map. `or_insert(value)` builds the value even when the key exists; prefer `or_insert_with`.

### A tuple as a return value

```ts
function channel(): [Sender, Receiver] { ... }
const [sender, receiver] = channel();
```

```rust
let (sender, receiver) = mpsc::unbounded_channel();
let pair: (u64, String) = (1, "a".to_owned());
pair.0;                      // tuple fields are numbered
```

---

## 11. Closures

```ts
const factor = 3;
const mul = (x: number) => x * factor;      // captures `factor` by reference
setTimeout(() => console.log(factor), 10);  // the closure keeps `factor` alive
```

```rust
let factor = 3;
let mul = |x: i32| x * factor;      // borrows `factor`
let owned = move |x: i32| x * factor; // takes ownership (needed for threads/async)
```

Three closure traits, chosen automatically by what the closure does to captured values:

- `Fn` — only reads captures (callable many times, concurrently)
- `FnMut` — mutates captures
- `FnOnce` — consumes captures (callable once)

When accepting callbacks, ask for the least restrictive one you need (`FnOnce` > `FnMut` > `Fn` in terms of what callers can pass).

Rule for this repo: an `async move` block passed to `tokio::spawn` (and a closure kept in a struct) is `move`, and the values it needs are cloned before it, on the line above: `let shared = shared.clone();`. On an `Arc`, `.clone()` copies the pointer and counts one more owner, not the data. (`Arc::clone(&shared)` does the same; this repo writes the short form.) A closure inside a closure is a sign to name a function.

### A function name as the closure

```ts
const names = items.map(String);                     // point-free: no `(x) => String(x)`
```

```rust
self.fields.get(name).map(String::as_str)            // the same as `.map(|text| text.as_str())`
notifications.iter().map(reason_of).collect()        // `reason_of` is a plain function
let token = doyen.token().ok_or_else(left_error)?;  // `left_error` is called only on `None`
```

If the function takes the same argument that the closure would pass on, write the function path. Keep the closure when you need to change or add an argument.

---

## 12. Mutability

Everything is immutable by default — `let` in Rust ≈ `const` in TS, but **deep**.

```ts
const v = [1, 2];
v.push(3);                 // allowed: `const` only freezes the binding
const frozen = Object.freeze([1, 2]);
frozen.push(3);            // TypeError at run time
```

```rust
let v = vec![1, 2];
// v.push(3);       // compile error
let mut v = vec![1, 2];
v.push(3);           // ok
```

There's no `readonly` vs. mutable object split: mutability is a property of the **binding/borrow**, not the type.

---

## 13. Async

| TS / Node           | Rust                                                            |
| ------------------- | --------------------------------------------------------------- |
| built-in event loop | pick a runtime: **`tokio`** (de facto)                          |
| `Promise<T>`        | `impl Future<Output = T>`                                       |
| Promises are eager  | Futures are **lazy** — nothing runs until `.await`ed or spawned |
| `Promise.all`       | `tokio::join!` / `futures::future::join_all`                    |
| `Promise.race`      | `tokio::select!`                                                |
| fire-and-forget     | `tokio::spawn(async move { ... })`                              |
| `setTimeout`        | `tokio::time::sleep(dur).await`                                 |
| `setInterval`       | `tokio::time::interval_at(Instant::now() + dur, dur)` + `.tick().await` (plain `interval` ticks at once) |
| `setTimeout` until a date | `tokio::time::sleep_until(instant).await`                 |
| `new Promise(() => {})` (never settles) | `std::future::pending().await`                  |
| `AbortController`   | drop the future / a `watch` channel (`CancellationToken` is in `tokio-util`, not a dependency here) |
| `AsyncIterable<T>`  | `impl Stream<Item = T>`                                         |
| `EventEmitter`      | a channel (`tokio::sync::mpsc`)                                 |
| single-threaded     | multi-threaded by default → `Send + 'static` bounds             |

```ts
async function main(): Promise<void> {
  const [a, b] = await Promise.all([fetch("a"), fetch("b")]);
  console.log(a, b);
}
```

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (a, b) = tokio::join!(fetch("a"), fetch("b"));
    println!("{} {}", a?, b?);
    Ok(())
}

async fn fetch(url: &str) -> anyhow::Result<String> {
    Ok(reqwest::get(url).await?.text().await?)
}
```

> [!warning]
> Never block inside async code (`std::thread::sleep`, heavy CPU, sync I/O). Use `tokio::task::spawn_blocking` — same idea as not blocking the Node event loop.

Shared state across tasks: `Arc<Mutex<T>>` (`tokio::sync::Mutex` if held across `.await`), or channels (`tokio::sync::mpsc`) — often cleaner.

### A background task: `tokio::spawn`

```ts
// fire and forget; errors must be caught inside
void (async () => {
  await renewLoop(shared);
})();
```

```rust
let shared = shared.clone();            // one more owner of the `Arc`, for the task
tokio::spawn(async move {
    renew_loop(shared).await;
});
```

`spawn` takes a future (here an `async move` block), not a closure. The task may run on another thread, so everything it captures must be `Send + 'static` (owned or `Arc`, no `Rc`, no borrow). The `JoinHandle` that `spawn` returns can be awaited like a promise; dropping it does **not** cancel the task (a dropped future does; see [Stop a task](#stop-a-task-a-dropped-sender)).

### Channels: the `EventEmitter` of Rust

A channel is a typed queue with a sending half and a receiving half. It replaces `EventEmitter`, callbacks and most shared flags.

| Need                               | TS                                   | tokio                                   |
| ---------------------------------- | ------------------------------------ | --------------------------------------- |
| many events, one reader            | `EventEmitter` / async iterator      | `mpsc::unbounded_channel()`             |
| one answer, once                   | `new Promise((resolve) => ...)`      | `oneshot::channel()`                    |
| the latest value, many readers     | a "signal" / `BehaviorSubject`       | `watch::channel(initial)`               |
| a stop signal                      | `AbortController`                    | `watch<bool>` or a dropped `oneshot`    |

```ts
// subscribe to status changes
const emitter = new EventEmitter();
emitter.on("status", (status) => console.log(status));
emitter.emit("status", { state: "master" });

// one answer, once
const { promise, resolve } = Promise.withResolvers<void>();
resolve();
await promise;
```

```rust
use tokio::sync::{mpsc, oneshot, watch};

// many events, one reader
let (sender, mut receiver) = mpsc::unbounded_channel::<Status>();
sender.send(Status::Master).ok();     // `Err` means the reader is gone: ignore it
while let Some(status) = receiver.recv().await {
    println!("{status:?}");
}                                        // ends when every sender is dropped

// one answer, once
let (done, wait) = oneshot::channel::<()>();
done.send(()).ok();
wait.await.ok();

// the latest value
let (leaving, mut is_leaving) = watch::channel(false);
leaving.send(true).ok();
is_leaving.changed().await.ok();        // wakes when the value changes
assert!(*is_leaving.borrow());
```

- `unbounded` means the queue never blocks the sender (≈ `emit`); a bounded channel (`mpsc::channel(n)`) gives back-pressure.
- `send` returns `Err` when the other half is dropped. For a status that nobody reads any more that is fine: ignore it, or `retain` the live senders.

### The `watch` channel in detail

A `watch` holds one value; a receiver sees the latest one. It is a `BehaviorSubject` (`.value` + `subscribe`). The code uses more than `channel`, `send`, `changed` and `borrow`:

```ts
const leaving = new BehaviorSubject(false);
const stop = leaving.pipe(first((value) => value));   // resolves now if already true
leaving.next(true);
const faults = new BehaviorSubject(new Map<Operation, Fault>());
faults.next(withoutFaults(faults.value));            // change the held value
```

```rust
// `faults` is a `watch::Sender<HashMap<Operation, Fault>>`
let leaving: watch::Sender<bool> = watch::Sender::new(false);   // no receiver yet
let mut is_leaving = leaving.subscribe();                       // a receiver, when you need one

leaving.send_replace(true);                 // always stores the value; `send` fails and stores nothing
                                            // when no receiver exists
faults.send_modify(|faults| faults.clear());                    // change the value in place

let _ = is_leaving.wait_for(|leaving| *leaving).await;          // returns at once if it is already true
let fault = faults.borrow_and_update().get(&operation).cloned(); // read, and mark this value as seen
let _ = faults.changed().await;                                  // wait for a value newer than the last seen
```

- `wait_for(predicate)` checks the current value first, then waits for each change. Use it to wait for a state ("`leaving` is true").
- `changed()` waits only for a value **newer** than the one this receiver has seen. If the value was set before you started waiting, it does not wake. Use it to wait for "the next change", and call `borrow_and_update()` before it so the value you read counts as seen.
- Both return `Err` when the sender is dropped.
- `send_replace` and `send_modify` work with no receiver; they are the choice for a flag that a receiver created later must still read as set.

### Stop a task: a dropped sender

In TS the object keeps an `AbortController` and calls `abort()` in `close()`. In Rust the object keeps a sender; when the object is dropped the sender is dropped, the receiver sees "closed", and the task ends. No `close()` to forget.

```ts
class Doyen {
  private readonly stop = new AbortController();
  async join(config: Config): Promise<void> {
    await firstJoin(config);
    runDriver(this.stop.signal);          // starts here, not in the constructor
  }                                       // the loop checks `signal.aborted`
  [Symbol.dispose]() {
    this.stop.abort();
  }
}
```

```rust
#[derive(Clone)]
pub struct Doyen {
    handle: Arc<Handle>,
}

/// The part that the clones share. The driver never holds it, so the drop of the last
/// clone drops the stop sender, and the driver stops at once.
struct Handle {
    shared: Arc<Shared>,
    _stop_driver_on_drop: oneshot::Sender<()>,   // never used: dropping it is the signal
}

// in `Doyen::with_parts`: the receiver goes into `Shared` (`driver_stop`)
let (stop_sender, stop_receiver) = oneshot::channel();
let shared = Shared::new(clock, given_backend, stop_receiver);

// in `Doyen::join`, after the first place (shortened)
if let Some(last_handle_dropped) = shared.take_driver_stop() {
    let signals = StopSignals::on_leave_or_drop(shared.leaving(), last_handle_dropped);
    Driver::start(shared.clone(), connection, signals, wake_ups);   // it calls `tokio::spawn`
}
```

The driver starts in `join`, not in `Doyen::new()`. `tokio::spawn` panics when there is no Tokio runtime, and the spec (rust-api, "No Tokio runtime") says that `Doyen::new()` must work there. `join` checks for a runtime first and fails with `DOYEN_UNSUPPORTED` when there is none. `Doyen` holds an `Arc<Handle>`, so every clone shares one stop sender; the last clone to drop closes the channel. Clones of `Shared` that the driver holds do not count, because the sender is in `Handle`.

The `_` prefix on the field says "kept only for its drop". A field name that starts with `_` is the convention; `#[expect(dead_code)]` is not needed.

### Wait for the first of several: `tokio::select!`

`Promise.race` returns the first settled promise. `select!` does the same with named branches, and **cancels the other branches** (their futures are dropped).

```ts
const result = await Promise.race([
  work.then((value) => ({ kind: "done", value })),
  stopped.then(() => ({ kind: "stopped" })),
]);
if (result.kind === "stopped") return undefined;
```

```rust
impl StopSignals {
    /// Run `work` until it ends or a signal comes. `None` means "stop now": the work was
    /// dropped (every adapter call is safe to cancel).
    pub(super) async fn unless_stopped<T>(&mut self, work: impl Future<Output = T>) -> Option<T> {
        tokio::select! {
            biased;
            () = self.stopped() => None,
            output = work => Some(output),
        }
    }

    /// Wait until a signal comes. Safe to cancel.   (shortened: the real one also waits for
    /// the last-handle oneshot inside a second `select!`)
    pub(super) async fn stopped(&mut self) {
        // An error means the sender is gone, so nothing can need this object any more.
        let _ = self.leaving.wait_for(|leaving| *leaving).await;
    }
}
```

The `biased;` first line makes the branches checked in order (the first ready wins) instead of at random. The stop branch is first, so when `leave` and the end of the work are ready together, the stop wins.

`stopped()` uses `wait_for(|leaving| *leaving)`, not `changed()`: `changed()` waits only for a value newer than the last one seen, so it would sleep forever if `leave` was called before the wait began, and it returns `Err` at once when the sender is dropped (a `_ =` pattern would then hide it and stop the work for the wrong reason). See [the watch channel](#the-watch-channel-in-detail).

The end of the lease is not a stop signal. The driver waits for it in its own `select!` branch (`Event::LeaseEnded`, with `sleep_until`; see below).

### Keep waiting for the loser: `tokio::pin!` and `&mut work`

`select!` drops the futures of the branches that lose. When the work must go on after the other branch won, race a **borrow** of it, so only the borrow is dropped.

```ts
const work = doWork();                                   // a promise: already running
const first = await Promise.race([work, leaseEnded()]);
if (first === LEASE_ENDED) {
  publishStatus();
  return await work;                                     // the same work, still running
}
```

```rust
async fn watch_the_lease<T>(
    shared: &Shared,
    signals: &mut StopSignals,
    work: impl Future<Output = T>,
) -> Option<T> {
    let lease_deadline = shared.lease_deadline();
    let work = signals.unless_stopped(work);
    // Pinned so that `&mut work` can be polled inside `select!` and awaited again after it.
    tokio::pin!(work);
    tokio::select! {
        output = &mut work => return output,
        () = sleep_until_if_set(lease_deadline) => shared.publish_status(),
    }
    work.await
}
```

A future must not move once it has started; `tokio::pin!` fixes it in place on the stack, and then `&mut work` can be polled many times. (A TS promise is already running and shared, so this problem does not exist.) Without the `&mut`, the first `select!` would take `work` and drop it.

### Wait for a time that may not exist: `sleep_until`, `pending`

```ts
async function sleepUntilIfSet(deadline?: number): Promise<void> {
  if (deadline === undefined) {
    return new Promise(() => {});          // never settles: this race branch never wins
  }
  await sleep(deadline - now());
}
```

```rust
async fn sleep_until_if_set(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,   // sleep(duration) is "after"; sleep_until is "at"
        None => std::future::pending().await,                        // never ready
    }
}
```

`std::future::pending()` is a future that never completes (≈ `new Promise(() => {})`). It keeps a `select!` branch alive without letting it win. When the compiler cannot infer the output type, write `pending::<()>()`.

### A ticking loop: `interval_at` with missed ticks

```ts
// waits one period before the first call; it does not wait for an async `renew` to end
const timer = setInterval(renew, renewIntervalMs);
```

```rust
let interval = connection.renew_interval();
let mut schedule = tokio::time::interval_at(Instant::now() + interval, interval);
// A tick missed after a suspend or a slow call does not send a burst of renews.
schedule.set_missed_tick_behavior(MissedTickBehavior::Skip);
loop {
    schedule.tick().await;
    renew().await;
}
```

- `tokio::time::interval(period)` completes its **first** tick at once. `setInterval` waits one period first. `interval_at(start, period)` takes the start of the first tick, so `Instant::now() + interval` gives the `setInterval` behaviour.
- `setInterval` does not wait for an async callback: two calls of a slow `renew` can overlap. In the Rust loop `renew().await` ends before the next `tick()`, so calls never overlap.
- When a tick is late (a slow call, a suspended machine), the default of tokio is `Burst`: it ticks many times to catch up. `Skip` runs once now and goes on with the normal schedule.

`Skip` is what you want for a renew: after the process was paused, run once now, not ten times.

### A stream: `AsyncIterable` → `Stream`

```ts
async function* statusChanges(): AsyncIterable<Status> { ... }

for await (const status of doyen.statusChanges()) {
  console.log(status);
}
```

```rust
use futures_util::StreamExt;              // brings `.next()` and the adapters

pub fn status_changes(&self) -> impl Stream<Item = Status> + Send + Unpin + 'static {
    let mut changes = self.handle.shared.subscribe();    // an mpsc receiver
    // Turn the receiver into a stream: `poll_fn` asks the receiver for the next value
    // each time the reader awaits (the `context` is the plumbing of `await`).
    futures_util::stream::poll_fn(move |context| changes.poll_recv(context))
}

let mut changes = doyen.status_changes();
while let Some(status) = changes.next().await {
    println!("{status:?}");
}
```

- `impl Stream<Item = T>` in a return type: the caller gets "an async iterable of `T`" and nothing more.
- `Unpin` in the bound lets the caller call `.next()` on a plain `let mut`; without it they would need `tokio::pin!(stream)` first.
- `BoxStream<'static, T>` (= `Pin<Box<dyn Stream<Item = T> + Send>>`) is the `dyn` form, for a trait method whose return type must be one fixed type (the `Backend::watch` above). `BoxFuture` is the same for a future. Build one with `Box::pin(async move { ... })` or `.boxed()`. Treat `Pin` as part of the type name; in the code of the crate you never call its methods. The one exception is the test `block_on` in [Testing](#run-a-future-without-tokio-a-hand-written-block_on).

### Read what is ready now: `now_or_never`

```ts
// no equivalent for a promise; the closest is a race against an immediate value
const NOT_READY = Symbol();
const item = await Promise.race([next(), Promise.resolve(NOT_READY)]);
```

```rust
use futures_util::FutureExt;               // brings `now_or_never`

// drop the wake-ups that are waiting now, without waiting for new ones
while let Some(Some(())) = stream.next().now_or_never() {}
```

`now_or_never()` polls the future once. It gives `Some(output)` if the future is ready and `None` if it would have to wait. `stream.next()` itself gives an `Option` (`None` when the stream ended), so the result is an `Option<Option<T>>`: the outer one says "ready?", the inner one says "an item?". The tests use it to read all the statuses that have arrived, without waiting for more.

### Fake time in tests

```ts
vi.useFakeTimers();
const joined = doyen.join(config);          // starts, and waits on a timer
await vi.advanceTimersByTimeAsync(2_000);   // every sleep/interval inside moves 2 s
await joined;
```

```rust
#[tokio::test(start_paused = true)]         // a current-thread runtime with the clock paused
async fn renews_after_the_interval() {
    doyen.join(config).await.expect("join works");
    // The driver runs in a spawned task. While the test sleeps, the paused clock jumps to
    // the next timer, so the driver renews as if 5 s had passed.
    sleep(Duration::from_secs(5)).await;
    assert!(adapter.call_count(Operation::Renew) >= 2);
}
```

- `#[tokio::test(start_paused = true)]` is what the engine tests use. It needs the `test-util` feature of tokio (a dev-dependency here). The memory and conformance tests call `tokio::time::pause()` at the start of their start function instead, because the macro `conformance_tests!` writes a plain `#[tokio::test]`. `pause()` panics on a runtime that is not `current_thread`.
- A future does nothing until it is awaited or spawned. `let joined = doyen.join(config);` and then `tokio::time::advance(…)` moves the time, but nothing waits on it. Await it in the test body, or `tokio::spawn` it (clone what it needs first), or put it in `tokio::join!` together with the `advance`.
- `tokio::time::advance(d).await` moves the clock by `d` and lets the tasks that became ready run. With time paused, `sleep`, `interval` and `timeout` inside tokio run instantly when every task is idle (auto-advance).
- `std::thread::sleep` is real time: never use it in an async test.

---

## 14. Modules & visibility

```
src/
├── main.rs        // binary entry (or lib.rs for a library)
├── config.rs      // mod config;
└── api/
    ├── mod.rs     // mod api;  (or api.rs next to the api/ folder)
    └── routes.rs
```

```ts
// index.ts
import { handler } from "./api/routes";
import { Config } from "./config";
export { Router } from "./api/routes";    // re-export
```

```rust
// main.rs
mod config;
mod api;
use api::routes::handler;          // like `import { handler } from "./api/routes"`
use crate::config::Config;         // `crate::` = project root
pub use api::routes::Router;       // re-export (like `export { X } from`)
```

- Modules must be **declared** with `mod` — files aren't auto-discovered. A folder module can be `engine.rs` + `engine/driver.rs` (no `mod.rs`); the file `engine.rs` declares `mod driver;`.
- Items are private by default.

### Visibility levels

| TS                                             | Rust            | Who sees it                      |
| ---------------------------------------------- | --------------- | -------------------------------- |
| not exported                                   | (nothing)       | this module and its children     |
| exported from the file, not from the package   | `pub(crate)`    | the whole crate, not its users   |
| exported only to the parent folder's index     | `pub(super)`    | the parent module                |
| exported from the package (`index.ts`)         | `pub`           | everyone                         |
| exported but `@internal` / not in the typings  | `#[doc(hidden)] pub` | everyone, not in the docs   |

`pub(crate)` is the usual choice for a trait or a helper that the adapters share but the API does not show (`Backend`, `Clock`). `#[doc(hidden)]` is for the `__binding` hooks: a binding needs them, a user never should.

### Compile-time switches: `#[cfg]`

```ts
// TS has only run-time checks
if (process.platform === "linux") { ... }
if (process.env.NODE_ENV === "test") { ... }
```

```rust
// clock.rs: the clock that counts the time of a suspend
#[cfg(target_os = "linux")]
const SUSPEND_AWARE_CLOCK: rustix::time::ClockId = rustix::time::ClockId::Boottime;
#[cfg(target_os = "macos")]
const SUSPEND_AWARE_CLOCK: rustix::time::ClockId = rustix::time::ClockId::Monotonic;

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        let reading = rustix::time::clock_gettime(SUSPEND_AWARE_CLOCK);
        // A reading is never negative; 0 is only a fallback that cannot happen.
        let seconds = u64::try_from(reading.tv_sec).unwrap_or(0);
        let nanoseconds = u32::try_from(reading.tv_nsec).unwrap_or(0);
        Duration::new(seconds, nanoseconds)
    }
}

// A build error with your text, on every platform that is not supported
#[cfg(not(any(
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ),
    all(target_os = "macos", target_arch = "aarch64"),
)))]
compile_error!(
    "doyen supports only Linux (x86_64 and aarch64) and macOS (aarch64): \
     it needs a monotonic clock that counts the time of a suspend"
);

#[cfg(test)]
mod tests;                           // compiled only by `cargo test`

#[cfg(test)]
pub(crate) struct FakeClock { ... }  // a test double that the normal build never sees
```

The reading of `rustix` is a `Timespec` with signed `tv_sec` and `tv_nsec`. It cannot become a `Duration` with `.into()` (a signed number may be negative). Convert each part with `try_from` (checked, see `TryFrom` in [section 2](#2-types-structural--nominal)), as `SystemClock::now` does.

`#[cfg(...)]` removes the item from the build; there is no run-time cost and no "unreachable branch" to test. The feature switches of [section 1](#optional-dependencies--cargo-features) are the same mechanism (and `cfg!(…)` is its expression form).

### Lints: `#[expect]`, `#[allow]`, `#![deny]`

```ts
// eslint-disable-next-line @typescript-eslint/no-unused-vars -- the Redis and MongoDB tests call it
function databaseUrl(variable: string): string | undefined { ... }
```

```rust
#[expect(
    dead_code,
    reason = "the test start functions of the Redis and MongoDB adapters call it"
)]
pub(crate) fn database_url(variable: &str) -> Option<String> { ... }

// backend.rs, at the top of the file: for the whole module, only when not testing
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "the adapters of later changes use these items")
)]
```

- `#[expect(lint)]` is a disable that **fails when it is no longer needed**: when the function gets a user, the compiler gives the warning `unfulfilled_lint_expectations` and says to remove the attribute. CI runs with `-D warnings`, which turns that warning into an error. Prefer `#[expect]` to `#[allow]`; keep `allow` only where `expect` cannot work (a lint that is raised in some builds and not in others).
- `#![cfg_attr(not(test), expect(...))]` means "when not testing, add this attribute". The `#![...]` form (with `!`) applies to the whole file or module. Here the adapters use the items only in later changes and the tests use them now, so the lint is raised in the normal build only, and `expect` is fulfilled in every build that has the attribute.
- `#![deny(missing_docs)]` at the top of `lib.rs` turns a missing doc comment on a public item into an error (like a lint set to `error` in `eslint.config`).

### Doc comments are the typings' JSDoc

```ts
/**
 * Joins the group. Resolves when the first try is over.
 * @throws DoyenError with `DOYEN_INVALID_CONFIG` when the config is wrong.
 */
async join(config: Config): Promise<void>
```

```rust
/// Join the group of the config: take a place at the end of the line.
///
/// It finishes when the instance has a place (the status is then master or slave). It
/// retries every network error meanwhile. It fails only on an error a retry cannot fix:
/// `DOYEN_INVALID_CONFIG`, `DOYEN_AUTH`, `DOYEN_UNSUPPORTED` (also outside a Tokio
/// runtime), `DOYEN_ALREADY_JOINED` (a second `join`) and `DOYEN_LEFT`.   (shortened)
pub async fn join(&self, config: DatabaseConfig) -> Result<(), Error>
```

The `# Errors` heading is the usual place for the errors in Rust docs (`/// # Errors`, then the list), but it is optional. This crate lists the errors in the text, as `Doyen::join` does above.

`//!` at the top of a file documents the module itself. A code block inside `///` is compiled and run by `cargo test` (a doc-test): the example cannot go stale.

---

## 15. Ecosystem equivalents

| Need                | Node / TS                 | Rust                                              |
| ------------------- | ------------------------- | ------------------------------------------------- |
| Validation / schema | zod                       | `serde` + `validator` / `garde`                   |
| JSON                | `JSON.parse`              | `serde_json`                                      |
| HTTP server         | Express / Fastify / Hono  | `axum` (tokio), `actix-web`                       |
| HTTP client         | fetch / axios             | `reqwest`                                         |
| ORM / SQL           | Prisma / Drizzle / Kysely | `sqlx` (compile-checked SQL), `diesel`, `sea-orm` |
| CLI                 | commander / yargs         | `clap` (derive)                                   |
| Env/config          | dotenv                    | `dotenvy`, `config`, `figment`                    |
| Logging             | pino / winston            | `tracing` + `tracing-subscriber`                  |
| Dates               | date-fns / Temporal       | `chrono`, `jiff`, `time`                          |
| UUID                | uuid                      | `uuid`                                            |
| URL parsing         | `new URL(s)`              | `url::Url::parse(s)`                              |
| Random              | `Math.random()`           | `rand`                                            |
| Host name           | `os.hostname()`           | `gethostname`                                     |
| Low-level OS calls  | — (Node hides them)       | `rustix` (safe wrappers over libc)                |
| Testing mocks       | vi.mock                   | `mockall` (or trait-based DI)                     |
| Desktop app         | Electron                  | Tauri                                             |
| Frontend in Rust    | React                     | Leptos, Dioxus, Yew                               |
| Calling from Node   | —                         | `napi-rs` (native addon), `wasm-bindgen` (WASM)   |

`serde` is the zod-equivalent cornerstone:

```ts
const CreateUser = z.object({
  userName: z.string(),
  isAdmin: z.boolean().default(false),
  email: z.string().optional(),
});
const input = CreateUser.parse(JSON.parse(body)); // parse = validate
```

```rust
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateUser {
    user_name: String,
    #[serde(default)]
    is_admin: bool,
    email: Option<String>,
}

let input: CreateUser = serde_json::from_str(body)?; // parse = validate
```

### Structured logs: `tracing`

```ts
const log = pino();
log.info({ group, state, reason, number, generation }, "the status changed");
log.debug({ group, call, error: message }, "a database call failed");
```

```rust
tracing::info!(
    target: "doyen",
    group = self.group.as_str(),
    state = state_name(status),
    reason,                     // Option<String>: a field with `None` is left out
    number,                     // Option<u64>
    generation,                 // Option<u64>
    "the status changed"
);

tracing::debug!(
    target: "doyen",
    group,
    call,
    error = message,
    "a database call failed"
);
```

The fields come first, the message last. `name = value` sets a field; a bare `group` uses the variable of that name. `target: "doyen"` sets the target to one fixed name (the default is the module path), so a user filters all the events of the library with `RUST_LOG=doyen=info`. A field that is an `Option` is recorded only when it is `Some`. A library only emits; the app installs the subscriber (`tracing-subscriber`), like the app picks the pino transport. Never put a URL with a password in a field.

---

## 16. Testing

Tests live **in the same file** as the code (unit) or in `tests/` (integration). This repo keeps them in a sibling file (`config.rs` + `config/tests.rs`, declared with `#[cfg(test)] mod tests;`), so the code file stays short.

```ts
// add.test.ts
import { describe, it, expect } from "vitest";
import { add } from "./add";

describe("add", () => {
  it("adds", () => {
    expect(add(2, 2)).toBe(4);
  });

  it("works async", async () => { /* ... */ });

  it("throws on bad input", () => {
    expect(() => parse("")).toThrow();
  });
});
```

```rust
pub fn add(a: i32, b: i32) -> i32 { a + b }

#[cfg(test)]
mod tests {
    use super::*;          // the private items of the file are visible here

    #[test]
    fn adds() {
        assert_eq!(add(2, 2), 4);
    }

    #[tokio::test]
    async fn works_async() { /* ... */ }

    #[test]
    #[should_panic]
    fn panics_on_bad_input() {
        parse("");
    }
}
```

- `assert!(cond, "{message}")`, `assert_eq!(a, b)`, `assert_ne!(a, b)` ≈ `expect(...)`. The extra arguments are a format string shown on failure.
- There is no `describe`: group with modules (`mod join { ... }`) or with the file name. A test name is a sentence in snake_case: `a_group_with_braces_is_invalid`.
- `cargo test name_part` runs the tests whose path contains it (≈ `vitest -t`).
- `#[should_panic]` is `expect(fn).toThrow()` for a panic. For a `Result`, assert on the `Err` instead.
- Doc comments with code blocks (` ``` `) are also compiled and run as tests (`cargo test --doc`).

### The same tests for every implementation: a test macro

In TS you write a function that takes the factory and registers the `it` blocks, and call it from each adapter's test file. In Rust a test must be a plain function with `#[test]` at the top level of a module, so the "function that registers tests" is a `macro_rules!` that writes those functions. Use it only for this case (a conformance suite); everywhere else, write a plain helper function.

```ts
export function backendConformance(
  name: string,
  start: () => Promise<TestDatabase | undefined>,   // undefined: no database here, skip
  options: { everyRuleFails?: boolean } = {},
) {
  describe(`${name} backend`, () => {
    for (const rule of RULES) {                       // one test per rule of the suite
      it(rule.name, async () => {
        const database = await start();
        if (database === undefined) return;
        if (options.everyRuleFails) {
          await expect(rule(database)).rejects.toThrow();   // an adapter broken on purpose
        } else {
          await rule(database);
        }
      });
    }
  });
}

// memory.test.ts
backendConformance("memory", startMemoryDatabase);
```

```rust
// conformance.rs (shortened: the real list has 17 rules)
macro_rules! conformance_tests {
    // One test: `$extra` holds the attributes to add, for example `#[should_panic]`.
    (@test $start:path, [$($extra:tt)*], $rule:ident) => {
        #[tokio::test]
        $($extra)*
        async fn $rule() {
            let Some(database) = $start().await else {
                return;
            };
            $crate::conformance::$rule(&database).await;
        }
    };
    (@tests $start:path, $extra:tt) => {
        $crate::conformance::conformance_tests!(
            @each_rule $start,
            $extra,
            adapter_decides_nothing,
            number_given_twice,
            // ... the other rules
            newer_format_while_running,
        );
    };
    (@each_rule $start:path, $extra:tt, $($rule:ident,)*) => {
        $(
            $crate::conformance::conformance_tests!(@test $start, $extra, $rule);
        )*
    };
    ($start:path, every_rule_fails) => {
        $crate::conformance::conformance_tests!(@tests $start, [#[should_panic]]);
    };
    ($start:path) => {
        $crate::conformance::conformance_tests!(@tests $start, []);
    };
}
pub(crate) use conformance_tests;

// memory/tests.rs
async fn start_memory_database() -> Option<MemoryDatabase> {
    tokio::time::pause();
    Some(MemoryDatabase::new())
}

crate::conformance::conformance_tests!(start_memory_database);
```

How to read it:

- A `macro_rules!` is a list of **arms**, tried from the top like the cases of a `switch`. The first arm whose pattern fits the call is used.
- `$start:path` is a parameter that must be a path (the name of a function, here `start_memory_database`); `$start().await` calls it. `$rule:ident` is a name. `$extra:tt` is any one token tree; `$($extra:tt)*` is zero or more of them (`*` repeats, like a rest parameter `...extra`).
- `$( ... )*` repeats its body once for each item that matched. `@each_rule` uses it to write one test per rule name, like the `for (const rule of RULES)` above.
- `$crate::conformance::$rule(&database)` calls the function of the suite with the same name as the test (`$crate` is the root of the crate where the macro was defined, so the call works from any module).
- `@test`, `@tests` and `@each_rule` are **internal rules**: arms that only the macro itself calls. By convention an arm that starts with `@` is internal, so these arms work like private helper functions of the macro. The two last arms are the public ones: `conformance_tests!(start)` and `conformance_tests!(start, every_rule_fails)` (the second checks that each rule fails on an adapter that is broken on purpose).
- The start function returns an `Option`: `None` means "no database on this machine", and the test returns early (see [Tests that need a real service](#tests-that-need-a-real-service)).

The macro body is pasted at the call site, so the tests appear in the caller's module. A macro is needed because a `#[test]` function cannot be generic: without it each adapter would copy the list of tests by hand and could miss a rule.

### A test double for a trait

`vi.mock` replaces a module. In Rust, the code depends on a trait and the test implements it (`FakeClock`, the in-memory backend). The fake lives under `#[cfg(test)]` so the real build never has it.

```ts
const clock = { now: vi.fn(() => 0) };
clock.now.mockReturnValue(5_000);
```

```rust
// clock.rs: reads the time of Tokio, so it moves with `tokio::time::pause()` and `advance`
#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct FakeClock {
    start: tokio::time::Instant,
}

#[cfg(test)]
impl Clock for FakeClock {
    fn now(&self) -> Duration {
        self.start.elapsed()
    }
}

// engine/tests/rust_api.rs: moves only when the test moves it, with no Tokio at all
#[derive(Clone, Default)]
struct ManualClock {
    now: Arc<Mutex<Duration>>,
}

impl ManualClock {
    fn advance(&self, duration: Duration) {
        *self.now.lock().expect("the clock lock") += duration;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Duration {
        *self.now.lock().expect("the clock lock")
    }
}
```

`FakeClock` has no `advance` of its own: the test moves it with `tokio::time::advance`. `ManualClock` is for the test that must work after the runtime is gone.

### Tests that need a real service

```ts
const url = process.env.DOYEN_TEST_REDIS_URL;
const test = url ? it : it.skip;
test("joins through redis", async () => { ... });
```

```rust
/// The URL of a test database, read from `variable` (for example `DOYEN_TEST_REDIS_URL`).
/// `None` means "skip the database tests". With `DOYEN_TEST_REQUIRE_DB=1` a missing URL panics.
#[expect(
    dead_code,
    reason = "the test start functions of the Redis and MongoDB adapters call it"
)]
pub(crate) fn database_url(variable: &str) -> Option<String> {
    let url = std::env::var(variable).ok();
    let require_database = std::env::var(REQUIRE_DATABASE_VARIABLE).ok();
    match choose_database_url(variable, url, require_database) {   // shortened: this function holds the rule
        Ok(Some(url)) => Some(url),
        Ok(None) => {
            // Only the name of the variable: the URL may hold a password.
            eprintln!("skipped: {variable} is not set");
            None
        }
        Err(message) => panic!("{message}"),
    }
}

#[tokio::test]
async fn joins_through_redis() {
    let Some(url) = database_url("DOYEN_TEST_REDIS_URL") else {
        return;   // skipped: no database on this machine
    };
    ...
}
```

A test cannot mark itself "skipped" at run time; an early `return` is the convention. `database_url` takes the full name of the variable. Set `DOYEN_TEST_REQUIRE_DB=1` in CI: a missing URL then panics, so the test fails instead of passing without a database.

### Run a future without Tokio: a hand-written `block_on`

One test must run with **no Tokio runtime** (spec rust-api, "No Tokio runtime"), so `#[tokio::test]` cannot be used, and the crate has no `futures-executor` dependency. So the test file has its own small `block_on`. It is the one place in the repo that calls `Pin` methods and polls a future by hand.

```ts
// no equivalent: in JS you cannot poll a promise by hand; `await` is the only way
const output = await promise;
```

```rust
/// Run a future to its end on this thread, without Tokio, like
/// `futures::executor::block_on`. A short busy loop is enough for a test.
fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);                       // std::pin::pin!, a pinned borrow
    let mut context = Context::from_waker(Waker::noop()); // a waker that does nothing (Rust 1.85)
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        std::thread::yield_now();
    }
}
```

`poll` asks the future "are you done?" once. `Poll::Ready(output)` means done; `Poll::Pending` means ask again later. A real runtime sleeps until the waker is called; this loop just asks again. Do not copy it into the library code.

### A runtime that the test owns

To check what happens after the runtime shuts down, the test builds the runtime itself, and drops it.

```ts
const worker = new Worker(file);
await worker.terminate();                  // the event loop is gone, then check what is left
```

```rust
#[test]
fn after_the_runtime_shuts_down_nothing_renews_and_the_lease_ends() {
    let clock = ManualClock::default();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)               // the same as `#[tokio::test(start_paused = true)]`
        .build()
        .expect("a runtime");
    let (doyen, adapter) = runtime.block_on(async { /* join, sleep 5 s */ });

    drop(runtime);                        // the driver task is gone
    clock.advance(Duration::from_secs(10));
    assert_ne!(doyen.status(), Status::Master);
}
```

### Is it `Send` and `Sync`? A check the compiler runs

```ts
// TS has no equivalent: types are erased and there are no threads to check
```

```rust
// Compile-time check of spec rust-api: "Doyen SHALL be Send and Sync".
fn assert_send_and_sync<T: Send + Sync>() {}
assert_send_and_sync::<Doyen>();        // turbofish: the type argument is written out
```

The function has an empty body. If `Doyen` were not `Send + Sync`, this line would not compile, so the test fails at build time.

### Test helpers: `thread_local!`, `RefCell`, `Once`

The log helper of the tests (`engine/tests/helpers.rs`) installs a `tracing` subscriber that it writes by hand (a `Subscriber` plus a `Visit` that reads the fields). It uses `thread_local!` with a `RefCell` (a value of one thread that can change, like a module-level `let` per thread) and a `static Once` (≈ `if (!initialized) { initialized = true; init(); }`, but safe for threads). Use these in test helpers only.

---

## 17. Common pitfalls for TS devs

1. **Trying to mutate while iterating** — collect indices/changes first, then apply. Or use `retain`, `iter_mut`.
2. **Returning references to locals** — return owned values (`String`, `Vec`) instead.
3. **Overusing `.unwrap()`** — fine in prototypes/tests; in real code use `?` or `expect("why it can't fail")`.
4. **Expecting `==` on structs to work** — `#[derive(PartialEq)]` first.
5. **Self-referential structs / graphs** — use arenas or index-based designs (`Vec<Node>` + `usize` ids).
6. **String indexing** — UTF-8 bytes; use `.chars()` or `.char_indices()`.
7. **Integer overflow** — panics in debug, wraps in release. Use `checked_add`, `saturating_add` when it matters.
8. **Getting stuck on lifetimes** — clone, use owned types, restructure. Lifetime gymnastics are rarely the answer early on.
9. **Async + `Send` errors** — usually a non-`Send` type (`Rc`, `RefCell`, std `MutexGuard`) held across an `.await`. Swap for `Arc` / drop it before awaiting.
10. **Huge compile errors** — read the _first_ error; rustc messages are excellent and usually suggest the fix.
11. **A future that never runs** — a `Promise` starts when created; a future starts when awaited or spawned. `let f = fetch();` does nothing until `f.await`.
12. **A lock held across `.await`** — the std `MutexGuard` is not `Send`, and even when it compiles it blocks every other task. Take the lock, copy what you need, drop the guard, then await.
13. **A test that sleeps for real** — `std::thread::sleep` in an async test freezes the runtime; use `#[tokio::test(start_paused = true)]` (or `tokio::time::pause()`) + `advance`.
14. **`let _ = guard;` drops the guard on that line** — name it (`let _guard = …;`) when it must live to the end of the block.
15. **A first tick that comes at once** — `tokio::time::interval` ticks immediately; `setInterval` waits one period. Use `interval_at`.

---

## 18. Suggested learning path

1. **The Rust Book** — chapters 1–10 thoroughly (ownership, structs, enums, errors, generics, traits). <https://doc.rust-lang.org/book/>
2. **Rustlings** — small exercises to cement syntax. `cargo install rustlings`
3. **Rust by Example** — quick reference by topic. <https://doc.rust-lang.org/rust-by-example/>
4. Build a **CLI** (`clap` + `serde` + `anyhow`) — port a small Node script you already have.
5. Build an **HTTP API** (`axum` + `sqlx` + `tokio` + `tracing`) — port a small Express/Fastify service.
6. Read Book chapters 13–16, 19 (iterators/closures, smart pointers, concurrency, advanced traits).
7. **Async Book** + Tokio tutorial. <https://tokio.rs/tokio/tutorial>
8. Later: _Rust for Rustaceans_ (Jon Gjengset), _Rust Atomics and Locks_ (Mara Bos).

> [!tip] Migration strategy for an existing TS codebase
> Don't rewrite everything. Identify a hot path (CPU-heavy parsing, image processing, crypto) and expose it to Node via **`napi-rs`** or **WASM** (`wasm-pack`). Grow Rust from there, or extract a standalone service behind an HTTP/gRPC boundary.

---

## Quick cheat sheet

| TypeScript                 | Rust                            |
| -------------------------- | ------------------------------- |
| `const x = 1`              | `let x = 1;`                    |
| `let x = 1`                | `let mut x = 1;`                |
| `function f(a: T): R {}`   | `fn f(a: T) -> R {}`            |
| `(a) => a + 1`             | `\|a\| a + 1`                   |
| `interface` / `implements` | `trait` / `impl Trait for Type` |
| `class`                    | `struct` + `impl`               |
| `A \| B` (tagged)          | `enum`                          |
| `"a" \| "b"`               | `enum` with no data + `Display` |
| `T \| undefined`           | `Option<T>`                     |
| `throw` / `try-catch`      | `Result<T, E>` + `?`            |
| `switch`                   | `match` (exhaustive)            |
| `for (const x of xs)`      | `for x in &xs`                  |
| `console.log(x)`           | `println!("{x:?}")` / `dbg!(x)` |
| `JSON.stringify(x)`        | `serde_json::to_string(&x)?`    |
| `export`                   | `pub`                           |
| package-internal           | `pub(crate)`                    |
| `import { a } from "./m"`  | `use crate::m::a;`              |
| `async/await`              | `async/.await` + tokio          |
| `Promise.race`             | `tokio::select!`                |
| `EventEmitter`             | `mpsc` channel                  |
| `AsyncIterable<T>`         | `impl Stream<Item = T>`         |
| `vi.useFakeTimers()`       | `#[tokio::test(start_paused = true)]` / `tokio::time::pause()` |
| `f<T>(x)`                  | `f::<T>(x)` (turbofish)         |
| `new Promise(() => {})`    | `std::future::pending()`        |
| `using _guard = …`         | `let _guard = …;` (not `let _ = …;`) |
| `map.get(k) ?? map.set(k, v)` | `map.entry(k).or_insert_with(f)` |
| `xs.map(String)`           | `xs.iter().map(String::as_str)` |
| `Symbol.dispose`           | `impl Drop`                     |
| `readonly`                 | default (no `mut`)              |
| `// eslint-disable-next-line` | `#[expect(lint)]`            |

Related: Rust, Tauri, Node
