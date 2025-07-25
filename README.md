# ygg - Yggdrasil GitHub Grep

**ygg** is a fast, concurrent Rust CLI tool for searching and grepping package versions or strings across GitHub repositories. It dynamically queries GitHub's API or uses repo lists to fetch and analyze files (e.g., package-lock.json) in parallel.

## Features
- Audit versions of a specific NPM package across repos.
- Search for strings in specified files across repos.
- Supports dynamic repo discovery via GitHub code search or static repo lists.
- Caches results for efficiency.

### GitHub Personal Access Token (GHP_TOKEN)
**ygg** requires a GitHub PAT to authenticate API requests. Set it as an environment variable: `export GHP_TOKEN=your_token_here`.

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

Full options:

```sh
❯ ygg -h
Ygg (Yggdrasil GitHub Grep): Grep GitHub repos to audit NPM lockfile versions or search custom strings in files

Usage: ygg [OPTIONS]

Options:
  -r, --repos <REPOS>        Path of the file containing json list of repositories (required unless --query is provided) [default: repos.json]
  -q, --query <QUERY>        Search query for GitHub code search (if provided, searches for repos dynamically instead of using --repos)
  -o, --org <ORG>            Organization name for code search (used with --query)
  -p, --package <PACKAGE>    Package name to check versions on (required for package-lock mode)
  -f, --filename <FILENAME>  Optional filename to fetch and search inside (if provided, performs string search instead of package-lock parsing)
  -s, --search <SEARCH>      Search string to find in the file content (required for string search mode)
  -c, --clear-cache          Clear cache to force fetch from GitHub
  -h, --help                 Print help
  -V, --version              Print version
```
