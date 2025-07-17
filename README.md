# ygg - Yggdrasil GitHub Grep

**ygg** is a fast, concurrent Rust CLI tool for searching and grepping package versions or strings across GitHub repositories. It dynamically queries GitHub's API or uses repo lists to fetch and analyze files (e.g., package-lock.json) in parallel.

## Features
- Audit versions of a specific NPM package across repos.
- Search for strings in specified files across repos.
- Supports dynamic repo discovery via GitHub code search or static repo lists.
- Caches results for efficiency.

### GitHub Personal Access Token (GHP_TOKEN)
Ygg requires a GitHub PAT to authenticate API requests. Set it as an environment variable: `export GHP_TOKEN=your_token_here`.

#### Creating the Token
1. Go to [https://github.com/settings/tokens](https://github.com/settings/tokens) and generate a new classic token.
2. Required scopes:
   - `repo`: Full control of private repositories (includes access to contents).
   - `read:org`: Read org and team membership, read org projects.
3. If your organization uses SSO (SAML single sign-on), enable SSO for the token and authorize it for your organization(s).

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
