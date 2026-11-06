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

#[cfg(not(feature = "redis"))]
fn connect_redis() -> Result<Connection, Error> {
    Err(Error::unsupported("the redis feature is off"))
}
```

The user picks: `cargo add doyen --no-default-features --features redis`. Tests run with `--all-features` and with `--no-default-features` to cover both.

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

let u = User { id: 1, name: "Ana".to_string() };
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

### An enum that may grow: `#[non_exhaustive]`

In TS, adding a member to a union breaks every exhaustive `switch` of every user. Rust can mark a public enum or struct so that users outside the crate must keep a `_ =>` arm (and cannot build the struct with a literal). Add it to every public enum of a library that will get new variants in a minor version.

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

- `to_owned()`, `to_string()` and `String::from(s)` all make a `String` from a `&str`; pick one and keep it (this repo uses `to_owned()` for a plain copy and `to_string()` for a `Display` value such as a number or an enum).
- A struct field is `String` (owned) unless there is a strong reason; a `&str` field forces a lifetime on the struct (see [Lifetimes](#lifetimes)).

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
| A separate copy                  | `.clone()`                         | `structuredClone(x)`        |
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
  private state = { status: "offline" as State, subscribers: [] as Subscriber[] };

  status(): State {
    return this.state.status;
  }

  setStatus(status: State): void {
    this.state.status = status;
    for (const s of this.state.subscribers) s(status);
  }
}
```

```rust
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

struct State {
    status: Status,
    subscribers: Vec<mpsc::UnboundedSender<Status>>,
}

struct Shared {
    state: Mutex<State>,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        // A thread that panicked while it held the lock "poisons" it; the data is still
        // fine for our use, so take it anyway.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn status(&self) -> Status {
        self.lock().status.clone()       // the guard is dropped at the end of this line
    }

    fn set_status(&self, status: Status) {
        let mut state = self.lock();
        state.status = status.clone();
        state.subscribers.retain(|s| s.send(status.clone()).is_ok());
    }                                     // lock released here
}

let shared = Arc::new(Shared { state: Mutex::new(State::default()) });
let for_the_task = Arc::clone(&shared);  // one more owner, same data
```

- Keep the guard short: never hold it across an `.await` (use `tokio::sync::Mutex` if you must).
- A single shared flag or counter does not need a `Mutex`: `Arc<AtomicBool>` / `AtomicU64` with `load(Ordering::SeqCst)` and `store(..)` (≈ a shared `boolean` that any closure may flip; `SeqCst` is the simple, always-correct ordering).

### Cleanup when a value goes away: `Drop`

JS has `Symbol.dispose` with `using` (TS 5.2) and finalizers; Rust has `Drop`, and it always runs, at a known time (end of scope, or when the last `Arc` owner goes).

```ts
class CancelJoinOnDrop implements Disposable {
  constructor(private readonly shared: Shared) {}
  [Symbol.dispose]() {
    this.shared.setStatus({ state: "offline", reason: "notJoined" });
  }
}

{
  using guard = new CancelJoinOnDrop(shared);
  await connect();           // if this throws, dispose still runs
  guard.disarm();
}
```

```rust
struct CancelJoinOnDrop {
    shared: Arc<Shared>,
    armed: bool,
}

impl Drop for CancelJoinOnDrop {
    fn drop(&mut self) {
        if self.armed {
            self.shared.set_status(Status::offline(Reason::NotJoined));
        }
    }
}

{
    let mut guard = CancelJoinOnDrop { shared: shared.clone(), armed: true };
    connect().await?;        // on error or cancel, `drop` still runs
    guard.armed = false;
}                            // dropped here
```

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

The two lifetimes you will see in this repo:

- `'static`: "lives for the whole program". A string literal is `&'static str`; a trait bound `T: 'static` means "`T` holds no borrow", which every value sent to a background task must satisfy (the task may outlive the caller). It is like the rule that a callback passed to `setTimeout` must not use a variable the caller will free.
- `'_`: "the compiler fills it in" (`MutexGuard<'_, State>`, `Formatter<'_>`). Write it when the type asks for a lifetime and you do not care which.

---

## 6. `null`/`undefined` → `Option<T>`

```ts
function find(id: number): User | undefined { ... }
const name = find(1)?.name ?? "anon";
```

```rust
fn find(id: u64) -> Option<User> { ... }

let name = find(1).map(|u| u.name).unwrap_or_else(|| "anon".into());
```

