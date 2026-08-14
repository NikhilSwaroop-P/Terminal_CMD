//! Shell integration hook generators for Bash, Zsh, and POSIX shells.
//!
//! Injects non-destructive OSC 133 semantic prompt markers and OSC 7
//! dynamic working directory tracking.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Shell integration script content for Bash.
pub const BASH_INTEGRATION: &str = r#"
__termcmd_prompt_start() {
    printf "\033]133;A\007"
    printf "\033]7;file://%s%s\007" "${HOSTNAME:-localhost}" "$PWD"
}
__termcmd_preexec() {
    local ret=$?
    printf "\033]133;C\007"
    return $ret
}
__termcmd_postexec() {
    local exit_code=$?
    printf "\033]133;D;%d\007" "$exit_code"
}
PROMPT_COMMAND="__termcmd_postexec; __termcmd_prompt_start; ${PROMPT_COMMAND:-}"
PS1="\[\033]133;B\007\]$PS1"
trap '__termcmd_preexec' DEBUG
"#;

/// Shell integration script content for Zsh.
pub const ZSH_INTEGRATION: &str = r#"
if [ -n "$TERMCMD_ORIG_ZDOTDIR" ]; then
    export ZDOTDIR="$TERMCMD_ORIG_ZDOTDIR"
else
    unset ZDOTDIR
fi
if [ -f "$HOME/.zshrc" ]; then
    source "$HOME/.zshrc"
fi
__termcmd_precmd() {
    local exit_code=$?
    printf "\033]133;D;%d\007" "$exit_code"
    printf "\033]7;file://%s%s\007" "${HOST:-localhost}" "$PWD"
    printf "\033]133;A\007"
}
__termcmd_preexec() {
    printf "\033]133;C\007"
}
autoload -Uz add-zsh-hook 2>/dev/null || true
add-zsh-hook precmd __termcmd_precmd 2>/dev/null || true
add-zsh-hook preexec __termcmd_preexec 2>/dev/null || true
PS1=$'%{\e]133;B\a%}'"$PS1"
"#;

/// Generic POSIX shell fallback integration.
pub const POSIX_INTEGRATION: &str = r#"
__termcmd_prompt() {
    local exit_code=$?
    printf "\033]133;D;%d\007\033]7;file://localhost%s\007\033]133;A\007" "$exit_code" "$PWD"
}
PS1='$(__termcmd_prompt)'"$PS1"
"#;

/// Detected shell family for integration configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellType {
    Bash,
    Zsh,
    Sh,
    Fish,
    Other,
}

impl ShellType {
    /// Detects shell type from command path or binary name.
    pub fn detect(shell_path: &str) -> Self {
        let name = Path::new(shell_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(shell_path);

        if name.contains("bash") {
            ShellType::Bash
        } else if name.contains("zsh") {
            ShellType::Zsh
        } else if name.contains("fish") {
            ShellType::Fish
        } else if name.contains("sh") {
            ShellType::Sh
        } else {
            ShellType::Other
        }
    }
}

/// Ephemeral shell initialization artifact holder.
pub enum ShellInit {
    BashFile(tempfile::NamedTempFile),
    ZshDir(tempfile::TempDir),
}

/// Generates an ephemeral initialization script or directory for the target shell type.
pub fn create_init_environment(shell_type: ShellType) -> std::io::Result<Option<ShellInit>> {
    match shell_type {
        ShellType::Bash => {
            let mut file = tempfile::Builder::new()
                .prefix("termcmd_bash_init_")
                .suffix(".sh")
                .tempfile()?;

            let user_home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
            let bashrc_path = PathBuf::from(&user_home).join(".bashrc");

            if bashrc_path.exists() {
                writeln!(file, "if [ -f \"{}\" ]; then . \"{}\"; fi", bashrc_path.display(), bashrc_path.display())?;
            } else if Path::new("/etc/bash.bashrc").exists() {
                writeln!(file, ". /etc/bash.bashrc")?;
            }

            writeln!(file, "{}", BASH_INTEGRATION)?;
            file.flush()?;
            Ok(Some(ShellInit::BashFile(file)))
        }
        ShellType::Zsh => {
            let temp_dir = tempfile::Builder::new()
                .prefix("termcmd_zsh_init_")
                .tempdir()?;

            let zshrc_path = temp_dir.path().join(".zshrc");
            let mut file = std::fs::File::create(zshrc_path)?;
            writeln!(file, "{}", ZSH_INTEGRATION)?;
            file.flush()?;

            Ok(Some(ShellInit::ZshDir(temp_dir)))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_shell_detection() {
        assert_eq!(ShellType::detect("/bin/bash"), ShellType::Bash);
        assert_eq!(ShellType::detect("/usr/bin/zsh"), ShellType::Zsh);
        assert_eq!(ShellType::detect("fish"), ShellType::Fish);
        assert_eq!(ShellType::detect("/bin/sh"), ShellType::Sh);
        assert_eq!(ShellType::detect("/usr/local/bin/nu"), ShellType::Other);
    }

    #[test]
    fn test_bash_init_script_generation() {
        let init = create_init_environment(ShellType::Bash).expect("create init");
        assert!(init.is_some());
        if let Some(ShellInit::BashFile(file)) = init {
            let content = fs::read_to_string(file.path()).expect("read script");
            assert!(content.contains("__termcmd_prompt_start"));
            assert!(content.contains("133;A"));
        } else {
            panic!("Expected BashFile");
        }
    }

    #[test]
    fn test_zsh_init_dir_generation() {
        let init = create_init_environment(ShellType::Zsh).expect("create init");
        assert!(init.is_some());
        if let Some(ShellInit::ZshDir(dir)) = init {
            let zshrc = dir.path().join(".zshrc");
            assert!(zshrc.exists());
            let content = fs::read_to_string(zshrc).expect("read zshrc");
            assert!(content.contains("__termcmd_precmd"));
            assert!(content.contains("add-zsh-hook"));
        } else {
            panic!("Expected ZshDir");
        }
    }
}
