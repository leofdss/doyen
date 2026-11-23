## Dev Distrobox

### Host

```bash
podman build -t localhost/doyen:latest -f Containerfile .
```

```bash
distrobox assemble create --replace --file distrobox.ini
```

```bash
distrobox enter doyen
```

### Distrobox

```bash
fnm install --lts
```
