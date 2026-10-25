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

## Device facts

- Updates are blocked: `/usr/bin/otaupd` and `/usr/bin/otav3` are renamed to `*.bck`.
- Dropbear host keys were regenerated on the device; the shared ones shipped in the public
  USBNetwork package are kept in `/mnt/us/usbnet/etc/shared-hostkeys.orig/`.
- USBNetwork starts at boot (`/mnt/us/usbnet/auto`), with sshd; USB drive mode is reached by
  toggling it off in KUAL.
- Toggle USBNetwork once and wait: overlapping toggles leave sshd stopped.
- An SSH session's `PATH` is `/usr/bin:/bin`: use `/sbin/reboot`, `/usr/sbin/mntroot`.
- The root filesystem is read-only; `mntroot rw` / `mntroot ro` around any change under `/etc`.
