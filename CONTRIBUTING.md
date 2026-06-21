# Contributing

Thanks for your interest in contributing to Realtime Call Translator!

## How to contribute

### Bug reports

Open an [issue](../../issues/new?template=bug_report.md) with:
- What you expected vs what happened
- Steps to reproduce
- macOS version, browser, audio setup

### Feature requests

Open an [issue](../../issues/new?template=feature_request.md) describing the use case and why it matters.

### Pull requests

1. Fork the repo
2. Create a branch: `git checkout -b feature/your-feature`
3. Make your changes
4. Test locally with a real call (Google Meet, Zoom, etc.)
5. Submit a PR with a clear description

### Development setup

```bash
git clone git@github.com:org-event/call-translator.git
cd call-translator
cargo run --release -p translator -- setup
cp .env.example .env   # optional: cloud API keys
cargo run --release -p translator
```

### Code style

- **Rust**: `cargo fmt` + `cargo clippy`

### What we especially welcome

- Windows / Linux audio backend support
- New TTS voice contributions
- Latency optimizations
- Documentation improvements
- Bug fixes with reproduction steps

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
