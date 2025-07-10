# ygg - Yggdrasil GitHub Grep

**ygg** is a fast, concurrent Rust CLI tool for searching and grepping package versions or strings across GitHub repositories. It dynamically queries GitHub's API or uses repo lists to fetch and analyze files (e.g., package-lock.json) in parallel.

### Features
- **Dynamic Repo Search**: Find repos via GitHub code search (e.g., by org and query).
- **Package Version Checking**: Extract versions from `package-lock.json` with semver sorting.
- **String Grep Mode**: Search for arbitrary strings in custom files.
- **Caching & Concurrency**: Parallel requests with local caching for efficiency.
- **GitHub Auth**: Uses `GHP_TOKEN` for authenticated API access.

### Installation
```sh
cargo install ygg
```

### Usage
Search for a package version across repos:
```sh
ygg --package "lodash" --query "lodash path:package-lock.json" --org "my-org"
```

Grep a string in a custom file:
```sh
ygg --filename "config.yaml" --search "enable-feature: true" --repos "repos.json"
```

Force refresh cache:
```sh
ygg --package "react" --force-fetch
```

For full options: `ygg --help`.
