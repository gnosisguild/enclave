# Git Hooks

This directory contains optional git hooks that can help with development workflow.

## commit-msg

This hook validates that commit messages follow the [Conventional Commits](https://www.conventionalcommits.org/) specification.

### Enable the hook

To enable commit message validation locally:

```bash
chmod +x .githooks/commit-msg
git config core.hooksPath .githooks
```

### What it validates

The hook ensures commit messages follow this format:

```
<type>[optional scope]: <description>
```

**Valid types:** feat, fix, chore

**Examples:**

- `feat: add new encryption provider`
- `fix(cli): resolve template initialization bug`
- `chore(ci): update GitHub Actions`
- `feat!: breaking change to API`

### Disabling the hook

To disable the hook:

```bash
git config --unset core.hooksPath
```

## Note

These hooks are optional for local development. All pull requests are automatically validated by GitHub Actions regardless of whether you use local hooks.
