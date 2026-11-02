//! The graphics modes of `pmset`, what the menu says about them and the sudoers rule that allows
//! switching without a password. Without system calls, so the rules are tested.

/// `pmset gpuswitch`: which graphics card macOS may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Only the integrated GPU (Intel): cool and quiet, no external monitors on most MacBooks.
    Integrated,
    /// Only the discrete GPU (NVIDIA or AMD).
    Discrete,
    /// macOS switches by itself, the discrete GPU as soon as an app asks for it.
    Automatic,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Integrated, Mode::Automatic, Mode::Discrete];

    pub fn value(self) -> u8 {
        match self {
            Mode::Integrated => 0,
            Mode::Discrete => 1,
            Mode::Automatic => 2,
        }
    }

    pub fn from_value(value: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.value() == value)
    }
}

/// The mode from the output of `pmset -g` (" gpuswitch            2").
pub fn parse_pmset(text: &str) -> Option<Mode> {
    text.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        (words.next()? == "gpuswitch").then_some(())?;
        Mode::from_value(words.next()?.parse().ok()?)
    })
}

/// The graphics card the built-in screen gets its picture from.
#[derive(Debug, Clone, PartialEq)]
pub struct Gpu {
    pub name: String,
    /// Metal's `isLowPower`: the integrated GPU.
    pub integrated: bool,
}

/// The card does not match the mode (yet): macOS has not switched, an app or a monitor holds the
/// discrete GPU, or the change applies only after logging out.
pub fn mismatch(mode: Mode, gpu: Option<&Gpu>) -> bool {
    match (mode, gpu) {
        (Mode::Integrated, Some(gpu)) => !gpu.integrated,
        (Mode::Discrete, Some(gpu)) => gpu.integrated,
        _ => false,
    }
}

/// A user name that can go into a sudoers rule and a shell command as it is.
pub fn valid_user(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('-') && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

pub const PMSET: &str = "/usr/bin/pmset";

/// Allows exactly the three `pmset` calls, nothing else.
pub fn sudoers_rule(user: &str) -> String {
    let commands: Vec<String> = Mode::ALL
        .iter()
        .map(|m| format!("{PMSET} -a gpuswitch {}", m.value()))
        .collect();
    format!(
        "# GPUYield: switch the graphics card without a password\n{user} ALL=(root) NOPASSWD: {}\n",
        commands.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const PMSET_OUTPUT: &str = "System-wide power settings:\nCurrently in use:\n standby              1\n \
                                gpuswitch            2\n sleep                1 (sleep prevented by backupd)\n";

    #[test]
    fn the_mode_is_read_from_pmset() {
        assert_eq!(parse_pmset(PMSET_OUTPUT), Some(Mode::Automatic));
        assert_eq!(parse_pmset(" gpuswitch 0"), Some(Mode::Integrated));
        assert_eq!(parse_pmset(" gpuswitch 1"), Some(Mode::Discrete));
        assert_eq!(parse_pmset(" gpuswitch 7"), None);
        assert_eq!(parse_pmset(" sleep 1\n"), None, "Macs with one GPU have no gpuswitch");
        assert_eq!(parse_pmset(""), None);
    }

    #[test]
    fn values_round_trip() {
        for mode in Mode::ALL {
            assert_eq!(Mode::from_value(mode.value()), Some(mode));
        }
    }

    #[test]
    fn a_card_that_does_not_fit_the_mode_is_noticed() {
        let intel = Gpu {
            name: "Intel HD Graphics 4000".into(),
            integrated: true,
        };
        let nvidia = Gpu {
            name: "NVIDIA GeForce GT 650M".into(),
            integrated: false,
        };
        assert!(mismatch(Mode::Integrated, Some(&nvidia)));
        assert!(!mismatch(Mode::Integrated, Some(&intel)));
        assert!(mismatch(Mode::Discrete, Some(&intel)));
        assert!(!mismatch(Mode::Automatic, Some(&nvidia)));
        assert!(!mismatch(Mode::Integrated, None));
    }

    #[test]
    fn only_plain_user_names_get_a_rule() {
        assert!(valid_user("markus"));
        assert!(valid_user("m.eckert_2"));
        for bad in ["", "-x", "a b", "root ALL", "a'b", "a;b", "ä"] {
            assert!(!valid_user(bad), "{bad:?}");
        }
    }

    #[test]
    fn the_rule_allows_only_the_three_pmset_calls() {
        let rule = sudoers_rule("markus");
        assert!(rule.ends_with('\n'), "sudo ignores a last line without newline");
        assert!(rule.contains(
            "markus ALL=(root) NOPASSWD: /usr/bin/pmset -a gpuswitch 0, /usr/bin/pmset -a gpuswitch 2, \
             /usr/bin/pmset -a gpuswitch 1\n"
        ));
        assert!(!rule.contains('*'));
    }
}
