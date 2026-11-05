# doyen

> 🚧 **Status: planning — not usable yet.** This README shows the planned design; the first
> release comes soon. The npm package `@leofdss/doyen` and the crate `doyen` only reserve the
> names.

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

- **One API, many databases.** Adapters for Redis and MongoDB first, then PostgreSQL.
- **Three languages, one core.** The core is written in Rust; Node.js and Go use it through
  bindings.
- **A small API.** `join`, `status`, `onStatusChange`, `token` and `leave` — nothing more.
- **Fencing token.** While an instance is master, `token()` returns `(generation, number)`,
  which only grows — also after a restart or a data loss of the database, because the
  generation comes from the database server's clock (rare exceptions in
  [Using the token](#using-the-token)). Send it with your writes, and your database can reject
  a late write from an old master.

Planned API (Node.js):

```ts
import { DatabaseType, Doyen, State } from "@leofdss/doyen";

const doyen = new Doyen();

doyen.onStatusChange((status) => {
  if (status.state === State.master) startJobs();
  else stopJobs();
  if (status.state === State.offline) {
    console.warn(`doyen offline: ${status.reason}`, status.error ?? "");
  }
});

await doyen.join({ type: DatabaseType.redis, url: "redis://localhost:6379", group: "billing-jobs" });

// Read the token at each write and send it with the write.
const token = doyen.token(); // null when this instance is not master
if (token) await saveInvoice(invoice, token); // see "Using the token"

process.on("SIGTERM", async () => {
  await doyen.leave(); // the next instance takes over at once
  process.exit(0);
});
```

> [!IMPORTANT]
> The token protects your data only if your database checks it: keep the highest token seen
> and reject lower ones (compare `generation` first, then `number`). Without that check, after
> a long pause (for example, garbage collection) or a Redis failover, an old master can still
> write for a moment. **With Redis, the check is required:** in a network partition an old
> master can write for as long as the partition lasts (see
> [Planned databases](#planned-databases)). Actions outside a database (an email, a call to
> another service) can happen twice. See [Using the token](#using-the-token).

## Status

`status()` returns `{ state }`: `master`, `slave` or `offline`. When the state is `offline`,
the status also has a `reason`, so you can find the problem, and `error`: the last database
error, when there is one (for example a wrong password).

| Reason | When | Where to look |
|---|---|---|
| `notJoined` | `join()` was not called yet, or it rejected. | Your app: call `join()`. If it rejected, its error says why. |
| `connecting` | `join()` is running, and the instance has no place in the line yet. doyen tries again after each network error. | This instance never reached the database: URL, DNS, network, firewall, or the database is not running. |
| `connectionLost` | The instance had a place, but it could not renew it in time. doyen keeps trying to renew the same place. | The database does not answer or is too slow, or the process stopped for a long time (for example a container pause). `error` shows the last database error. |
| `reconnecting` | The place is gone (it expired, or the database lost it). doyen joins again, at the end of the line. | After `connectionLost`: the problem lasted longer than `timeoutMs`. Directly from `master` or `slave`: the database lost data (a Redis restart without persistence, a failover). If it stays, `error` says why the joins fail. |
| `left` | After `leave()`. | Normal stop. |

- When the database goes down, the status changes to `offline` (`connectionLost`) when the
  local lease ends: 7–9 s with the defaults. A shorter problem does not change the master.
- `onStatusChange` runs on every change, and a new reason is a change too. A failed retry
  with the same reason does not call it, even when `error` changes; `status()` always has the
  last `error`.
- After `join()` resolves, doyen never gives up: it retries every error (also a wrong
  password, which you can fix while the service runs).
- The reasons are strings, so a log line shows `connectionLost`, not a number.
- **Warnings.** doyen warns once, at connect, about a setup that works but has a known risk:
  in Node.js with `process.emitWarning` (codes `DOYEN_MONGODB_ARBITER`,
  `DOYEN_REDIS_EVICTION`, `DOYEN_REDIS_NO_MIN_REPLICAS` and `DOYEN_REDIS_SENTINEL_TIMING`), in
  Rust with `tracing`, in Go with `slog`.

## Using the token

Your database keeps the highest token it has seen, and each write checks it first. Example
with PostgreSQL and the `pg` package — one fence row per group:

```sql
CREATE TABLE doyen_fence (
  grp text PRIMARY KEY,
  gen bigint NOT NULL,
  num bigint NOT NULL
);
INSERT INTO doyen_fence VALUES ('billing-jobs', 0, 0);
```

```ts
import pg from "pg";
import type { Token } from "@leofdss/doyen";

const pool = new pg.Pool();

async function saveInvoice(invoice: Invoice, token: Token): Promise<boolean> {
  const client = await pool.connect();
  try {
    await client.query("BEGIN");

    // Keep the highest token. An older token changes no row.
    const fence = await client.query(
      `UPDATE doyen_fence SET gen = $2, num = $3
        WHERE grp = $1 AND (gen, num) <= ($2::bigint, $3::bigint)`,
      ["billing-jobs", token.generation, token.number],
    );
    if (fence.rowCount === 0) {
      await client.query("ROLLBACK");
      // A newer token was used: a newer master exists, or this master's token changed
      // (read doyen.token() again before the next write).
      return false;
    }

    await client.query("UPDATE invoices SET status = $2 WHERE id = $1", [
      invoice.id,
      invoice.status,
    ]);
    await client.query("COMMIT");
    return true;
  } catch (err) {
    await client.query("ROLLBACK");
    throw err;
  } finally {
    client.release();
  }
}
```

Why it works:

- `(gen, num) <= (…)` compares `gen` first, then `num` — the order of the token.
- The `UPDATE` locks the fence row until `COMMIT`, so the writes of two masters run one after
  the other, never at the same time.
- After the first write of the new master, every write of the old master changes no fence
  row and is rolled back.

Any database with transactions can do the same: in one transaction, update the fence record
only if the token is not lower, then write your data.

The token can go back only in rare cases. Then the fence rejects every write of the new
master, and you reset the fence row:

- After you restore doyen's MongoDB data from a backup. Better: delete the group's document
  in doyen's collection (default `doyen`) after the restore; doyen creates it again with a
  new generation.
- Redis: Redis goes back to older data (a failover that loses recent writes, or an old
  primary that comes back), and no running instance saw the newer generation (for example
  every instance restarted in the meantime). While one instance that saw it still runs,
  doyen sees that the data went back and makes a new, higher generation.

## Timeout and renew interval

You choose them in the config. The right values depend on your infrastructure, so test them
there.

| Setting | Default | What it does |
|---|---|---|
| `timeoutMs` | `10000` (10 s) | An instance that does not renew for this long leaves the line. |
| `renewIntervalMs` | `2000` (2 s) | How often each instance renews its place. |

With the defaults, the next instance takes over 8–12 s after the master crashes (at once
after `leave()`), and a database problem shorter than 6 s never changes the master.

- A **shorter timeout** gives a faster takeover, but more master changes when the network or
  the database is slow for a moment.
- Keep `timeoutMs` at **3 to 5 times** `renewIntervalMs`. `join` rejects a config where
  `timeoutMs` is less than 3 times `renewIntervalMs`: with less, a slow but good renew can
  arrive after the local lease ended, and the status goes `offline` for nothing.
- **Test a database failover.** If it takes longer than `timeoutMs`, there is no master
  during the failover. That is safe, but your master work stops for that time.

## Planned databases

| Database | Needs | Note |
|---|---|---|
| Redis | 6.2 or newer; one server, Redis Cluster or Sentinel (not Active-Active); best with `noeviction` (see Redis memory) | The weakest for safety: Redis can lose recent writes after a restart or a failover, so two masters can exist for a moment — and for a whole network partition (see below). The token helps your database reject the old one. |
| MongoDB | 5.0 or newer; a replica set or a sharded cluster | All writes use `w: "majority"`. With an arbiter (primary, secondary, arbiter), if one data node is down, no instance can be master until it comes back. doyen warns at connect (`DOYEN_MONGODB_ARBITER`). |
| PostgreSQL | Later | |

- **Redis URL:** `redis://` or `rediss://` (TLS) for one server or Redis Cluster (doyen finds
  out which); `redis+sentinel://[user:pass@]host1:26379,host2:26379/<master name>` for
  Sentinel (`rediss+sentinel://` with TLS; the sentinels' own login in the query:
  `?sentinelUsername=…&sentinelPassword=…`). With one URL and no Sentinel, doyen follows a
  failover only if the name moves to the new primary (for example the primary endpoint of a
  managed service).
- **Redis network partition:** with Sentinel or a managed endpoint, an old primary cut off
  from the others keeps accepting writes, so its master keeps working while the other side
  has a new master: two masters for as long as the partition lasts. Set
  `min-replicas-to-write 1` and `min-replicas-max-lag 10` on the primary: an isolated primary
  then stops accepting writes, and the old master goes `offline` about
  `min-replicas-max-lag` + `timeoutMs` after the partition began (~20 s). doyen warns when a
  primary with replicas does not have it (`DOYEN_REDIS_NO_MIN_REPLICAS`).
- **Sentinel timing:** the old master goes `offline` before the new one starts only when
  Sentinel's `down-after-milliseconds` is more than `min-replicas-max-lag` + `timeoutMs` +
  1 s (true with the default 30 s). With a shorter value (for example 5 s), there are two
  masters for some seconds; the token covers it, and doyen warns
  (`DOYEN_REDIS_SENTINEL_TIMING`).
- **Redis Cluster:** a primary cut off from the others stops writes after
  `cluster-node-timeout`, so two masters can exist for about that time (the token covers it).
  No warning.
- **Redis keys:** they start with `doyen:{group}:`, so a `group` cannot contain `{` or `}`.
  Deleting them is safe (the instances join again); do not edit them.
- **Redis user (ACL):** doyen's user needs the keys `~doyen:*`, the channels `&doyen:*`, the
  plain data and transaction commands, and `INFO`, `TIME` and `WAIT`. `INFO` is in the
  `@dangerous` category: a user with `-@dangerous` needs `+info`. With Sentinel, the
  sentinels' user needs `SENTINEL MASTER`. The exact list comes with 0.1.
- **Redis behind a proxy or a managed service:** doyen reads `INFO` from the server that holds
  the keys. If a field it needs is missing, `join` rejects with `DOYEN_UNSUPPORTED` and names
  the field. Active-Active (several writable primaries) is not supported.
- **Redis memory:** with `maxmemory` set, an `allkeys-*` policy is refused (Redis could delete
  the counters), and a `volatile-*` policy gives the warning `DOYEN_REDIS_EVICTION` (Redis
  could delete a member's place: a master change, with two masters for up to one renew
  interval — the token covers it). Best: `noeviction`.
- **MongoDB database:** the one in the URL path, else `doyen`. The settings `database` and
  `collection` (default `doyen`) change it.

## Why the name

The *doyen* of the diplomatic corps is the ambassador who has served longest in a country.
When they leave, the next longest-serving ambassador becomes doyen — no election. That is the
rule of this library.

## Planned releases

| Version | What | Where |
|---|---|---|
| 0.1 | Rust core + Redis and MongoDB adapters | crates.io |
| 0.2 | Node.js package | npm: `@leofdss/doyen` |
| 0.3 | Go module | `github.com/leofdss/doyen-go` |
| 0.4 | PostgreSQL adapter | all three |

## Development

The development environment is a Distrobox container with Rust, Node.js and Go — see
[distrobox/README.md](distrobox/README.md):

```bash
distrobox assemble create --file distrobox/distrobox.ini
distrobox enter doyen
cargo build
```

| Path | What |
|---|---|
| `crates/doyen` | The Rust crate `doyen` (now a placeholder that reserves the name) |
| `bindings/node` | The npm package `@leofdss/doyen` (now a placeholder) |
| `openspec/` | The specs and the planned changes ([OpenSpec](https://github.com/Fission-AI/OpenSpec)) |
| `distrobox/` | The development image and container |

The work is planned as OpenSpec changes, applied one at a time in the order of
[TODO.md](TODO.md). How to work here (also for AI agents): [AGENTS.md](AGENTS.md); how to
contribute: [CONTRIBUTING.md](CONTRIBUTING.md); who decides: [GOVERNANCE.md](GOVERNANCE.md).
Commits follow [Conventional Commits](https://www.conventionalcommits.org/); the release
number comes from them (SemVer, [release-please](https://github.com/googleapis/release-please)),
one version for the crate, the npm package and the Go module. Contributions
made with AI tools are welcome under the [AI policy](AI_POLICY.md). To report a
vulnerability, see [SECURITY.md](SECURITY.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion
in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above,
without any additional terms or conditions.
