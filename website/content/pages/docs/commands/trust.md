---

title: jbx trust command
description: Pin, list, remove, or clear trusted hashes for remote scripts.
---

# `trust`

Pin, list, remove, or clear trusted hashes for remote scripts.

## When to use it

- Pin a reviewed remote script before automation runs it.
- Rotate trust after a remote script intentionally changes.
- Audit which URLs are allowed to run without prompting.

## Common workflows

```bash
jbx trust list
jbx trust add https://example.com/tool.java
jbx trust remove https://example.com/tool.java
```

## Real-life examples

### Pin a reviewed remote script

```bash
jbx trust list
jbx trust add https://example.com/tool.java
jbx trust list
```

Use `trust` when automation must run a remote script repeatedly and the source has been reviewed.

### Agent loop

1. List current trust entries before adding or removing anything.
2. Review the remote script content outside the trust command.
3. Add or remove exactly one URL/hash entry.
4. Re-list and then run the remote script only if execution is approved.

## Agent notes

Trust changes are security changes. Do not add trust for convenience when a local checked-in script would be safer.

## Verification checklist

- Trust list contains the expected URL/hash after the change.
- Removed entries no longer appear.
- Remote script content was reviewed before trust was granted.

## Arguments and flags

This section is copied from the CLI help for this release so the page explains the actual accepted arguments.

### `jbx trust`

```text
Manage trusted remote scripts

Usage: jbx trust <COMMAND>

Commands:
  add     Trust the current content hash of a remote script URL
  remove  Remove a trusted remote script URL
  list    List trusted remote script URLs and hashes
  clear   Clear all trusted remote script entries
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### `jbx trust list`

```text
List trusted remote script URLs and hashes

Usage: jbx trust list [OPTIONS]

Options:
      --cache-dir <CACHE_DIR>  Override cache directory
  -h, --help                   Print help
```

### `jbx trust add`

```text
Trust the current content hash of a remote script URL

Usage: jbx trust add [OPTIONS] <URL>

Arguments:
  <URL>  Remote http(s) Java source URL

Options:
      --cache-dir <CACHE_DIR>  Override cache directory
  -h, --help                   Print help
```

### `jbx trust remove`

```text
Remove a trusted remote script URL

Usage: jbx trust remove [OPTIONS] <URL>

Arguments:
  <URL>  Remote http(s) Java source URL

Options:
      --cache-dir <CACHE_DIR>  Override cache directory
  -h, --help                   Print help
```

### `jbx trust clear`

```text
Clear all trusted remote script entries

Usage: jbx trust clear [OPTIONS]

Options:
      --cache-dir <CACHE_DIR>  Override cache directory
  -h, --help                   Print help
```

## Skill

Agents can fetch the matching versioned skill with:

```bash
jbx skill get jbx-trust
```
