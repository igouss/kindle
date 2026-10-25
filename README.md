# Kindle Paperwhite 2 (PW2, Wi-Fi, FW 5.12.2.2) as a USB-attached display

The Kindle is jailbroken (WinterBreak) and runs NiLuJe's USBNetwork. Over the USB cable it
is `192.168.15.244`; this host is `192.168.15.201`. `ssh kindle` logs in as root with
`~/.ssh/id_ed25519_kindle` (backed up in the dotfiles repo).

## Verify the link

```sh
host/verify-link.sh
```

Exits 0 only if all of these pass:

| Check | What it proves |
|---|---|
| `host_to_kindle` | this host reaches the Kindle over SSH |
| `wifi_off` | Kindle Wi-Fi is disabled, so the next checks can only succeed over USB |
| `default_route_usb0` | the Kindle's default route is the USB link |
| `kindle_to_host` | the Kindle fetches a per-run nonce served by this host |
| `internet_via_usb` | the Kindle fetches `$INTERNET_URL` (default `https://example.com/`) with source IP `192.168.15.244` |

Removing the firewall input rule makes `kindle_to_host` fail; removing the NAT rule makes
`internet_via_usb` fail.

## Host side (infra2)

devbox declares the link in infra2 (`downstream_links` in `ansible/host_vars/devbox.yml`,
ADR-0037), deployed by `just deploy firewall devbox`:

- the interface is always `kindle0` (systemd `.link` matching the Kindle's USB vendor)
- NetworkManager profile `homelab-kindle` holds `192.168.15.201/24`, never the default route
- IPv4 forwarding is on; `inet homelab` admits everything from `kindle0`, forwards it anywhere
  but `tailscale0`, and masquerades `192.168.15.0/24`

## Kindle side

`kindle/upstart/kindle-uplink.conf`, installed by `host/install-uplink.sh` into
`/etc/upstart/`, runs after USBNetwork starts: default route via `192.168.15.201`, resolvers
`1.1.1.1` and `8.8.8.8` in `/var/run/resolv.conf` (the target of `/etc/resolv.conf`).
`verify-link.sh` sets nothing on the Kindle, so `default_route_usb0` and `internet_via_usb`
fail if the job did not run.

## Rust apps

Each app is a crate in the workspace, drawing through the shared `eink` crate (canvas, fonts,
panel refresh), cross-compiled to `armv7-unknown-linux-musleabi`: static
musl, so the firmware's old glibc never matters, and soft-float to match its armel ABI.
`rust-toolchain.toml` installs the target; `.cargo/config.toml` links with `rust-lld`, so no
ARM gcc is needed.

```sh
host/run-app.sh hello      # build, copy to /mnt/us/kindle-dev/, run over ssh
cargo test                 # host-side tests of the pure drawing code
cargo mutants              # every viable mutant is caught; .cargo/mutants.toml leaves out
                           # only code that needs the device (ioctls, main, run)
```

`hello` draws a test page: banner, device facts, 16 gray swatches. The screen is
`/dev/fb0`, 8 bpp gray (0 black, 255 white), 758x1024 visible in a 768-byte stride. Writing
it changes nothing on the panel until `MXCFB_SEND_UPDATE` is issued; the PW2 (Carta panel)
uses FBInk's `mxcfb_update_data` (72 bytes) and the `_IOWR` `MXCFB_WAIT_FOR_UPDATE_COMPLETE`
(`.ref/FBInk/eink/mxcfb-kindle.h`). `eink/src/epdc.rs` is the only module allowed `unsafe`;
compile-time asserts pin each ioctl struct's size, since the ioctl number encodes it.
The Kindle UI still owns the screen and redraws over the page when it next updates.

## Claude Code usage on the Kindle

`usage` draws what Omarchy's agents panel shows for Claude: plan, each limit's meter and
reset countdown, tokens by day for the last week (today bold, with its prompts and
sessions), and the four heaviest models. It reads the same record the panel reads,
`~/.local/state/omarchy/agents/usage/claude.json`, which the panel's collector
(`omarchy-agent-usage-update`) rewrites every 15 minutes while the Omarchy shell runs.

```sh
host/push-usage.sh          # pipe the record to the Kindle over ssh and redraw
host/push-usage.sh --flash  # same, with a full flashing refresh
```

`kindle-usage.timer` (a systemd user unit kept in the dotfiles repo) runs the push every
5 minutes; while the Kindle is unplugged each run fails and the next one recovers.

devbox pushes and the Kindle only draws, so the Kindle needs no network code and devbox
no listener. devbox's clock is the only clock: `--now` carries its time and UTC offset, from
which the Kindle computes countdowns and today's row. Refreshes are quiet (no flash)
except in the first five minutes of each hour, to clear e-ink ghosting.

The page stays up while the Kindle sits in its screensaver state; waking the Kindle hands
the screen back to its UI until the next push. The layout is pinned by
`usage/golden/full.pgm`; after an intended layout change, review the new page and rewrite
it with `UPDATE_GOLDEN=1 cargo test -p usage golden`.

## Device facts

- Updates are blocked: `/usr/bin/otaupd` and `/usr/bin/otav3` are renamed to `*.bck`.
- Dropbear host keys were regenerated on the device; the shared ones shipped in the public
  USBNetwork package are kept in `/mnt/us/usbnet/etc/shared-hostkeys.orig/`.
- USBNetwork starts at boot (`/mnt/us/usbnet/auto`), with sshd; USB drive mode is reached by
  toggling it off in KUAL.
- Toggle USBNetwork once and wait: overlapping toggles leave sshd stopped.
- An SSH session's `PATH` is `/usr/bin:/bin`: use `/sbin/reboot`, `/usr/sbin/mntroot`.
- The root filesystem is read-only; `mntroot rw` / `mntroot ro` around any change under `/etc`.
