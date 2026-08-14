//! Shell integration hook generators for Bash, Zsh, Fish, and POSIX shells.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Shell integration script content for Bash.
pub const BASH_INTEGRATION: &str = r#"
__termcmd_preexec() {
    if [ -n "${__termcmd_in_prompt:-}" ]; then
        return 0
    fi
    if [ -z "${__termcmd_executed:-}" ]; then
        __termcmd_executed=1
        printf "\033]133;C\007"
    fi
    return 0
}

__termcmd_prompt() {
    local exit_code=$?
    printf "\033]133;D;%d\007" "$exit_code"
    printf "\033]7;file://%s%s\007" "${HOSTNAME:-localhost}" "$PWD"
    printf "\033]133;A\007"
    __termcmd_in_prompt=1
    __termcmd_executed=
}

__termcmd_prompt_end() {
    __termcmd_in_prompt=
}

PROMPT_COMMAND='__termcmd_prompt'
PS1="\[\033]133;B\007\$(__termcmd_prompt_end)\]$PS1"
trap '__termcmd_preexec' DEBUG
"#;

/// Shell integration script content for Zsh.
pub const ZSH_INTEGRATION: &str = r#"
autoload -Uz add-zsh-hook

__termcmd_precmd() {
    local exit_code=$?
    printf "\033]133;D;%d\007" "$exit_code"
    printf "\033]7;file://%s%s\007" "$HOST" "$PWD"
    printf "\033]133;A\007"
}

__termcmd_preexec() {
    printf "\033]133;C\007"
}

add-zsh-hook precmd __termcmd_precmd
add-zsh-hook preexec __termcmd_preexec
"#;

/// Shell integration script content for Fish.
pub const FISH_INTEGRATION: &str = r#"
function __termcmd_sync_dir_history --on-variable PWD
    set -l dir_hash (echo -n "$PWD" | md5sum 2>/dev/null | cut -d" " -f1)
    if test -z "$dir_hash"
        return
    end
    set -l dir_dir "$HOME/.local/share/termcmd/dir_history"
    mkdir -p "$dir_dir"
    set -l dir_file "$dir_dir/$dir_hash"

    for item in (history search --prefix "./" 2>/dev/null) (history search --prefix "../" 2>/dev/null) (history search --prefix "cd " 2>/dev/null)
        if string match -qr "^cd\s+(?![/~-])" -- $item
            set -l target (string replace -r "^cd\s+" "" -- $item)
            if not test -d "$target"
                history delete --exact --case-sensitive "$item" 2>/dev/null
            end
        else if string match -qr "^(\./|\.\./)" -- $item
            set -l target (string match -r "^[^\s]+" -- $item)
            if not test -e "$target"
                history delete --exact --case-sensitive "$item" 2>/dev/null
            end
        end
    end

    if test -f "$dir_file"
        while read -l line
            if test -n "$line"
                history append "$line" 2>/dev/null
            end
        end < "$dir_file"
    end
end

function __termcmd_prompt_enter --on-event fish_prompt
    set -g __termcmd_in_prompt 1
end

function __termcmd_preexec --on-event fish_preexec
    if not set -q __termcmd_in_prompt
        printf "\033]133;C\007"
    end
end

function __termcmd_postexec --on-event fish_postexec
    set -l last_status $status
    set -l cmd "$argv[1]"
    printf "\033]133;D;%d\007" $last_status
    if string match -qr "^(\./|\.\./|cd\s+(?![/~-]))|(\s\./|\s\.\./)" -- $cmd
        set -l dir_hash (echo -n "$PWD" | md5sum 2>/dev/null | cut -d" " -f1)
        if test -n "$dir_hash"
            set -l dir_dir "$HOME/.local/share/termcmd/dir_history"
            mkdir -p "$dir_dir"
            set -l dir_file "$dir_dir/$dir_hash"
            if test -f "$dir_file"
                if not grep -F -x "$cmd" "$dir_file" >/dev/null 2>&1
                    echo "$cmd" >> "$dir_file"
                end
            else
                echo "$cmd" >> "$dir_file"
            end
        end
    end
end

function __termcmd_prompt_render --on-event fish_postexec
    set -e __termcmd_in_prompt
    printf "\033]133;A\007"
    printf "\033]7;file://%s%s\007" (hostname) (pwd)
    if not set -q __termcmd_dir_synced
        set -g __termcmd_dir_synced 1
        __termcmd_sync_dir_history
    end
end

if not set -q __termcmd_dir_synced
    set -g __termcmd_dir_synced 1
    __termcmd_sync_dir_history
    printf "\033]133;A\007"
    printf "\033]7;file://%s%s\007" (hostname) (pwd)
end
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

        if name == "bash" || name.starts_with("bash") {
            ShellType::Bash
        } else if name == "zsh" || name.starts_with("zsh") {
            ShellType::Zsh
        } else if name == "fish" || name.starts_with("fish") {
            ShellType::Fish
        } else if name == "sh" || name == "dash" || name == "ash" {
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
    FishFile(tempfile::NamedTempFile),
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
        ShellType::Fish => {
            let mut file = tempfile::Builder::new()
                .prefix("termcmd_fish_init_")
                .suffix(".fish")
                .tempfile()?;

            writeln!(file, "{}", FISH_INTEGRATION)?;
            file.flush()?;
            Ok(Some(ShellInit::FishFile(file)))
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
        assert_eq!(ShellType::detect("/usr/bin/fish"), ShellType::Fish);
        assert_eq!(ShellType::detect("/bin/sh"), ShellType::Sh);
        assert_eq!(ShellType::detect("/bin/unknown_shell"), ShellType::Other);
    }

    #[test]
    fn test_bash_init_script_generation() {
        let init = create_init_environment(ShellType::Bash).expect("create init");
        assert!(init.is_some());
        if let Some(ShellInit::BashFile(file)) = init {
            let content = fs::read_to_string(file.path()).expect("read script");
            assert!(content.contains("__termcmd_prompt"));
            assert!(content.contains("133;A"));
        } else {
            panic!("Expected BashFile");
        }
    }

    #[test]
    fn test_fish_init_script_generation() {
        let init = create_init_environment(ShellType::Fish).expect("create init");
        assert!(init.is_some());
        if let Some(ShellInit::FishFile(file)) = init {
            let content = fs::read_to_string(file.path()).expect("read script");
            assert!(content.contains("__termcmd_prompt"));
        } else {
            panic!("Expected FishFile");
        }
    }

    #[test]
    fn test_zsh_init_dir_generation() {
        let init = create_init_environment(ShellType::Zsh).expect("create init");
        assert!(init.is_some());
        if let Some(ShellInit::ZshDir(dir)) = init {
            let zshrc = dir.path().join(".zshrc");
            assert!(zshrc.exists());
            let content = fs::read_to_string(zshrc).expect("read script");
            assert!(content.contains("add-zsh-hook"));
        } else {
            panic!("Expected ZshDir");
        }
    }
}
