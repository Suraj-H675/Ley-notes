# Import a local project into a Brain

M2 imports a deliberately selected repository or folder into an existing Project Brain. The import runs locally
without Codex, Claude, a model, or a network provider. It retains filtered project text as Sources and immutable
SourceVersions. It does not adopt imported text as requirements or authorize model sharing.

## Create and attach

Create a Brain, then explicitly associate the selected folder. Use the returned project and locator IDs in later
commands. The Brain identity does not depend on the folder path.

```sh
ley brain create --name "My project" --request create-1 --json
ley brain attach --project PROJECT_ID --root /selected/project --request attach-1 --json
```

Attachment alone does not import content. A copied `.ley` marker, matching folder name, or Git remote cannot select
or authorize a Brain. These commands do not initialize the preceding continuity workflow or write repo markers.
Locators migrated from schema v12 have no recorded directory identity. Revoke them through the local core control
API and explicitly attach again before M2 import. Import does not infer their identity from the current pathname.

## Import and inspect

Supply the existing Brain, authorized working copy, and selected root explicitly.

```sh
ley brain import --project PROJECT_ID --locator LOCATOR_ID --root /selected/project --request import-1 --json
ley brain contents --project PROJECT_ID --locator LOCATOR_ID --json
```

Use the same request ID to replay an import. Use a new request ID to observe the folder again. Unchanged retained
representations reuse their SourceVersion; the new import records another observation. Changed files keep their
Source and acquire another version. Exact unambiguous original-content matches can preserve Source identity across
moves inside the folder. Ambiguous matches create new Sources.

Contents describes a recorded working-copy observation. It does not continuously check the filesystem. Missing,
omitted, and retained files have distinct states. A missing file retains its historical evidence. An ignored or
excluded file is not evidence of deletion. Separate working copies have separate observed inventories.
Explicitly removed Sources stay inactive. Their associated paths are disclosed as omissions during later imports;
unrelated eligible files still import. Removed ignore files remain exclusion policy without acquiring new versions.

Read historical retained evidence by its IDs rather than opening a current path:

```sh
ley brain versions --project PROJECT_ID --source SOURCE_ID --json
ley brain version --project PROJECT_ID --source SOURCE_ID --version VERSION_ID --json
ley brain version --project PROJECT_ID --source SOURCE_ID --version VERSION_ID --raw
```

The raw command returns the exact retained bytes. Version metadata identifies the retained representation and its
transformation. Redacted evidence is exact retained text, not exact original text.

## Move an attached folder

After moving the folder on disk, explicitly update its locator:

```sh
ley brain move-locator --project PROJECT_ID --locator LOCATOR_ID --root /new/project --request move-1 --json
ley brain import --project PROJECT_ID --locator LOCATOR_ID --root /new/project --request import-2 --json
```

The Brain identity survives the move. The old root must be absent, and the new root must preserve the recorded
directory identity. Copies and cross-filesystem replacements require separate explicit attachment. The engine
rejects conflicting or unsafe locator changes and does not infer authority from copied markers.

## Scope and failure

Import applies hard secret, dependency, generated-output, and private-state exclusions alongside local ignore rules.
Binary and unsupported text representations are omitted. Symlinks cannot expand the selected boundary. Omission
metadata explains skipped material. Redaction reduces recognizable credentials in eligible text before retention;
local import does not imply that any tracked or retained file is safe for future model egress.

The importer applies these fixed bounds:

| Work | Bound |
| --- | --- |
| Original or retained file | 1 MiB |
| Original and retained totals | 32 MiB each per scan |
| Directory entries | 20,000 per scan |
| Published inventory, including missing/omitted historical paths | 20,000 per working copy |
| Directory depth | 64 |
| Relative path | 4,096 bytes |
| Ignore file | 64 KiB each, 1 MiB total |

Oversized files are disclosed omissions. Total work and traversal failures abort publication. A failed eligible scan cannot publish a new successful inventory or partially committed SourceVersions. Inspect the latest attempt separately
from the last successful observation and retry after resolving the failure.

Git provenance records available repository/worktree, HEAD, branch, and uncommitted state. Retained working-copy
bytes are not assumed to equal committed HEAD bytes. Non-Git folders work through the same import contract.
Git observation uses metadata commands instead of `git status` or working-file diffs, which can execute configured
content filters. `dirty` conservatively reports staged differences or changed working-file metadata against the
index. Timestamp-only changes can therefore report dirty; `worktreeStateBasis` discloses that comparison.
Git subprocesses use the opened root through Linux procfs. When that capability binding is unavailable, including
on other platforms, import discloses unavailable Git provenance and still imports eligible local text. Directory
identity uses device/inode on Unix and volume/file ID on Windows, and fails closed when a stable ID is unavailable.

Complete Project Brain backup, Chronicle capture, derived knowledge, optional analysis, and redesigned Desktop
navigation remain later milestones. Portable continuity v1 rejects imported Brain state it cannot faithfully carry.
