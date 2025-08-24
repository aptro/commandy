use std::env;

pub struct ShellDetector;

impl ShellDetector {
    pub fn detect_shell() -> String {
        // Try to detect from SHELL environment variable
        if let Ok(shell) = env::var("SHELL") {
            if let Some(shell_name) = shell.split('/').next_back() {
                return shell_name.to_string();
            }
        }

        // Fallback detection methods
        if env::var("ZSH_VERSION").is_ok() {
            return "zsh".to_string();
        }

        if env::var("BASH_VERSION").is_ok() {
            return "bash".to_string();
        }

        // Default fallback
        "sh".to_string()
    }

    pub fn get_shell_config_file() -> Option<String> {
        let shell = Self::detect_shell();
        let home = env::var("HOME").ok()?;

        match shell.as_str() {
            "zsh" => Some(format!("{home}/.zshrc")),
            "bash" => {
                // Check for .bashrc first, then .bash_profile
                let bashrc = format!("{home}/.bashrc");
                let bash_profile = format!("{home}/.bash_profile");

                if std::path::Path::new(&bashrc).exists() {
                    Some(bashrc)
                } else {
                    Some(bash_profile)
                }
            }
            "fish" => Some(format!("{home}/.config/fish/config.fish")),
            _ => None,
        }
    }

    pub fn get_completion_script(&self, shell: &str) -> Option<String> {
        match shell {
            "bash" => Some(self.get_bash_completion()),
            "zsh" => Some(self.get_zsh_completion()),
            "fish" => Some(self.get_fish_completion()),
            _ => None,
        }
    }

    fn get_bash_completion(&self) -> String {
        r#"# Commandy bash completion
_commandy_complete() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    opts="init update config clear doctor version --help --explain --suggestions --no-cache --verbose"
    
    case ${prev} in
        commandy)
            COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
            return 0
            ;;
        update)
            COMPREPLY=( $(compgen -W "--model --binary" -- ${cur}) )
            return 0
            ;;
        clear)
            COMPREPLY=( $(compgen -W "--cache --context" -- ${cur}) )
            return 0
            ;;
        *)
            ;;
    esac
    
    COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
    return 0
}

complete -F _commandy_complete commandy
"#.to_string()
    }

    fn get_zsh_completion(&self) -> String {
        r#"# Commandy zsh completion
_commandy() {
    local context state state_descr line
    typeset -A opt_args
    
    _arguments \
        '1: :->commands' \
        '--explain[Show detailed explanations]' \
        '--suggestions[Number of suggestions to show]:number:' \
        '--no-cache[Skip cache and force fresh inference]' \
        '--verbose[Verbose output]' \
        '--help[Show help]' \
        '*: :->args'
    
    case $state in
        commands)
            local commands
            commands=(
                'init:Initialize commandy setup'
                'update:Update model or binary'
                'config:Show configuration'
                'clear:Clear cache and context'
                'doctor:Run diagnostics'
                'version:Show version information'
            )
            _describe 'commands' commands
            ;;
        args)
            case $words[2] in
                update)
                    _arguments \
                        '--model[Update the ML model]' \
                        '--binary[Update the binary]'
                    ;;
                clear)
                    _arguments \
                        '--cache[Clear command cache]' \
                        '--context[Clear learning context]'
                    ;;
            esac
            ;;
    esac
}

compdef _commandy commandy
"#
        .to_string()
    }

    fn get_fish_completion(&self) -> String {
        r#"# Commandy fish completion
complete -c commandy -f

# Main commands
complete -c commandy -n "not __fish_seen_subcommand_from init update config clear doctor version" -a "init" -d "Initialize commandy setup"
complete -c commandy -n "not __fish_seen_subcommand_from init update config clear doctor version" -a "update" -d "Update model or binary"
complete -c commandy -n "not __fish_seen_subcommand_from init update config clear doctor version" -a "config" -d "Show configuration"
complete -c commandy -n "not __fish_seen_subcommand_from init update config clear doctor version" -a "clear" -d "Clear cache and context"
complete -c commandy -n "not __fish_seen_subcommand_from init update config clear doctor version" -a "doctor" -d "Run diagnostics"
complete -c commandy -n "not __fish_seen_subcommand_from init update config clear doctor version" -a "version" -d "Show version information"

# Global options
complete -c commandy -l explain -d "Show detailed explanations"
complete -c commandy -l suggestions -d "Number of suggestions to show"
complete -c commandy -l no-cache -d "Skip cache and force fresh inference"
complete -c commandy -l verbose -d "Verbose output"
complete -c commandy -l help -d "Show help"

# Subcommand options
complete -c commandy -n "__fish_seen_subcommand_from update" -l model -d "Update the ML model"
complete -c commandy -n "__fish_seen_subcommand_from update" -l binary -d "Update the binary"
complete -c commandy -n "__fish_seen_subcommand_from clear" -l cache -d "Clear command cache"
complete -c commandy -n "__fish_seen_subcommand_from clear" -l context -d "Clear learning context"
"#.to_string()
    }
}
