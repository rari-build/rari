## Highlights

- Short, user-facing summary of the most important changes
- Prefer outcomes over commit subjects

## Breaking Changes

- List removals, renames, and required upgrades
- Include migration hints when possible

<!--
File naming (checked in order):
  1. --notes-file / RELEASE_NOTES_FILE
  2. .github/release-notes/<tag>.md
     `/` in scoped tags is replaced with `-` for the filename
     e.g. rari@0.15.0.md, v0.15.0.md, @rari-use-cache@0.15.0.md
  3. .github/release-notes/<version>.md
     e.g. 0.15.0.md (shared across release units)

Copy this template to one of those names before running `just release`.
Manual notes are prepended to git-cliff output for GitHub releases and
injected under the version heading in CHANGELOG.md.
-->