| TS                        | Rust                                       |
| ------------------------- | ------------------------------------------ |
| `x?.y`                    | `x.map(\|v\| v.y)` / `x.as_ref().map(...)` |
| `x ?? d`                  | `x.unwrap_or(d)` / `unwrap_or_else(f)`     |
| `x!`                      | `x.unwrap()` / `x.expect("msg")` (panics)  |
| `if (x) {...}`            | `if let Some(v) = x {...}`                 |
| optional param `a?:`      | `a: Option<T>`                             |
| `x ?? throw err`          | `x.ok_or(err)?`                            |
| `x?.toString()` on a text | `x.as_deref()` (`Option<String>` → `Option<&str>`) |
| `const v = x; x = undefined` | `let v = x.take();`                     |

Inside a function returning `Option`, `?` short-circuits on `None` — like `?.` but for the whole function.

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
#[derive(Debug, thiserror::Error)]
#[error("{code}: {message}")]
pub struct Error {
    code: ErrorCode,
    message: String,
}

impl Error {
    pub fn code(&self) -> ErrorCode { self.code }
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
const url = (() => { try { return new URL(raw); } catch { throw invalidConfig("url"); } })();
const token = status.token ?? throwLeft();
```

```rust
let url = url::Url::parse(raw).map_err(|_| Error::invalid_config("url"))?;
let token = status.token.ok_or(Error::left())?;
```

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
    fn describe(&self) -> String { "dog".into() }
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
    pub fn rename(&mut self, name: &str) { self.name = name.into(); }
}
```

`&self` is a method that reads; `&mut self` is a method that changes the object (the caller needs a `mut` binding); `self` consumes it (rare: builders, `into_*`).

### An interface with async methods: `#[async_trait]`

A TS interface can have `Promise` methods. A Rust trait can have `async fn` too, but then it cannot be used as a `dyn` object (a value typed by the interface). The `async-trait` crate fixes that with one attribute on the trait and on each `impl`.

```ts
interface Backend {
  connect(): Promise<void>;
  join(member: Member): Promise<View>;
  watch(group: string): Promise<AsyncIterable<void> | undefined>;
}
```

```rust
use async_trait::async_trait;

#[async_trait]
pub(crate) trait Backend: Send + Sync + 'static {
    async fn connect(&self) -> Result<()>;
    async fn join(&self, member: &Member) -> Result<View>;
    async fn watch(&self, group: &str) -> Result<Option<BoxStream<'static, ()>>>;
}

#[async_trait]
impl Backend for MemoryBackend { ... }
```

`Send + Sync + 'static` after the trait name: every implementation can be shared between tasks and holds no borrow. Without it the engine could not keep a `dyn Backend` inside a `tokio::spawn`.

### A value typed by the interface: `dyn Trait`

```ts
class Doyen {
  private backend: Backend;             // any implementation
  constructor(backend: Backend) { this.backend = backend; }
}
```

```rust
pub struct Connection {
    backend: Arc<dyn Backend>,            // any implementation, shared, chosen at run time
}

fn with_backend(backend: Box<dyn Backend>) -> Doyen { ... }
```

- `Arc<dyn Trait>` when it is shared; `Box<dyn Trait>` when one owner holds it.
- `impl Trait` in a parameter (`fn run(backend: impl Backend)`) is the static form: one copy of the function per type, no pointer. Use `dyn` when the type is chosen at run time (the config says `redis` or `mongodb`).

### A callback stored in a field

```ts
type LivenessCheck = () => boolean;
let check: LivenessCheck | undefined;

function setLiveness(doyen: Doyen, check: LivenessCheck): void { ... }
```

```rust
type LivenessCheck = Arc<dyn Fn() -> bool + Send + Sync>;
let check: Option<LivenessCheck>;

pub fn set_liveness(doyen: &Doyen, check: impl Fn() -> bool + Send + Sync + 'static) {
    let check: LivenessCheck = Arc::new(check);
    ...
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
async function callWithTimeout<T>(timeoutMs: number, call: Promise<T>): Promise<T> {
  return Promise.race([call, rejectAfter(timeoutMs)]);
}
```

```rust
async fn call_with_timeout<T>(
    timeout: Duration,
    call: impl Future<Output = backend::Result<T>>,
) -> backend::Result<T> {
    match tokio::time::timeout(timeout, call).await {
        Ok(result) => result,
        Err(_elapsed) => Err(BackendError::Network("timed out".to_owned())),
    }
}
```

`impl Future<Output = T>` is the parameter type of "a promise of `T`". A generic `<T>` stays when the caller must see the type again in the return.

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

Rule for this repo: a closure passed to `tokio::spawn` or kept in a struct is `move`, and the values it needs are cloned before it (`let shared = Arc::clone(&shared);` on the line above). A closure inside a closure is a sign to name a function.

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
| `setInterval`       | `tokio::time::interval(dur)` + `.tick().await`                  |
| `AbortController`   | drop the future / `CancellationToken`                           |
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
let shared = Arc::clone(&shared);
tokio::spawn(async move {
    renew_loop(shared).await;
});
```

The task runs on another thread, so everything it captures must be `Send + 'static` (owned or `Arc`, no `Rc`, no borrow). The `JoinHandle` that `spawn` returns can be awaited like a promise; dropping it does **not** cancel the task (a dropped future does; see [Stop a task](#stop-a-task-a-dropped-sender)).

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
sender.send(Status::master()).ok();     // `Err` means the reader is gone: ignore it
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

### Stop a task: a dropped sender

In TS the object keeps an `AbortController` and calls `abort()` in `close()`. In Rust the object keeps a sender; when the object is dropped the sender is dropped, the receiver sees "closed", and the task ends. No `close()` to forget.

```ts
class Doyen {
  private readonly stop = new AbortController();
  constructor() {
    runDriver(this.stop.signal);          // the loop checks `signal.aborted`
  }
  [Symbol.dispose]() {
    this.stop.abort();
  }
}
```

```rust
pub struct Doyen {
    _stop_driver_on_drop: oneshot::Sender<()>,   // never used: dropping it is the signal
}

impl Doyen {
    pub fn new() -> Self {
        let (stop_sender, stop_receiver) = oneshot::channel();
        tokio::spawn(run_driver(stop_receiver));
        Self { _stop_driver_on_drop: stop_sender }
    }
}
```

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
async fn unless_stopped<T>(&mut self, work: impl Future<Output = T>) -> Option<T> {
    tokio::select! {
        value = work => Some(value),
        _ = self.leaving.changed() => None,   // the lease ended or `leave` was called
    }
}
```

The `biased;` first line makes the branches checked in order (the first ready wins) instead of at random; use it when the stop branch must win a tie.

### A ticking loop: `interval` with missed ticks

```ts
const timer = setInterval(renew, renewIntervalMs);   // ticks queue up if `renew` is slow
```

```rust
let mut schedule = tokio::time::interval(renew_interval);
schedule.set_missed_tick_behavior(MissedTickBehavior::Skip);  // no catch-up burst after a stall
loop {
    schedule.tick().await;
    renew().await;
}
```

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
    let mut receiver = self.shared.subscribe();          // an mpsc receiver
    // Turn the receiver into a stream: `poll_fn` asks the receiver for the next value
    // each time the reader awaits (the `context` is the plumbing of `await`).
    futures_util::stream::poll_fn(move |context| receiver.poll_recv(context))
}

let mut changes = doyen.status_changes();
while let Some(status) = changes.next().await {
    println!("{status:?}");
}
```

- `impl Stream<Item = T>` in a return type: the caller gets "an async iterable of `T`" and nothing more.
- `Unpin` in the bound lets the caller call `.next()` on a plain `let mut`; without it they would need `tokio::pin!(stream)` first.
- `BoxStream<'static, T>` (= `Pin<Box<dyn Stream<Item = T> + Send>>`) is the `dyn` form, for a trait method whose return type must be one fixed type (the `Backend::watch` above). `BoxFuture` is the same for a future. Build one with `Box::pin(async move { ... })` or `.boxed()`. Treat `Pin` as part of the type name; you never call its methods.

### Fake time in tests

```ts
vi.useFakeTimers();
const joined = doyen.join(config);
await vi.advanceTimersByTimeAsync(2_000);   // every sleep/interval inside moves 2 s
```

```rust
#[tokio::test]
async fn renews_after_the_interval() {
    tokio::time::pause();                            // needs the `test-util` feature
    let joined = doyen.join(config);
    tokio::time::advance(Duration::from_millis(2_000)).await;
    ...
}
```

With time paused, `sleep`, `interval` and `timeout` inside tokio run instantly when every task is idle (auto-advance). `std::thread::sleep` is real time: never use it in an async test.

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
#[cfg(target_os = "linux")]
fn now() -> Duration { rustix::time::clock_gettime(ClockId::Boottime).into() }

#[cfg(target_os = "macos")]
fn now() -> Duration { rustix::time::clock_gettime(ClockId::Monotonic).into() }

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
compile_error!("doyen supports Linux and macOS only");   // a build error with your text

#[cfg(test)]
mod tests;                           // compiled only by `cargo test`

#[cfg(test)]
pub(crate) struct FakeClock { ... }  // a test double that the normal build never sees

#[cfg_attr(not(test), allow(dead_code))]   // "when not testing, do not warn about unused"
```

`#[cfg(...)]` removes the item from the build; there is no run-time cost and no "unreachable branch" to test. The feature switches of [section 1](#optional-dependencies--cargo-features) are the same mechanism.

### Lints: `#[expect]`, `#[allow]`, `#![deny]`

```ts
// eslint-disable-next-line @typescript-eslint/no-unused-vars -- used by the Redis tests later
function databaseUrl(name: string): string | undefined { ... }
```

```rust
#[expect(dead_code, reason = "used by the Redis tests, not in this version yet")]
fn database_url(name: &str) -> Option<String> { ... }
```

- `#[expect(lint)]` is a disable that **fails when it is no longer needed**: when the function gets a user, the compiler says to remove the attribute. Prefer it to `#[allow]`.
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
/// Joins the group. Returns when the first try is over.
///
/// # Errors
///
/// `DOYEN_INVALID_CONFIG` when the config is wrong.
pub async fn join(&self, config: DatabaseConfig) -> Result<(), Error>
```

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
log.info({ group, member: member.number }, "joined the group");
log.warn({ code: "DOYEN_MONGODB_ARBITER" }, "PSA replica set");
```

```rust
tracing::info!(group, member = member.number, "joined the group");
tracing::warn!(code = "DOYEN_MONGODB_ARBITER", "PSA replica set");
```

The fields come first, the message last. A library only emits; the app installs the subscriber (`tracing-subscriber`), like the app picks the pino transport. Never put a URL with a password in a field.

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
export function backendConformance(name: string, start: () => Promise<TestDatabase>) {
  describe(`${name} backend`, () => {
    it("gives the oldest member the master", async () => { ... });
    it("renews the lease", async () => { ... });
  });
}

// memory.test.ts
backendConformance("memory", () => MemoryDatabase.start());
```

```rust
macro_rules! conformance_tests {
    ($start:expr) => {
        #[tokio::test]
        async fn gives_the_oldest_member_the_master() {
            let database = $start.await;
            ...
        }
        #[tokio::test]
        async fn renews_the_lease() { ... }
    };
}
pub(crate) use conformance_tests;

// memory/tests.rs
crate::conformance::conformance_tests!(MemoryDatabase::start());
```

`$start:expr` is the macro parameter (an expression); the macro body is pasted at the call site, so the tests appear in the caller's module.

### A test double for a trait

`vi.mock` replaces a module. In Rust, the code depends on a trait and the test implements it (`FakeClock`, the in-memory backend). The fake lives under `#[cfg(test)]` so the real build never has it.

```ts
const clock = { now: vi.fn(() => 0) };
clock.now.mockReturnValue(5_000);
```

```rust
#[cfg(test)]
pub(crate) struct FakeClock {
    now: Mutex<Duration>,
}

#[cfg(test)]
impl FakeClock {
    pub(crate) fn advance(&self, by: Duration) { *self.now.lock().unwrap() += by; }
}

#[cfg(test)]
impl Clock for FakeClock {
    fn now(&self) -> Duration { *self.now.lock().unwrap() }
}
```

### Tests that need a real service

```ts
const url = process.env.DOYEN_TEST_REDIS_URL;
const test = url ? it : it.skip;
test("joins through redis", async () => { ... });
```

```rust
fn database_url(name: &str) -> Option<String> {
    std::env::var(format!("DOYEN_TEST_{name}_URL")).ok()
}

#[tokio::test]
async fn joins_through_redis() {
    let Some(url) = database_url("REDIS") else {
        return;   // skipped: no database on this machine
    };
    ...
}
```

A test cannot mark itself "skipped" at run time; an early `return` is the convention. Set `DOYEN_TEST_REQUIRE_DB=1` in CI to turn the early return into a failure.

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
13. **A test that sleeps for real** — `std::thread::sleep` in an async test freezes the runtime; use `tokio::time::pause()` + `advance`.

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
| `vi.useFakeTimers()`       | `tokio::time::pause()`          |
| `Symbol.dispose`           | `impl Drop`                     |
| `readonly`                 | default (no `mut`)              |
| `// eslint-disable-next-line` | `#[expect(lint)]`            |

Related: Rust, Tauri, Node
