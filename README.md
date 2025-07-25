# ygg - Yggdrasil GitHub Grep

**ygg** is a fast, concurrent Rust CLI tool for searching and grepping package versions or strings across GitHub repositories. It dynamically queries GitHub's API or uses repo lists to fetch and analyze files (e.g., package-lock.json) in parallel.

Use ygg to:
- Dynamically search for repositories using GitHub's code search API (--query).
- Audit NPM package versions in package-lock.json files (--package).
- Search for custom strings in specified files (--filename and --search).

Modes:
- Package audit mode: Use --package to check versions in package-lock.json (default file).
- String search mode: Use --filename and --search to find strings in custom files.
- Repo listing: If no mode is specified, lists repositories from --repos or --query.

Requires GHP_TOKEN environment variable for GitHub authentication.

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
ygg (Yggdrasil GitHub Grep): Grep GitHub repos to audit NPM package versions or search strings in specified files

Usage: ygg [OPTIONS]

Options:
  -r, --repos <REPOS>        Path to a JSON file containing a list of repositories (e.g., ["org/repo1", "org/repo2"]) [default: repos.json]
  -q, --query <QUERY>        GitHub code search query to dynamically discover repositories (e.g., "language:javascript path:package.json")
  -o, --org <ORG>            GitHub organization to scope the search (e.g., "myorg")
  -p, --package <PACKAGE>    NPM package name to audit versions for in package-lock.json files (e.g., "lodash")
  -f, --filename <FILENAME>  Filename to fetch from each repository (e.g., "config.yaml")
  -s, --search <SEARCH>      String to search for within the fetched file content (e.g., "secret_key")
  -c, --clear-cache          Clear the local cache before fetching files from GitHub
  -h, --help                 Print help (see more with '--help')
  -V, --version              Print version
```
