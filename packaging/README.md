# Packaging

The binary is the product. The OCI image is an optional packaging channel.
Phase 0 ships the process skeleton and health endpoints. The SSH/SFTP listener
arrives in Phase 1.

## Build

```bash
cargo build --locked --release --target x86_64-unknown-linux-musl -p latchix-cli
podman build --format docker -t latchix:0.1.0 .
```

The image uses `distroless/static-debian12:nonroot`, runs as UID/GID 65532, and
has no shell. Config, host keys, and the bootstrap token are mounted. They are
never baked into the image.

## Run

Create the host key and a data volume:

```bash
mkdir -p ~/.config/latchix
ssh-keygen -t ed25519 -N '' -f ~/.config/latchix/host_ed25519_key
podman volume create latchix-data
```

Use a config file that points at `/var/lib/latchix`. The example config is at
`config/latchix.example.toml`.

```bash
podman run --rm \
  --name latchix \
  --read-only \
  --tmpfs /run \
  --tmpfs /tmp \
  --cap-drop=ALL \
  --security-opt=no-new-privileges \
  --userns=keep-id:uid=65532,gid=65532 \
  -p 2222:2222 \
  -p 127.0.0.1:8080:8080 \
  -v latchix-data:/var/lib/latchix:U,Z \
  -v "$HOME/.config/latchix/config.toml:/etc/latchix/config.toml:ro,Z" \
  -v "$HOME/.config/latchix/host_ed25519_key:/etc/latchix/host_ed25519_key:ro,Z" \
  latchix:0.1.0
```

Check it without a shell:

```bash
podman exec latchix /usr/local/bin/latchix health --config /etc/latchix/config.toml
curl --fail http://127.0.0.1:8080/readyz
```

The `keep-id` mapping makes your host user appear as UID/GID 65532 inside the
container, so host key files owned by you with mode `0600` remain readable. The
`:U` volume option tells Podman to chown the named volume to the container
user. For Docker, chown the mounted host key to UID 65532 instead.

## systemd

Install the binary and config:

```bash
install -m 0755 target/x86_64-unknown-linux-musl/release/latchix /usr/local/bin/latchix
install -d -m 0750 /etc/latchix
install -m 0644 config/latchix.example.toml /etc/latchix/config.toml
install -m 0644 packaging/systemd/latchix.service /etc/systemd/system/latchix.service
useradd --system --home-dir /var/lib/latchix --shell /usr/sbin/nologin latchix
install -o latchix -g latchix -m 0600 ~/.config/latchix/host_ed25519_key /etc/latchix/host_ed25519_key
systemctl daemon-reload
systemctl enable --now latchix
```

The unit creates `/var/lib/latchix`, `/var/log/latchix`, and `/run/latchix`
through systemd. Adjust ownership if you create them yourself.

## Rootless Quadlet

Copy the Quadlet and the config:

```bash
mkdir -p ~/.config/containers/systemd ~/.config/latchix
cp packaging/quadlet/latchix.container ~/.config/containers/systemd/
cp config/latchix.example.toml ~/.config/latchix/config.toml
systemctl --user daemon-reload
systemctl --user enable --now latchix
```

Replace `Image=latchix:0.1.0` with the published image reference.

## Health

`latchix health` checks configuration, storage, database directory, host key
algorithm, and the API socket directory, then probes the running API. It does
not start the server. The container healthcheck runs the same command.

## Security notes

- RSA host keys are rejected because of RUSTSEC-2023-0071. Use Ed25519 or ECDSA.
- Do not mount the Docker or Podman socket.
- Do not bake config, host keys, or bootstrap tokens into an image.
- Keep the API on a Unix socket or a private interface.
