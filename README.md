# Crossplane YAML

Crossplane-aware YAML support for Zed.

This extension adds:

- Crossplane package diagnostics
- Crossplane YAML syntax highlighting
- Go-template highlighting inside Crossplane `function-go-templating` inline templates
- YAML highlighting inside generated YAML sections of those templates

The extension manages its language server automatically. No separate setup is needed for normal use.

## Status

Crossplane YAML is waiting to be included in the Zed extension registry:

https://github.com/zed-industries/extensions/pull/6157

If you want to show interest, add a thumbs-up reaction to that PR. Please avoid comment-only "+1" messages.

## Installation

Until the marketplace PR is merged, install it as a local dev extension.

Install Rust with `rustup` if you do not already have it. Zed's dev-extension
workflow expects a rustup-managed Rust toolchain.

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

On macOS with Homebrew, install rustup with:

```sh
brew install rustup
$(brew --prefix rustup)/bin/rustup-init
```

Prefer `rustup` for this setup. Homebrew's `rust` formula installs a standalone
Rust toolchain, while `rustup` installs and manages the Rust toolchains Zed
expects for dev extensions.

Clone this repository:

```sh
git clone https://github.com/io41/crossplane-yaml.git
```

In Zed:

1. Open the command palette.
2. Run `zed: install dev extension`.
3. Select the cloned `crossplane-yaml` directory.
4. Restart Zed if the language does not appear immediately.

## Usage

Open a file classified as `Crossplane YAML`.

The extension recognizes common Crossplane package filenames, including:

- `crossplane.yaml`
- `crossplane.yml`
- `upbound.yaml`
- `upbound.yml`
- `composition.yaml`
- `composition.yml`
- `definition.yaml`
- `definition.yml`

Many Crossplane repositories use project-specific filenames, such as
`xexample-composition.yaml`, that Zed will otherwise open as regular YAML.

For a one-off file, use Zed's language selector in the status bar and choose
`Crossplane YAML`.

For a repository naming pattern, open Zed settings with `zed: open settings` and
add patterns under `file_types`:

```jsonc
{
  "file_types": {
    "Crossplane YAML": [
      "**/*-composition.yaml",
      "**/*-composition.yml",
      "**/*-definition.yaml",
      "**/*-definition.yml"
    ]
  }
}
```

Use patterns that only match Crossplane package files. For example, if all YAML
files under your package `api/` or `apis/` directories are Crossplane resources,
you can add directory-scoped patterns:

```jsonc
{
  "file_types": {
    "Crossplane YAML": [
      "**/api/**/*.yaml",
      "**/api/**/*.yml",
      "**/apis/**/*.yaml",
      "**/apis/**/*.yml"
    ]
  }
}
```

Reopen matching files after changing the setting.

The extension keeps Zed's normal YAML support enabled for ordinary YAML files.

## Template Highlighting

Crossplane `function-go-templating` templates often contain YAML inside Go-template blocks. Crossplane YAML highlights both the template expressions and the YAML they generate.

Some heavily templated YAML can still look imperfect when indentation, list items, or scalar values are split across template expressions.

## Troubleshooting

If the language does not appear after installing as a dev extension, restart Zed and make sure you selected the repository root containing `extension.toml`.

If a Crossplane file opens as regular YAML, add a `file_types` mapping for that filename pattern.

For development, release, and maintainer notes, see [DEVELOPMENT.md](DEVELOPMENT.md).

## License

MIT. See [LICENSE](LICENSE).
