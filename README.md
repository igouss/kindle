# kindle

A jailbroken Kindle Paperwhite 2 used as an e-ink dashboard, plugged into a Linux PC by USB.
The PC gives the Kindle a network and internet access over the cable, and small static Rust
programs draw straight to the e-ink panel.

<img src="docs/usage.png" alt="The Kindle in landscape showing Claude Code usage: plan, limit meters, tokens by model and by day" width="512">

The first real app shows Claude Code usage (rate limits, tokens per day, tokens per model),
the same numbers [Omarchy](https://omarchy.org)'s agents panel shows on the desktop.

## What is here

| Path | What it is |
|---|---|
| `eink/` | Rust library: an 8-bit gray canvas, TrueType text (Liberation Sans), and the panel refresh ioctls |
| `hello/` | Test page: proves the cross-build and the refresh path |
| `usage/` | The Claude Code usage dashboard |
| `host/` | Scripts that run on the PC: build and run an app, push usage, install the Kindle's uplink job, verify the link |
| `kindle/` | Files that run on the Kindle: the uplink upstart job and the link self-check |

## The device

Kindle Paperwhite 2 (2013, Wi-Fi), firmware 5.12.2.2: an i.MX6 SoloLite (ARMv7, Cortex-A9)
running Linux 3.0.35 with a soft-float (armel) userland and an old glibc. The panel is
758×1024 at 212 dpi, 16 gray levels.

It was prepared with community tools, none of which are in this repo:

- WinterBreak ([guide](https://kindlemodding.org/jailbreaking/WinterBreak/),
  [thread](https://www.mobileread.com/forums/showthread.php?t=365372)) to jailbreak it, with
  the OTA updater disabled afterwards so an update cannot undo it
- [KUAL](https://www.mobileread.com/forums/showthread.php?t=203326) and MRPI to install packages
- USBNetwork, from [NiLuJe's hacks](https://www.mobileread.com/forums/showthread.php?t=225030),
  which turns the USB port into an Ethernet link with an SSH server (dropbear) on the Kindle

Other models and firmware need different jailbreaks, and the panel ioctls differ between
generations (see [Drawing on the panel](#drawing-on-the-panel)).

## The USB link

With USBNetwork enabled the Kindle is `192.168.15.244` and the PC is `192.168.15.201` on a
`/24`. The Kindle's Wi-Fi stays off; everything goes through the cable.

### PC side

Three pieces, shown as they are deployed on the author's machine (Arch Linux with
NetworkManager and nftables).

A stable interface name, whatever USB port or gadget driver the Kindle comes up with, in
`/etc/systemd/network/60-homelab-kindle.link`:

```ini
[Match]
Property=ID_VENDOR=Linux_3.0.35-lab126_with_fsl-usb2-udc

[Link]
Name=kindle0
```

The PC's address on the link, never a default route, in
`/etc/NetworkManager/system-connections/homelab-kindle.nmconnection` (mode 0600):

```ini
[connection]
id=homelab-kindle
type=ethernet
interface-name=kindle0
autoconnect=true

[ipv4]
method=manual
address1=192.168.15.201/24
never-default=true

[ipv6]
method=disabled
```

IPv4 forwarding (`net.ipv4.ip_forward = 1`), and these rules in the input, forward and NAT
postrouting chains of an nftables `inet` table. The forward rule keeps the Kindle off the
author's Tailscale network; drop that clause if you have none.

```
iifname "kindle0" accept
iifname "kindle0" oifname != "tailscale0" accept
ip saddr 192.168.15.0/24 oifname != "kindle0" masquerade
```

### Kindle side

The Kindle needs a default route through the PC and DNS resolvers.
`kindle/upstart/kindle-uplink.conf` sets both every time USBNetwork starts (resolvers
`1.1.1.1` and `8.8.8.8`). Install it once:

```sh
host/install-uplink.sh
```

All the scripts reach the Kindle as `ssh kindle`, so add a host entry:

```
Host kindle
    HostName 192.168.15.244
    User root
    IdentityFile ~/.ssh/id_ed25519_kindle
    IdentitiesOnly yes
```

### Check it

```sh
host/verify-link.sh
```

It exits 0 only if all five checks pass. It sets nothing up itself, so a pass means the
setup holds on its own:

| Check | What it proves |
|---|---|
| `host_to_kindle` | the PC reaches the Kindle over SSH |
| `wifi_off` | the Kindle's Wi-Fi is off, so the remaining checks can only pass over USB |
| `default_route_usb0` | the Kindle's default route is the USB link |
| `kindle_to_host` | the Kindle fetches a random token the PC serves for this run |
| `internet_via_usb` | the Kindle fetches `https://example.com/` (or `$INTERNET_URL`) from its USB address |

## Building and running apps

The apps are cross-compiled for `armv7-unknown-linux-musleabi`: statically linked against
musl, so the Kindle's old glibc never matters, and soft-float to match its ABI.
`rust-toolchain.toml` installs the target, and `.cargo/config.toml` links with `rust-lld`,
so no ARM C toolchain is needed.

```sh
host/run-app.sh hello   # build, copy to /mnt/us/kindle-dev/ on the Kindle, run over ssh
cargo test              # tests run on the PC; nothing in them needs the device
```

`hello` draws a banner, the framebuffer geometry and a 16-step gray ramp.

## Claude Code usage

`usage` draws what Omarchy's agents panel shows for Claude Code: the plan, each rate
limit's meter and reset countdown, tokens per day for the last week (today in bold, with
its prompt and session counts), and the four heaviest models.

The page is landscape, for a Kindle standing on its long edge. The PW2 has no
accelerometer (its firmware's `accelerometer` property always reads `U`), so it cannot tell
which way it is turned. Say where the USB port is instead: `usage --port right` (the
default in `host/push-usage.sh`) or `--port left`, or set `KINDLE_PORT=left` for the push
script. The page is drawn at 1024×758 and turned in software (`Canvas::upright` in
`eink`) before it reaches the portrait framebuffer.

The PC pushes and the Kindle only draws:

```sh
host/push-usage.sh          # send the usage record to the Kindle and redraw
host/push-usage.sh --flash  # the same, with a full refresh that clears ghosting
```

- The data is Omarchy's own record, `~/.local/state/omarchy/agents/usage/claude.json`,
  which its collector rewrites about every 15 minutes. The record travels over SSH, so the
  Kindle needs no network code and the PC opens no port.
- The PC's clock is the only clock: `--now` carries its time and UTC offset, from which the
  Kindle works out the countdowns and which row is today.
- A systemd user timer runs the push every 5 minutes. Refreshes are quiet except in the
  first five minutes of each hour, when a full flash clears e-ink ghosting.
- The page stays up while the Kindle sits in its screensaver. Waking the Kindle gives the
  screen back to its own UI until the next push.

## Drawing on the panel

The panel is `/dev/fb0`: 8 bits per pixel, 0 black and 255 white, 758 visible pixels per row
in a 768-byte stride. Writing to it changes nothing you can see. The e-ink controller
redraws only when asked, with the `MXCFB_SEND_UPDATE` ioctl, and
`MXCFB_WAIT_FOR_UPDATE_COMPLETE` waits until it has finished.

These ioctls differ between Kindle generations. The PW2 takes the 72-byte
`mxcfb_update_data` and the 8-byte update marker from
[FBInk](https://github.com/NiLuJe/FBInk)'s `eink/mxcfb-kindle.h`. An ioctl number encodes
its struct's size, so compile-time asserts pin every struct size in `eink/src/epdc.rs`. That
file is the only place `unsafe` is allowed; the workspace denies it everywhere else.

## Tests

```sh
cargo test
cargo mutants
```

The drawing code is tested on the PC. `usage/golden/full.pgm` pins the dashboard's layout
pixel for pixel; after an intended layout change, look at the new page and rewrite it with
`UPDATE_GOLDEN=1 cargo test -p usage golden`. Mutation testing with
[cargo-mutants](https://mutants.rs) catches every viable mutant. `.cargo/mutants.toml`
leaves out only code that can run solely on the device: the ioctls and the programs' entry
points.

## Device notes

- Updates are blocked: `/usr/bin/otaupd` and `/usr/bin/otav3` are renamed to `*.bck`.
- USBNetwork's public package ships the same SSH host keys to everyone. Regenerate them on
  the device.
- USBNetwork starts at boot when `/mnt/us/usbnet/auto` exists. Turning it off in KUAL
  brings back USB drive mode.
- Toggle USBNetwork once and wait: toggles that overlap leave sshd stopped.
- An SSH session's `PATH` is only `/usr/bin:/bin`; call `/sbin/reboot` and
  `/usr/sbin/mntroot` by full path.
- The root filesystem is read-only: `mntroot rw` before changing anything under `/etc`,
  `mntroot ro` after.

## Credits

The jailbreak, KUAL, MRPI and USBNetwork come from the MobileRead Kindle community, and
NiLuJe's FBInk was the reference for the panel ioctls. Liberation Sans is © Google and Red
Hat under the SIL Open Font License; see `eink/assets/LiberationSans-LICENSE`.
