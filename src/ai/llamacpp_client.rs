use anyhow::{Context, Result};
use log::{debug, info, warn};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::cli::Suggestion;
use crate::config::Settings;
use crate::context::ContextData;

/// Client for interacting with llama.cpp binary for local inference
pub struct LlamaCppClient {
    binary_path: PathBuf,
    model_name: String,
    max_tokens: u32,
    temperature: f32,
}

impl LlamaCppClient {
    /// Creates a new LlamaCppClient instance with configuration from settings
    pub fn new(settings: &Settings) -> Result<Self> {
        let binary_path = Self::detect_binary_path()?;
        let model_name = settings.model.model_path.clone(); // Repurpose for model name
        let max_tokens = settings.model.max_tokens;
        let temperature = settings.model.temperature;

        Ok(Self {
            binary_path,
            model_name,
            max_tokens,
            temperature,
        })
    }

    /// Detects the llama.cpp binary path in the system
    fn detect_binary_path() -> Result<PathBuf> {
        // First, try the local installation path
        let home_dir = dirs::home_dir().context("Could not find home directory")?;
        let local_binary = home_dir.join(".commandy").join("bin").join("llama-cpp");
        
        if local_binary.exists() {
            return Ok(local_binary);
        }

        // Try Windows executable extension
        let local_binary_exe = home_dir.join(".commandy").join("bin").join("llama-cpp.exe");
        if local_binary_exe.exists() {
            return Ok(local_binary_exe);
        }

        // Try system PATH
        if let Ok(output) = Command::new("which").arg("llama-cpp").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout);
                let path_str = path_str.trim();
                if !path_str.is_empty() {
                    return Ok(PathBuf::from(path_str));
                }
            }
        }

        // Try common system locations
        let system_paths = [
            "/usr/local/bin/llama-cpp",
            "/usr/bin/llama-cpp",
            "/opt/llama-cpp/bin/llama-cpp",
        ];

        for path in &system_paths {
            let path_buf = PathBuf::from(path);
            if path_buf.exists() {
                return Ok(path_buf);
            }
        }

        Err(anyhow::anyhow!(
            "llama.cpp binary not found. Please run 'commandy init' to install it."
        ))
    }

    /// Verifies that the llama.cpp binary is working
    pub async fn verify_connection(&self) -> Result<()> {
        debug!("Verifying llama.cpp binary at {:?}", self.binary_path);

        let output = Command::new(&self.binary_path)
            .arg("--version")
            .output()
            .context("Failed to execute llama.cpp binary")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "llama.cpp binary test failed: {}",
                stderr
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        info!("llama.cpp binary verified: {}", stdout.lines().next().unwrap_or("unknown version"));
        Ok(())
    }

    /// Generates command suggestions based on user prompt and context
    pub async fn generate_suggestions(
        &self,
        prompt: &str,
        context: &ContextData,
        max_suggestions: usize,
    ) -> Result<Vec<Suggestion>> {
        debug!("Generating suggestions for prompt: {prompt}");

        let enhanced_prompt = self.build_enhanced_prompt(prompt, context);
        let response = self.generate_text(&enhanced_prompt).await?;
        let suggestions = self.parse_response(&response, max_suggestions);

        info!("Generated {} suggestions", suggestions.len());
        Ok(suggestions)
    }

    /// Executes llama.cpp binary with the given prompt and returns the response
    async fn generate_text(&self, prompt: &str) -> Result<String> {
        debug!("Executing llama.cpp with prompt length: {}", prompt.len());

        let mut command = Command::new(&self.binary_path);
        command
            .arg("-hf")
            .arg(&self.model_name)
            .arg("-c")
            .arg("0") // Use full context
            .arg("-fa") // Flash attention
            .arg("-p")
            .arg(prompt)
            .arg("-n")
            .arg(self.max_tokens.to_string())
            .arg("--temp")
            .arg(self.temperature.to_string())
            .arg("--no-display-prompt") // Don't echo the prompt
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!("Executing command: {:?}", command);

        let output = command.output().context("Failed to execute llama.cpp")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "llama.cpp execution failed: {}",
                stderr
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let response = stdout.trim().to_string();

        debug!("Generated response length: {}", response.len());
        Ok(response)
    }

    /// Builds an enhanced prompt with context information for better command generation
    fn build_enhanced_prompt(&self, user_prompt: &str, context: &ContextData) -> String {
        let environment = &context.environment;
        let recent_commands = &context.recent_commands;
        let context_content = &context.content;

        let available_tools = environment
            .get("available_tools")
            .map_or("basic".to_string(), |v| {
                v.split(',').take(20).collect::<Vec<_>>().join(", ")
            });

        let mut prompt = format!(
            r#"Generate ONLY valid shell commands for: {}

System Information:
- OS: {}
- Shell: {}
- Available executables: {}
- Recent commands: {}

CRITICAL REQUIREMENTS:
1. Commands MUST use only executables that exist in PATH
2. Start with real command names, not pseudo-commands
3. Use proper shell syntax
4. Be directly executable
5. Provide safe, practical solutions

Output format: Return 1-3 shell commands, each on a new line.
Example format:
docker ps -a
ls -la /var/log
grep -r "error" /var/log/

Commands for: {}"#,
            user_prompt,
            environment.get("os").map_or("unknown", |v| v.as_str()),
            environment.get("shell").map_or("unknown", |v| v.as_str()),
            available_tools,
            recent_commands
                .iter()
                .take(3)
                .map(|cmd| cmd.split_whitespace().next().unwrap_or(""))
                .collect::<Vec<_>>()
                .join(", "),
            user_prompt
        );

        // Add learned context if available
        if !context_content.is_empty() {
            let relevant_patterns: Vec<&str> = context_content
                .lines()
                .filter(|line| line.contains("→") || line.contains("✓"))
                .take(5)
                .collect();

            if !relevant_patterns.is_empty() {
                prompt.push_str("\n\nLearned patterns:\n");
                prompt.push_str(&relevant_patterns.join("\n"));
            }
        }

        prompt.push_str("\n\nCommands:");
        prompt
    }

    /// Parses the response from llama.cpp and extracts valid command suggestions
    fn parse_response(&self, response: &str, max_suggestions: usize) -> Vec<Suggestion> {
        debug!("Parsing response: {}", response);

        let mut suggestions = Vec::new();
        
        // Split response into lines and extract potential commands
        for line in response.lines() {
            let line = line.trim();
            
            // Skip empty lines, comments, or lines that are too long
            if line.is_empty() || line.starts_with('#') || line.len() > 300 {
                continue;
            }

            // Skip explanatory text (look for lines that start with command words)
            if self.looks_like_command(line) && self.is_valid_command(line) {
                suggestions.push(Suggestion {
                    command: line.to_string(),
                    explanation: None, // Could be enhanced to extract explanations
                    confidence: 0.8,
                });

                if suggestions.len() >= max_suggestions {
                    break;
                }
            }
        }

        // If no commands found, try to extract from longer text
        if suggestions.is_empty() {
            suggestions = self.extract_commands_fallback(response, max_suggestions);
        }

        suggestions
    }

    /// Fallback method to extract commands when primary parsing fails
    fn extract_commands_fallback(&self, response: &str, max_suggestions: usize) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        
        // Look for command-like patterns in the text
        let words: Vec<&str> = response.split_whitespace().collect();
        let mut current_command = String::new();
        
        for word in words {
            if word.len() > 100 {
                continue; // Skip very long words
            }
            
            // Look for command starters
            if self.is_command_starter(word) {
                if !current_command.is_empty() && self.is_valid_command(&current_command) {
                    suggestions.push(Suggestion {
                        command: current_command.trim().to_string(),
                        explanation: None,
                        confidence: 0.6,
                    });
                    
                    if suggestions.len() >= max_suggestions {
                        break;
                    }
                }
                current_command = word.to_string();
            } else if !current_command.is_empty() {
                current_command.push(' ');
                current_command.push_str(word);
                
                // Stop at sentence endings
                if word.ends_with('.') || word.ends_with('!') || word.ends_with('?') {
                    if self.is_valid_command(&current_command) {
                        suggestions.push(Suggestion {
                            command: current_command.trim_end_matches(&['.', '!', '?']).to_string(),
                            explanation: None,
                            confidence: 0.6,
                        });
                        
                        if suggestions.len() >= max_suggestions {
                            break;
                        }
                    }
                    current_command.clear();
                }
            }
        }
        
        // Handle last command if any
        if !current_command.is_empty() && self.is_valid_command(&current_command) {
            suggestions.push(Suggestion {
                command: current_command.trim().to_string(),
                explanation: None,
                confidence: 0.6,
            });
        }
        
        suggestions
    }

    /// Checks if a word could be the start of a command
    fn is_command_starter(&self, word: &str) -> bool {
        matches!(
            word.trim_start_matches(|c: char| c.is_ascii_punctuation()),
            "ls" | "cd" | "grep" | "find" | "docker" | "kubectl" | "git" | "curl" | "wget" |
            "ssh" | "sudo" | "cp" | "mv" | "rm" | "cat" | "tail" | "head" | "ps" | "kill" |
            "top" | "df" | "du" | "tar" | "zip" | "unzip" | "chmod" | "chown" | "systemctl" |
            "service" | "apt" | "yum" | "npm" | "yarn" | "pip" | "cargo" | "make" | "cmake" |
            "rsync" | "scp" | "awk" | "sed" | "sort" | "uniq" | "cut" | "tr" | "xargs"
        )
    }

    /// Checks if a line looks like a shell command
    fn looks_like_command(&self, line: &str) -> bool {
        let first_word = line.split_whitespace().next().unwrap_or("");
        
        // Check if it starts with a known command
        if self.is_command_starter(first_word) {
            return true;
        }
        
        // Check for command-like patterns
        line.contains("--") || line.contains("-") && line.split_whitespace().count() > 1
    }

    /// Validates that a command is safe and executable
    fn is_valid_command(&self, command: &str) -> bool {
        // Basic safety checks
        let dangerous_patterns = ["rm -rf /", "rm -rf *", "dd if=", "mkfs", "fdisk", "> /dev/"];
        
        for pattern in &dangerous_patterns {
            if command.contains(pattern) {
                warn!("Rejected dangerous command: {}", command);
                return false;
            }
        }

        // Check length and basic format
        if command.is_empty() || command.len() > 500 {
            return false;
        }

        // Extract the executable name
        let first_word = command.split_whitespace().next().unwrap_or("").trim();
        
        if first_word.is_empty() || first_word.starts_with('#') {
            return false;
        }

        // Check if executable exists using 'which' command
        if let Ok(output) = Command::new("which").arg(first_word).output() {
            if output.status.success() {
                return true;
            }
        }

        // Allow shell built-ins and paths
        if first_word.contains('/') || matches!(first_word, "cd" | "echo" | "pwd" | "export" | "alias") {
            return true;
        }

        // Reject pseudo-commands
        let pseudo_patterns = [" query ", " api ", " endpoint ", " service "];
        for pattern in &pseudo_patterns {
            if command.to_lowercase().contains(pattern) {
                return false;
            }
        }

        debug!("Command '{}' not found in PATH", first_word);
        false
    }
}