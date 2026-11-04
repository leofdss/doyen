# Development environment (Distrobox)

doyen is built inside a [Distrobox](https://distrobox.it/) container, the same model as
[StemLoft](https://github.com/leofdss/stem-loft/tree/main/distrobox). This folder has two
files:

- `Containerfile` — what is **inside** the image.
- `distrobox.ini` — how the container is **created** from the image.

## Quick start

You need Distrobox and Podman on the host. From the repository root:

```bash
distrobox assemble create --file distrobox/distrobox.ini
distrobox enter doyen
```

The first command pulls the image and creates the `doyen` container. It is safe to run it
again: if the container exists, it does nothing. Add `--replace` to create it again (for
example, for a newer image). Your work is not lost: the repository and your `HOME` are on the
host.

Inside the container, the repository is at the same path as on the host, and `git`,
`cargo`, `node`, `go`, `claude` and `code` use your host logins.

## What is in the image

Arch Linux (`quay.io/toolbx/arch-toolbox`) with:

| Tool | For |
|---|---|
| Rust (`rust` package) + `cargo-llvm-cov` | the core, the adapters, test coverage |
| Node.js, npm, fnm | the npm package (napi-rs, v0.2) |
| Go, cbindgen (+ gcc from base-devel) | the Go module (cgo + C ABI, v0.3) |
| Python, git, OpenSSL | scripts, version control, TLS for the drivers |
| Claude Code, VS Code, fish, starship | editing and the shell |

Decisions (the same as StemLoft):

- **Nothing in `/home`.** Distrobox mounts the host's `HOME` over the container's `HOME`, so
  files installed there at build time disappear. Every tool goes to `/usr` or `/opt`.
- **Rust from the Arch package, not rustup.** rustup lives in `$HOME`. Arch is a rolling
  release, so the package is always the latest stable Rust.
- **`cargo-llvm-cov` installed with `--root /usr/local`**, so it is on every user's `PATH`.
- **Logins need nothing in the image.** `~/.gitconfig`, `~/.claude`,
  `~/.cargo/credentials.toml` and `~/.npmrc` live in `HOME`, which is shared: log in once,
  inside or outside.

## Test databases

Redis and MongoDB for the tests run as Podman containers **on the host**, not in this
container. From inside, use `distrobox-host-exec podman …`. (The commands come with the v0.1
tests.)

## Build and test the image locally

```bash
podman build -t localhost/doyen-dev:latest -f distrobox/Containerfile .
podman run --rm localhost/doyen-dev:latest bash -lc \
  'rustc --version && cargo llvm-cov --version && node --version && go version && cbindgen --version'
```

To create the container from the local image, change two lines in `distrobox.ini`, create it
with `--replace`, and undo the change before you commit:

```ini
image=localhost/doyen-dev:latest
pull=false
```

## Publishing (GitHub Actions)

`.github/workflows/build-dev-image.yml` builds the image and pushes it to
`ghcr.io/leofdss/doyen-dev` on each push that changes the `Containerfile`, every Monday, and
by hand (`workflow_dispatch`). Tags: `latest` and `YYYYMMDD-<short sha>`.

A new package on GHCR is **private**, even in a public repository. After the first publish,
make it public once: *Package settings* of `doyen-dev` in
`github.com/users/leofdss/packages/container/doyen-dev/settings`.
