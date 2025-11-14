# Build Requirements

## Required Tools

### 1. Rust (1.70+)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2. Protocol Buffer Compiler (protoc)

**CRITICAL**: This is required to build the project!

#### Ubuntu/Debian
```bash
sudo apt-get update
sudo apt-get install -y protobuf-compiler
```

#### macOS
```bash
brew install protobuf
```

#### From Source
```bash
# Download from https://github.com/protocolbuffers/protobuf/releases
wget https://github.com/protocolbuffers/protobuf/releases/download/v25.1/protoc-25.1-linux-x86_64.zip
unzip protoc-25.1-linux-x86_64.zip -d $HOME/.local
export PATH="$PATH:$HOME/.local/bin"
```

#### Verify Installation
```bash
protoc --version
# Should output: libprotoc 3.x.x or higher
```

### 3. Build Essentials

#### Ubuntu/Debian
```bash
sudo apt-get install -y build-essential pkg-config libssl-dev
```

#### macOS
```bash
xcode-select --install
brew install openssl
```

## Building

Once all requirements are installed:

```bash
cd rust-fe

# Check that protoc is available
which protoc

# Build
cargo build --release

# Run
cargo run --release
```

## Common Build Errors

### Error: "Could not find `protoc`"

**Solution**: Install protobuf-compiler as shown above

```bash
# Ubuntu/Debian
sudo apt-get install protobuf-compiler

# macOS
brew install protobuf

# Verify
protoc --version
```

### Error: "could not find libssl"

**Solution**: Install OpenSSL development files

```bash
# Ubuntu/Debian
sudo apt-get install libssl-dev pkg-config

# macOS
brew install openssl
export PKG_CONFIG_PATH="/usr/local/opt/openssl/lib/pkgconfig"
```

### Error: Linking errors

**Solution**: Install build-essential

```bash
# Ubuntu/Debian
sudo apt-get install build-essential

# macOS
xcode-select --install
```

## Skip Proto Compilation (Not Recommended)

If you cannot install protoc and just want to see the code structure:

```bash
SKIP_PROTO=1 cargo check
```

**Warning**: This will not produce a working binary. BE communication will not function.

## Docker Build (Recommended for Consistency)

To avoid dependency issues, use Docker:

```dockerfile
FROM rust:1.75

RUN apt-get update && apt-get install -y \\
    protobuf-compiler \\
    build-essential \\
    pkg-config \\
    libssl-dev

WORKDIR /app
COPY . .

RUN cargo build --release

CMD ["./target/release/doris-rust-fe"]
```

Build and run:

```bash
docker build -t doris-rust-fe .
docker run -p 9030:9030 -p 8030:8030 doris-rust-fe
```

## Development Dependencies

For development (testing, linting):

```bash
# Format code
cargo fmt

# Lint
cargo clippy --all-targets --all-features

# Run tests
cargo test
```

## Minimum Versions

- Rust: 1.70+
- protoc: 3.12+
- cargo: 1.70+

## Platform Support

- ✅ Linux (x86_64, aarch64)
- ✅ macOS (x86_64, Apple Silicon)
- ⚠️ Windows (should work but untested)

## Next Steps

After successful build, see:
- [QUICKSTART.md](QUICKSTART.md) - Quick start guide
- [README.md](README.md) - Full documentation
- [ARCHITECTURE.md](ARCHITECTURE.md) - Architecture details
