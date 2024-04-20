# Inference

## VS Code setup

### Extensions
- rust-analyzer
- Even Better TOML
- crates

### Configuration

Enable proc macros

```json
"rust-analyzer.procMacro.enable": true,
```

Default linter

```json
"rust-analyzer.check.command": "clippy",
```

Format on save

```json
"[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer",
    "editor.formatOnSave": true
}
```
