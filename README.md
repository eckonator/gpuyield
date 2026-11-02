# GPUYield

A small menu bar app for Intel MacBooks with two graphics cards that switches the discrete GPU
(NVIDIA or AMD) off and on again – without typing `sudo pmset -a gpuswitch …` in Terminal.

Made for an older MacBook Pro that runs hot: the discrete GPU shares its heat sink with the CPU,
so letting the Intel GPU draw keeps the whole machine cooler and quieter.

The menu offers three modes:

- **Intel only (cool and quiet)**: the discrete GPU stays off (`pmset -a gpuswitch 0`).
- **Automatic**: macOS switches by itself, to the discrete GPU as soon as an app asks for it
  (`pmset -a gpuswitch 2`, the default of macOS).
- **NVIDIA only (full power)**: the discrete GPU is always on (`pmset -a gpuswitch 1`).

The mode applies on battery and on the power adapter and stays set when GPUYield quits; it belongs
to macOS, not to the app.

- The top of the menu shows the graphics card that actually draws the picture right now, not only
  the setting. In the menu bar the core of the chip is filled while the discrete GPU is on and
  empty while the Intel GPU draws.
- If the card does not follow the mode, the menu says so. macOS may keep the discrete GPU until
  the apps holding it end, or until you log out and in again.
- On most of these MacBooks (for example the MacBook Pro Retina 2012–2014) the external display
  ports are wired to the discrete GPU: with "Intel only", external monitors stay dark. The menu
  reminds you of this.
- The menu is in German or English, after the first preferred language of the system (System
  Settings → General → Language & Region); a change applies at the next start of GPUYield.

Intel Macs with two graphics cards only (macOS 10.15 or newer, including macOS versions installed
with OpenCore Legacy Patcher). On Macs with one graphics card the menu says so.

## Switching without a password

Changing the mode needs administrator rights, so macOS asks for the password each time. "Switch
without password" in the menu asks once and installs `/etc/sudoers.d/gpuyield`, which allows
exactly these three commands for the current user and nothing else:

```
/usr/bin/pmset -a gpuswitch 0, /usr/bin/pmset -a gpuswitch 2, /usr/bin/pmset -a gpuswitch 1
```

The rule is checked with `visudo` before it is installed. Choosing the menu item again removes it.

## Install

Download the disk image (`GPUYield-<version>-macOS.dmg`) from the
[releases](https://github.com/eckonator/gpuyield/releases) and drag GPUYield to Applications.

GPUYield is not notarized by Apple. On the first start macOS refuses it once: open System Settings
→ Privacy & Security and choose "Open Anyway".

## Releases

Every push to `main` builds the app on GitHub. If the `version` in `Cargo.toml` has no release yet,
the build becomes the release `v<version>`; otherwise it replaces the pre-release `testing`. So for
a new release, raise the version in `Cargo.toml` and push.

## Files

- Log: `~/Library/Logs/GPUYield/gpuyield.log`
- Start at login: `~/Library/LaunchAgents/de.eckonator.gpuyield.plist`
- Password-free switching: `/etc/sudoers.d/gpuyield`

## Build

```sh
cargo test
scripts/bundle-macos.sh   # dist/GPUYield.app
scripts/make-dmg.sh       # dist/GPUYield-<version>-macOS.dmg
```

## License

GPL-3.0-or-later, see [LICENSE](LICENSE).
