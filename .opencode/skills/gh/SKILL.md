---
name: gh
description: |
  Use ONLY when the user asks about GitHub operations — viewing PRs, issues, reviews, comments, diffs, CI checks, or managing repos.
  Covers `gh` CLI usage patterns, authentication troubleshooting, and GitHub API access.
---

# GitHub CLI (gh) Skill

## Overview

`gh` is the official GitHub CLI. It wraps both the REST API and GraphQL API.
Key commands for code review workflows:

- `gh pr view <number>` — show PR summary
- `gh pr view <number> --json <fields>` — structured PR data
- `gh pr diff <number>` — show PR diff
- `gh pr review <number> --json <fields>` — get review comments
- `gh api <endpoint>` — direct API access

## Authentication

- `gh auth status` — check current login
- `gh auth token` — print the active token (use with `curl -H "Authorization: token $(gh auth token)"`)

Fine-grained PATs (`github_pat_...`) require explicit repo access. If API
returns 404 but git works, the token lacks repo scope — fall back to
`curl` with SSH for git operations or re-authenticate with a classic PAT
that has `repo` scope.

## Viewing PRs when API is restricted

When `gh pr view` fails with "Could not resolve to a Repository" but
`git ls-remote origin` shows `refs/pull/N/head`, the PR exists but the
token lacks API access. Workarounds:

1. Fetch the PR branch: `git fetch origin refs/pull/N/head:pr-N`
2. Review the diff locally: `git diff dev..pr-N`
3. List changed files: `git diff --stat dev..pr-N`
4. Show commit messages: `git log dev..pr-N --oneline`

## Viewing reviews/comments from fetched PRs

Use `gh api` with the correct endpoint if the token has access:
```
gh api repos/<owner>/<repo>/pulls/<number>/reviews
gh api repos/<owner>/<repo>/pulls/<number>/comments
gh api repos/<owner>/<repo>/pulls/<number>/files
```

If the token doesn't have access, use the web interface or ask the user
to share the review content.
