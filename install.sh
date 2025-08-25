#!/bin/bash
set -e

# Commandy Unified Installation Script
# Handles OS detection, Ollama installation, and Commandy setup

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${PURPLE}[STEP]${NC} $1"
}

# Configuration
COMMANDY_VERSION="latest"
COMMANDY_DIR="$HOME/.commandy"
GITHUB_REPO="aptro/commandy"
INSTALL_DIR="$HOME/.local/bin"
LLAMA_CPP_REPO="ggml-org/llama.cpp"
GEMMA_MODEL="ggml-org/gemma-3-270m-GGUF"

# Global variables
OS=""
ARCH=""
PLATFORM=""

# Display banner
show_banner() {
    echo -e "${PURPLE}"
    cat << 'EOF'
  ____  _     _                       
 |  _ \| |__ | | ___   ___ _ __ ___  
 | |_) | '_ \| |/ _ \ / _ \ '_ ` _ \ 
 |  __/| | | | | (_) |  __/ | | | | |
 |_|   |_| |_|_|\___/ \___|_| |_| |_|
         Secure, fast command suggestions using local models
EOF
    echo -e "${NC}"
    echo "Installing Commandy with Ollama integration..."
    echo ""
}

# Detect OS and architecture
detect_platform() {
    log_step "Detecting platform..."
    
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)
    
    case "$OS" in
        "darwin")
            OS="macos"
            ;;
        "linux")
            OS="linux"
            ;;
        *)
            log_error "Unsupported operating system: $OS"
            exit 1
            ;;
    esac
    
    case "$ARCH" in
        "x86_64"|"amd64")
            ARCH="x86_64"
            ;;
        "arm64"|"aarch64")
            ARCH="aarch64"
            ;;
        *)
            log_error "Unsupported architecture: $ARCH"
            exit 1
            ;;
    esac
    
    # Set platform for binary download
    case "$OS-$ARCH" in
        "macos-x86_64")
            PLATFORM="x86_64-apple-darwin"
            ;;
        "macos-aarch64")
            PLATFORM="aarch64-apple-darwin"
            ;;
        "linux-x86_64")
            PLATFORM="x86_64-unknown-linux-gnu"
            ;;
        "linux-aarch64")
            PLATFORM="aarch64-unknown-linux-gnu"
            ;;
        *)
            log_error "Unsupported platform combination: $OS-$ARCH"
            exit 1
            ;;
    esac
    
    log_success "Platform detected: $OS ($ARCH) -> $PLATFORM"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check prerequisites
check_prerequisites() {
    log_step "Checking prerequisites..."
    
    # Check curl
    if ! command_exists curl; then
        log_error "curl is required but not installed"
        
        case "$OS" in
            "macos")
                log_info "Install curl with: brew install curl"
                ;;
            "linux")
                log_info "Install curl with: sudo apt-get install curl (Ubuntu/Debian) or sudo yum install curl (RHEL/CentOS)"
                ;;
        esac
        exit 1
    fi
    
    # Create local bin directory if it doesn't exist
    if [ ! -d "$INSTALL_DIR" ]; then
        log_info "Creating local bin directory: $INSTALL_DIR"
        mkdir -p "$INSTALL_DIR"
    fi
    
    log_success "Prerequisites check passed"
}

# Install llama.cpp binary
install_llamacpp() {
    log_step "Installing llama.cpp..."
    
    local bin_dir="$COMMANDY_DIR/bin"
    local binary_name="llama-cpp"
    local binary_path="$bin_dir/$binary_name"
    
    # Add .exe extension for Windows (future support)
    if [[ "$OS" == "windows" ]]; then
        binary_name="llama-cpp.exe"
        binary_path="$bin_dir/$binary_name"
    fi
    
    # Check if already installed
    if [ -f "$binary_path" ]; then
        log_info "llama.cpp already installed at $binary_path"
        if "$binary_path" --version >/dev/null 2>&1; then
            log_success "llama.cpp binary is working"
            return 0
        else
            log_warning "Existing binary not working, reinstalling..."
        fi
    fi
    
    # Create bin directory
    mkdir -p "$bin_dir"
    
    # Determine download URL based on platform
    local download_url
    case "$PLATFORM" in
        "x86_64-apple-darwin")
            download_url="https://github.com/$LLAMA_CPP_REPO/releases/download/b6265/llama-b6265-bin-macos-x64.zip"
            ;;
        "aarch64-apple-darwin")
            download_url="https://github.com/$LLAMA_CPP_REPO/releases/download/b6265/llama-b6265-bin-macos-arm64.zip"
            ;;
        "x86_64-unknown-linux-gnu")
            download_url="https://github.com/$LLAMA_CPP_REPO/releases/download/b6265/llama-b6265-bin-ubuntu-x64.zip"
            ;;
        *)
            log_error "Unsupported platform for llama.cpp: $PLATFORM"
            exit 1
            ;;
    esac
    
    log_info "Downloading llama.cpp from: $download_url"
    
    local temp_file=$(mktemp)
    local temp_dir=$(mktemp -d)
    
    # Download the zip file
    if ! curl -L --fail --progress-bar "$download_url" -o "$temp_file"; then
        log_error "Failed to download llama.cpp"
        rm -f "$temp_file"
        rm -rf "$temp_dir"
        exit 1
    fi
    
    # Extract the zip file
    if ! unzip -q "$temp_file" -d "$temp_dir"; then
        log_error "Failed to extract llama.cpp archive"
        rm -f "$temp_file"
        rm -rf "$temp_dir"
        exit 1
    fi
    
    # Find the main binary (llama-cli in the build/bin directory)
    local extracted_binary=""
    for binary in "$temp_dir/build/bin/llama-cli" "$temp_dir/llama-cli" "$temp_dir/main" "$temp_dir/build/bin/main"; do
        if [ -f "$binary" ]; then
            extracted_binary="$binary"
            break
        fi
    done
    
    if [ -z "$extracted_binary" ]; then
        log_error "Could not find llama.cpp binary in archive"
        rm -f "$temp_file"
        rm -rf "$temp_dir"
        exit 1
    fi
    
    # Copy to our bin directory
    cp "$extracted_binary" "$binary_path"
    chmod +x "$binary_path"
    
    # Copy required shared libraries
    local lib_dir="${extracted_binary%/*}"  # Get directory of the binary
    if [ -d "$lib_dir" ]; then
        log_info "Copying shared libraries..."
        for lib in "$lib_dir"/*.dylib "$lib_dir"/*.so; do
            if [ -f "$lib" ]; then
                cp "$lib" "$bin_dir/"
            fi
        done
    fi
    
    # Cleanup
    rm -f "$temp_file"
    rm -rf "$temp_dir"
    
    # Verify installation
    if ! "$binary_path" --version >/dev/null 2>&1; then
        log_error "llama.cpp binary installation verification failed"
        exit 1
    fi
    
    log_success "llama.cpp installed successfully to $binary_path"
}

# Initialize commandy directory structure
initialize_commandy_directories() {
    log_step "Creating commandy directory structure..."
    
    # Create all necessary directories
    local subdirs=("cache" "models" "logs" "backups" "bin")
    for subdir in "${subdirs[@]}"; do
        mkdir -p "$COMMANDY_DIR/$subdir"
    done
    
    log_success "Directory structure created"
}

# Verify llama.cpp installation and model
verify_llamacpp_setup() {
    log_step "Verifying llama.cpp setup..."
    
    local binary_path="$COMMANDY_DIR/bin/llama-cpp"
    if [[ "$OS" == "windows" ]]; then
        binary_path="$COMMANDY_DIR/bin/llama-cpp.exe"
    fi
    
    # Test binary
    if ! [ -f "$binary_path" ]; then
        log_error "llama.cpp binary not found at $binary_path"
        return 1
    fi
    
    # Test binary execution
    if ! "$binary_path" --version >/dev/null 2>&1; then
        log_error "llama.cpp binary test failed"
        return 1
    fi
    
    log_info "Using Gemma 3 270M model: $GEMMA_MODEL"
    log_info "The model will be downloaded automatically on first use"
    log_success "llama.cpp setup verified"
}

# Download and install Commandy binary
install_commandy_binary() {
    log_step "Installing Commandy binary..."
    
    # Check if already installed
    if command_exists commandy; then
        local current_version
        current_version=$(commandy --version 2>/dev/null | head -n1 || echo "unknown")
        log_info "Commandy already installed: $current_version"
        
        read -p "Do you want to reinstall? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_info "Skipping binary installation"
            return 0
        fi
    fi
    
    # Create install directory if needed
    if [ ! -d "$INSTALL_DIR" ]; then
        log_info "Creating install directory: $INSTALL_DIR"
        mkdir -p "$INSTALL_DIR" || {
            log_error "Failed to create install directory"
            exit 1
        }
    fi
    
    # Determine download URL
    local binary_url="https://github.com/$GITHUB_REPO/releases/$COMMANDY_VERSION/download/commandy-$PLATFORM"
    local temp_file
    temp_file=$(mktemp)
    
    log_info "Downloading Commandy binary from GitHub..."
    log_info "URL: $binary_url"
    
    if ! curl -L --fail --progress-bar "$binary_url" -o "$temp_file"; then
        log_error "Failed to download Commandy binary"
        log_info "Please check if the release exists at: https://github.com/$GITHUB_REPO/releases"
        rm -f "$temp_file"
        exit 1
    fi
    
    # Install binary
    log_info "Installing binary to $INSTALL_DIR/commandy"
    
    mv "$temp_file" "$INSTALL_DIR/commandy"
    chmod +x "$INSTALL_DIR/commandy"
    
    # Verify installation
    if ! "$INSTALL_DIR/commandy" --version >/dev/null 2>&1; then
        log_error "Binary installation verification failed"
        exit 1
    fi
    
    log_success "Commandy binary installed successfully"
}

# Initialize Commandy
initialize_commandy() {
    log_step "Initializing Commandy..."
    
    # Run commandy init
    if command_exists commandy; then
        commandy init || {
            log_warning "Commandy initialization failed, but continuing..."
        }
        log_success "Commandy initialized"
    else
        log_warning "Commandy binary not found in PATH, skipping initialization"
        log_info "You may need to add $INSTALL_DIR to your PATH"
    fi
}

# Setup shell integration
setup_shell_integration() {
    log_step "Setting up shell integration..."
    
    # Detect shell
    local shell_name
    shell_name=$(basename "$SHELL" 2>/dev/null || echo "bash")
    
    local rc_file=""
    case "$shell_name" in
        "bash")
            if [[ "$OS" == "macos" ]]; then
                rc_file="$HOME/.bash_profile"
            else
                rc_file="$HOME/.bashrc"
            fi
            ;;
        "zsh")
            rc_file="$HOME/.zshrc"
            ;;
        "fish")
            rc_file="$HOME/.config/fish/config.fish"
            mkdir -p "$(dirname "$rc_file")"
            ;;
        *)
            log_warning "Shell integration not available for $shell_name"
            return 0
            ;;
    esac
    
    # Check if PATH update is needed
    if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
        log_info "Adding $INSTALL_DIR to PATH in $rc_file"
        
        # Add PATH export
        echo "" >> "$rc_file"
        echo "# Commandy" >> "$rc_file"
        echo "export PATH=\"$INSTALL_DIR:\$PATH\"" >> "$rc_file"
        
        log_success "Shell integration added to $rc_file"
        log_warning "Please restart your shell or run: source $rc_file"
    else
        log_info "$INSTALL_DIR already in PATH"
    fi
}

# Run health check
health_check() {
    log_step "Running health check..."
    
    local issues=0
    
    # Check binary
    if ! command_exists commandy; then
        log_error "❌ Commandy binary not found in PATH"
        issues=$((issues + 1))
    else
        log_success "✅ Commandy binary accessible"
    fi
    
    # Check llama.cpp binary
    local llamacpp_binary="$COMMANDY_DIR/bin/llama-cpp"
    if [[ "$OS" == "windows" ]]; then
        llamacpp_binary="$COMMANDY_DIR/bin/llama-cpp.exe"
    fi
    
    if ! [ -f "$llamacpp_binary" ]; then
        log_error "❌ llama.cpp binary not found at $llamacpp_binary"
        log_info "Run installation again to download llama.cpp"
        issues=$((issues + 1))
    elif ! "$llamacpp_binary" --version >/dev/null 2>&1; then
        log_error "❌ llama.cpp binary not working"
        log_info "Try reinstalling with the installation script"
        issues=$((issues + 1))
    else
        log_success "✅ llama.cpp binary working"
    fi
    
    # Model info (no need to check as it downloads automatically)
    log_success "✅ Using Gemma 3 270M model ($GEMMA_MODEL)"
    log_info "Model will be downloaded automatically on first use"
    
    # Check directory
    if [ ! -d "$COMMANDY_DIR" ]; then
        log_warning "⚠️  Commandy directory not initialized"
        log_info "Run: commandy init"
    else
        log_success "✅ Commandy directory exists"
    fi
    
    if [ $issues -eq 0 ]; then
        log_success "All health checks passed!"
    else
        log_warning "Found $issues issue(s) that may need attention"
    fi
    
    return $issues
}

# Show completion message
show_completion() {
    echo ""
    log_success "🎉 Commandy installation complete!"
    echo ""
    echo -e "${BLUE}Quick Start:${NC}"
    echo "  commandy \"list running processes\""
    echo "  commandy \"find large files\""
    echo "  commandy --explain \"git commit with message\""
    echo ""
    echo -e "${BLUE}Useful Commands:${NC}"
    echo "  commandy doctor          # Check system health"
    echo "  commandy --help          # Show help"
    echo "  commandy config          # Show configuration"
    echo ""
    
    if ! command_exists commandy; then
        echo -e "${YELLOW}Note:${NC} You may need to restart your shell or run:"
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        echo ""
    fi
    
    echo -e "${PURPLE}Documentation:${NC} https://github.com/$GITHUB_REPO"
    echo -e "${PURPLE}Issues:${NC} https://github.com/$GITHUB_REPO/issues"
}

# Cleanup on error
cleanup() {
    if [ $? -ne 0 ]; then
        log_error "Installation failed!"
        log_info "You can report issues at: https://github.com/$GITHUB_REPO/issues"
    fi
}

# Main installation function
main() {
    # Set up error handling
    trap cleanup EXIT
    
    show_banner
    detect_platform
    check_prerequisites
    initialize_commandy_directories
    install_llamacpp
    verify_llamacpp_setup
    install_commandy_binary
    initialize_commandy
    setup_shell_integration
    
    echo ""
    if health_check; then
        show_completion
    else
        log_warning "Installation completed with some issues"
        log_info "Run 'commandy doctor' for more details"
        show_completion
    fi
}

# Run main function
main "$@"