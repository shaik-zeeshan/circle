# Circle

A screen and audio recording application that allows users to search and rewind through their history by continuously capturing their screen and audio - all stored locally for privacy.

## Architecture

Circle is built as a multi-language application with the following stack:

- **Backend**: Rust - High-performance screen capture, audio recording, and media encoding
- **Frontend**: TypeScript/React - Web interface for searching, viewing, and managing recordings

## Project Structure

```
circle/
├── Cargo.toml          # Rust workspace configuration
├── crates/             # Rust crates
│   ├── capture/        # Screen and audio capture functionality
│   ├── cli/           # Command-line interface
│   └── encoder/       # Media encoding and processing
├── packages/          # Frontend packages (if needed)
├── apps/              # Frontend applications
└── LICENSE           # MIT License
```

## Rust Crates

### `capture`
Library responsible for:
- Screen capture (multi-platform support)
- Audio recording from system/microphone
- Real-time data streaming
- Memory-efficient buffering

### `cli`
Command-line interface for:
- Starting/stopping recording
- Configuration management
- System status and diagnostics
- Direct control of recording sessions

### `encoder`
Media processing library for:
- Video encoding (H.264, HEVC, etc.)
- Audio encoding (AAC, Opus, etc.)
- Compression and optimization
- Format conversion and streaming

## Development

### Prerequisites
- Rust (latest stable)
- Node.js & npm/pnpm/yarn
- Platform-specific recording permissions

### Building

```bash
# Build all Rust crates
cargo build --workspace

# Build specific crate
cargo build -p capture
cargo build -p cli
cargo build -p encoder
```

### Running

```bash
# Run CLI tool
cargo run -p cli -- --help
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Roadmap

- [ ] Core screen capture functionality
- [ ] Audio recording capabilities
- [ ] Real-time encoding
- [ ] Web-based search interface
- [ ] Timeline navigation
- [ ] Multi-platform support (Windows, macOS, Linux)
- [ ] Local storage optimization
- [ ] Privacy and security features