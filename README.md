# doyen

> 🚧 **Status: planning — not usable yet.** This repository holds the design and, soon, the
> first release. The npm package `@leofdss/doyen` 0.0.1 only reserves the name.

Pick **one master** among the instances of a service, through **the database you already
run**.

You scale a service horizontally, but some work must run on only one instance at a time — a
scheduler, a consumer that owns a set of data. ZooKeeper or etcd solve this, but most teams do
not run them. doyen uses the database the team already has.

## How it works

Every instance joins a line. The **oldest live instance is the master**. When it dies, the next
in line takes over. There is no vote and no new server: the line lives in your database.

```mermaid
graph TD
    A["A · joined 1st<br/>master"] --> B["B · joined 2nd<br/>next in line"]
    B --> C["C · joined 3rd"]
    X["A dies"] -.-> B2["B is now the oldest → master"]
    R["A comes back"] -.-> R2["A joins the end of the line"]
```

- **One API, many databases.** Adapters for Redis (first), then PostgreSQL and MongoDB.
- **Three languages, one core.** The core is written in Rust; Node.js and Go use it through
  bindings.
- **Fencing token.** Each master gets a token, `(generation, number)`, that only grows. Send it
  with your writes, and your database can reject a late write from an old master.

## Why the name

The *doyen* of the diplomatic corps is the ambassador who has served longest in a country.
When they leave, the next longest-serving ambassador becomes doyen — no election. That is the
rule of this library.

## Planned releases

| Version | What | Where |
|---|---|---|
| 0.1 | Rust core + Redis adapter | crates.io |
| 0.2 | Node.js package | npm: `@leofdss/doyen` |
| 0.3 | Go module | `github.com/leofdss/doyen-go` |
| 0.4 | PostgreSQL adapter | all three |

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion
in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above,
without any additional terms or conditions.
