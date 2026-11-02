//! Switching the mode needs root. With the sudoers rule (`/etc/sudoers.d/gpuyield`, allows only
//! the three `pmset` calls) it goes through `sudo -n` without a question; without the rule macOS
//! asks for an administrator password each time. Installing and removing the rule asks once.

use std::path::Path;
use std::process::Command;

use crate::i18n::tr;
use crate::model::{self, Mode};

const RULE: &str = "/etc/sudoers.d/gpuyield";

/// The user cancelled the password dialog.
pub const CANCELLED: &str = "cancelled";

pub fn passwordless() -> bool {
    Path::new(RULE).exists()
}

fn user() -> Option<String> {
    unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if pw.is_null() {
            return None;
        }
        let name = std::ffi::CStr::from_ptr((*pw).pw_name).to_string_lossy().into_owned();
        model::valid_user(&name).then_some(name)
    }
}

fn pmset_command(mode: Mode) -> String {
    format!("{} -a gpuswitch {}", model::PMSET, mode.value())
}

/// Sets the mode for battery and power adapter. Blocks while the password dialog is open.
pub fn set_mode(mode: Mode) -> Result<(), String> {
    if passwordless() {
        let status = Command::new("/usr/bin/sudo")
            .args(["-n", model::PMSET, "-a", "gpuswitch", &mode.value().to_string()])
            .status();
        if status.is_ok_and(|s| s.success()) {
            return Ok(());
        }
        crate::log!("sudo -n refused, asking for the password instead");
    }
    as_admin(&pmset_command(mode), tr("GPUYield wants to switch the graphics card."))
}

/// Installs the rule (checked by `visudo` first) or removes it.
pub fn set_passwordless(on: bool) -> Result<(), String> {
    if !on {
        return as_admin(
            &format!("/bin/rm -f {RULE}"),
            tr("GPUYield wants to remove its sudoers rule."),
        );
    }
    let user = user().ok_or("user name not usable in a sudoers rule")?;
    let lines: Vec<String> = model::sudoers_rule(&user).lines().map(|l| format!("'{l}'")).collect();
    let script = format!(
        "set -e; tmp=$(/usr/bin/mktemp /tmp/gpuyield.XXXXXX); \
         /usr/bin/printf '%s\\n' {} > \"$tmp\"; \
         /usr/sbin/visudo -cf \"$tmp\" >/dev/null; \
         /usr/bin/install -m 0440 -o root -g wheel \"$tmp\" {RULE}; /bin/rm -f \"$tmp\"",
        lines.join(" ")
    );
    as_admin(
        &script,
        tr("GPUYield wants to allow switching the graphics card without a password."),
    )
}

/// Runs `script` as root after macOS asked for an administrator password. The script goes in as
/// an argument, so nothing in it needs AppleScript quoting.
fn as_admin(script: &str, prompt: &str) -> Result<(), String> {
    let output = Command::new("/usr/bin/osascript")
        .args([
            "-e",
            "on run argv",
            "-e",
            "do shell script (item 1 of argv) with prompt (item 2 of argv) with administrator privileges",
            "-e",
            "end run",
            "--",
            script,
            prompt,
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
    // -128: "User canceled."
    if err.contains("-128") {
        Err(CANCELLED.into())
    } else {
        Err(err)
    }
}
