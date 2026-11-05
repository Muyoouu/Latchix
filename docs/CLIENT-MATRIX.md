# Client matrix

Phase 1 launch clients:

| Client | Mode | Gate |
|---|---|---|
| OpenSSH `sftp` | Automated | Required |
| rclone | Automated | Required |
| FileZilla | Manual | Recorded checklist |

## Automated checks

The Rust integration tests start a real `russh` listener on an ephemeral
localhost port and connect with the installed client.

- `cargo test -p latchix-sftp --test openssh`
- `cargo test -p latchix-sftp --test rclone`
- `cargo test -p latchix-sftp --test extensions`
- `cargo test -p latchix-sftp --test handles`
- `scripts/test-container.sh -- cargo test --locked -p latchix-sftp`

OpenSSH covers login, realpath, stat, upload, download, list, mkdir, rename,
remove, and rmdir. rclone covers a recursive copy and binary payloads.
CI installs `openssh-client` and `rclone` so neither test skips.

The server advertises `posix-rename@openssh.com` and `fsync@openssh.com`.
Rename is also available through the SFTP v3 core request. Unsupported
extensions return `SSH_FX_OP_UNSUPPORTED`.

## FileZilla manual checklist

Run the dev server in the container:

```bash
scripts/sftp-dev-container.sh
```

Connect from the host:

- Protocol: SFTP
- Host: `127.0.0.1`
- Port: `2222`
- User: `demo`
- Key file: `.ci/sftp-dev/id_ed25519`

Record the FileZilla version and each result:

- [ ] Connect and list the root.
- [ ] Upload a small text file.
- [ ] Download the same file and compare contents.
- [ ] Create a directory.
- [ ] Rename a file.
- [ ] Delete a file.
- [ ] Delete the directory.
- [ ] Upload a file larger than 1 MiB.
- [ ] Confirm a symlink created on the host under `.ci/sftp-dev/root` is not
      followed by the client.

A failed item blocks Gate 1 until fixed or explicitly deferred.
