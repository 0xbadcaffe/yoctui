Read AGENTS.md first and follow it as the execution contract.

Then inspect:

1. docs/current-task.md
2. docs/task-registry.toml
3. docs/implementation-status.md
4. docs/architecture.md
5. docs/ui-spec.md
6. docs/product-roadmap.md
7. scripts/verify-completion.sh
8. scripts/verify-roadmap.sh
9. the existing BitBake capability/version-correlation implementation
10. daemon/client protocol and job infrastructure
11. embedded PTY/session infrastructure
12. Recipes metadata/inventory
13. existing utility-workbench implementation
14. existing command palette and popup editor
15. existing persistence/configuration infrastructure

Also inspect the BitBake cheatsheet supplied with this task.

The supplied file is:

bitbake_cheatsheet_yocto_wrynose_6.0_bitbake_2.18(1).md

It is an authoritative project reference for the Raw Mode feature, but it is specifically a Wrynose 6.0 / BitBake 2.18 reference.

FIRST ACTION

Copy the supplied cheatsheet into the Yoctui repository under an appropriate documentation/reference path.

Prefer a stable name such as:

docs/reference/bitbake-cheatsheet-wrynose-6.0-bitbake-2.18.md

Do not modify or simplify its contents merely to make parsing easier.

Preserve it as a human-readable reference artifact.

Add documentation explaining:

* its source/version scope
* that it is a reference snapshot
* that runtime Raw Mode availability is determined by the connected BitBake environment, not blindly by this document

The cheatsheet must remain tracked in the repository.

==================================================
FEATURE: RAW MODE

Introduce a new Yoctui workspace called:

Raw Mode

Raw Mode provides expert-level access to the BitBake CLI command surface.

It is intended for users who know BitBake and want direct access to commands and options without leaving Yoctui.

Raw Mode is NOT:

* an arbitrary shell-command launcher
* a replacement for the embedded native terminal
* a string passed to /bin/sh -c
* a way around Yoctui safety checks
* a static assumption that every BitBake release supports every command in the Wrynose cheatsheet

Raw Mode IS:

* a structured browser over BitBake command templates
* organized according to the reference cheatsheet
* dynamically correlated with the active BitBake/Yocto environment
* capable of constructing advanced BitBake argv combinations
* capable of recipe/target/task selection
* capable of editable parameters
* capable of launching interactive commands through a PTY where necessary
* capable of launching noninteractive commands as daemon-owned jobs
* capable of retaining output/history
* capable of storing favorite command templates

==================================================
UI CONCEPT

Add Raw Mode to the Navigator.

Selecting it opens a hierarchical browser.

The first level follows the cheatsheet Table of Contents.

Example:

Raw Mode
├─ Version and help
├─ Basic builds
├─ CLI options
├─ Recipe task execution
├─ Normal recipe tasks
├─ Manually invoked tasks
├─ Image tasks
├─ Kernel tasks
├─ Cleaning and rebuilding
├─ Parse / dry run
├─ Environment / variables
├─ Dependency graphs
├─ Signatures
├─ Shared state
├─ Debug / logging
├─ Server / client
├─ Configuration injection
├─ Multiconfig
├─ runall / runonly
├─ Fetch debugging
├─ Compile debugging
├─ Package debugging
├─ Image debugging
├─ Debugging workflows
├─ Commands worth memorizing
├─ BitBake setup
├─ Companion commands
└─ Favorites

Do not blindly expose conceptual/reference-only sections as executable command groups.

Classify entries appropriately.

Selecting a category opens its command list.

Example:

Task control
  bitbake -c <task> <recipe>
  bitbake -f -c <task> <recipe>
  bitbake -C <task> <recipe>
  bitbake <target> --runall=<task>
  bitbake <target> --runonly=<task>
  bitbake --no-setscene <target>
  bitbake --skip-setscene <target>
  bitbake --setscene-only <target>

When the selection moves, the Inspector/help pane must immediately show the description belonging to the EXACT highlighted command.

Example:

┌ Raw Mode: Task control ──────────────────────┐
│                                              │
│ > bitbake -c <task> <recipe>                 │
│   bitbake -f -c <task> <recipe>              │
│   bitbake -C <task> <recipe>                 │
│   ...                                        │
└─────────────────────────────┬────────────────┘
                              │
┌ Command Help ───────────────┤
│ Execute one named task for  │
│ a recipe.                   │
│                             │
│ Template                    │
│ bitbake -c <task> <recipe>  │
│                             │
│ Capability                  │
│ Available                   │
│                             │
│ Favorite                    │
│ ☆                           │
└─────────────────────────────┘

The description must originate from the structured command catalog derived from the reference, not from approximate UI prose.

==================================================
COMMAND SELECTION

Pressing Enter on an executable command opens a typed command configuration window.

Example:

┌ Run BitBake Command ──────────────────────────────┐
│                                                   │
│ Command                                           │
│ bitbake -c <task> <recipe>                        │
│                                                   │
│ Recipe    [ busybox                    ▼ ]         │
│ Task      [ compile                    ▼ ]         │
│                                                   │
│ Additional arguments                              │
│ [                                                ]│
│                                                   │
│ Exact argv                                        │
│ [0] bitbake                                       │
│ [1] -c                                            │
│ [2] compile                                       │
│ [3] busybox                                       │
│                                                   │
│ [ Run ] [ Favorite ] [ Cancel ]                   │
└───────────────────────────────────────────────────┘

Parameter types must be modeled.

Examples:

<recipe>      Recipe selector
<image>       Image selector
<target>      Target selector/editor
<task>        Task selector/editor
<ui>          Enumerated selector where discoverable
<file>        Validated path editor
<value>       Bounded text editor
<number>      Numeric editor
<config>      Multiconfig selector where authoritative

Recipe/image/target parameters should use authoritative inventories when available.

Users must also be able to manually edit parameters where appropriate.

Selection and free editing must coexist.

Do not force users to select only discovered recipes when BitBake itself accepts a valid manually entered target.

==================================================
EXACT ARGV PREVIEW

Before execution always display the exact command vector.

Example:

Executable:
    bitbake
Arguments:
    [0] -f
    [1] -c
    [2] compile
    [3] busybox

Never internally reduce this to:

"bitbake -f -c compile busybox"

for shell execution.

Spawn with an argv vector.

No:

sh -c
bash -c
eval
system(command_string)

for ordinary Raw Mode execution.

==================================================
ADVANCED ARGUMENT EDITOR

Allow expert users to append/edit supported BitBake arguments.

The editor must:

* tokenize into argv without shell evaluation
* preserve quoted argument intent safely
* reject NUL/control corruption
* bound total arguments
* bound argument lengths
* display the resulting indexed argv
* allow review before execution

Do not support shell operators:

|
>
>>
<
&&
||
;
$()
backticks

Those belong in the embedded native shell, not Raw Mode.

==================================================
EXECUTION WINDOW

After confirmation, open a dedicated command execution view.

For noninteractive commands:

┌ Raw: bitbake -e busybox ─────────────────────┐
│ State: Running                               │
│ Elapsed: 00:03.4                             │
│                                              │
│ ... streamed output ...                     │
│                                              │
├──────────────────────────────────────────────┤
│ Follow ON | Search | Save | Cancel | Detach │
└──────────────────────────────────────────────┘

Requirements:

* daemon-owned execution
* background job identity
* bounded output
* stdout/stderr
* elapsed time
* exit code
* cancellation
* search
* scroll
* follow/pause
* detach
* reattach
* history

Closing the Raw execution window must not necessarily kill the job.

==================================================
INTERACTIVE RAW COMMANDS

Some BitBake commands invoke interactive interfaces.

Examples may include UI selection, menuconfig-like workflows, devshell-related behavior, or other terminal applications.

When the selected command requires interaction:

Raw Mode
   ↓
typed argv
   ↓
daemon-owned PTY
   ↓
embedded Yoctui terminal pane

Do not attempt to capture an interactive ncurses program through a normal line-oriented job runner.

Reuse the existing PTY/session architecture.

The user must be able to:

* interact
* resize
* detach
* return to Yoctui
* reattach
* terminate
* inspect session state

==================================================
FAVORITES

Raw Mode must support persistent favorite commands.

A favorite stores a COMMAND TEMPLATE plus user-selected defaults.

Examples:

★ Build core-image-minimal
  bitbake core-image-minimal
★ BusyBox menuconfig
  bitbake -c menuconfig busybox
★ Dump BusyBox environment
  bitbake -e busybox
★ Force kernel compile
  bitbake -f -c compile virtual/kernel

Do not store transient runtime state such as:

* PID
* output
* job ID
* absolute temporary paths unless explicitly part of user input

Favorites persist across Yoctui restarts.

When Raw Mode opens, Favorites should be easy to reach, preferably first or as a pinned group.

Favorite operations:

* add
* remove
* rename
* reorder if practical
* edit defaults
* execute
* inspect compatibility

==================================================
CAPABILITY CORRELATION

This requirement is CRITICAL.

The reference cheatsheet targets:

Yocto Wrynose 6.0
BitBake 2.18

Raw Mode must NOT assume every connected build environment is BitBake 2.18.

Integrate with Yoctui’s release/capability-correlation architecture.

Each command template must specify required capabilities.

Example conceptually:

RawCommand {
    id,
    category,
    description,
    executable,
    arguments,
    parameters,
    required_capabilities,
    interaction_mode,
    safety,
    reference,
}

For the connected environment:

Raw command
     ↓
required capabilities
     ↓
daemon CapabilitySnapshot
     ↓
   Available
   Limited
   Unavailable
   Unknown

Unsupported commands remain visible where useful for discoverability, but disabled.

Inspector example:

Availability
Unavailable
Reason
Connected BitBake does not expose this option.

Never launch an unsupported command merely because it appears in the cheatsheet.

Future BitBake releases should use positive capability probing where possible.

==================================================
REFERENCE ARCHITECTURE

Do not parse the Markdown cheatsheet at runtime on every startup.

The Markdown file is documentation/reference.

Create a typed Raw Mode command catalog in code or a validated bundled data representation.

The catalog must preserve traceability back to:

* reference section
* reference command
* reference description

Add a test/tool that checks the catalog against the reference where practical.

Desired architecture:

bundled cheatsheet
      │
      │ development/reference
      ▼
typed RawCommand catalog
      │
      ├── categories
      ├── templates
      ├── descriptions
      ├── parameters
      ├── capabilities
      ├── interaction mode
      └── safety classification
                │
                ▼
         daemon capability snapshot
                │
                ▼
           Raw Mode model
                │
                ▼
       Navigator / Workspace / Inspector
                │
                ▼
         typed execution request
                │
         ┌──────┴───────┐
         ▼              ▼
    Job runner           PTY
   noninteractive     interactive

==================================================
TASK REGISTRY

Extend docs/task-registry.toml with a new required milestone.

Use the next available milestone number.

Suggested milestone title:

Raw BitBake Command Workbench

Do not assume a milestone number; inspect the registry.

Create atomic dependency-ordered tasks.

At minimum create tasks equivalent to:

RAW-REF-001
Import and preserve BitBake cheatsheet reference.

RAW-SPEC-001
Specify Raw Mode UX, execution, safety, favorites and capability contract.

RAW-CATALOG-MODEL-001
Define typed command/category/parameter model.

RAW-CATALOG-001
Encode the cheatsheet’s executable BitBake command surface into the catalog.

RAW-CATALOG-TRACE-001
Verify catalog traceability to the bundled cheatsheet.

RAW-CAP-001
Map Raw commands to capability requirements.

RAW-CAP-PROBE-001
Integrate command availability with the daemon capability snapshot.

RAW-PARAM-001
Implement typed parameter definitions and validation.

RAW-RECIPE-001
Integrate authoritative recipe/image/target selectors.

RAW-ARG-001
Implement bounded expert argv editor.

RAW-PREVIEW-001
Implement exact executable/indexed-argv preview.

RAW-MODEL-001
Implement Raw Mode application state/reducer/actions.

RAW-NAV-001
Add Raw Mode Navigator destination.

RAW-CATEGORY-UI-001
Implement scrollable TOC/category browser.

RAW-COMMAND-UI-001
Implement command list.

RAW-HELP-UI-001
Implement exact selection-following command help Inspector.

RAW-FORM-UI-001
Implement command parameter form.

RAW-EXEC-MODEL-001
Add typed execution lifecycle.

RAW-JOB-001
Execute noninteractive Raw commands through daemon-owned jobs.

RAW-PTY-001
Execute interactive Raw commands through daemon-owned PTYs.

RAW-OUTPUT-UI-001
Implement dedicated output/execution workspace.

RAW-HISTORY-001
Persist bounded Raw command history.

RAW-FAVORITE-MODEL-001
Define persistent favorite command model.

RAW-FAVORITE-PERSIST-001
Persist favorites atomically.

RAW-FAVORITE-UI-001
Implement Favorites browser/actions.

RAW-SEARCH-001
Search categories, commands and descriptions.

RAW-MOUSE-001
Add first-class mouse behavior.

RAW-RESPONSIVE-001
Implement wide/medium/narrow layouts.

RAW-A11Y-001
Verify themes, no-color, focus and reduced-motion behavior.

RAW-SECURITY-001
Verify no shell-evaluation escape paths.

RAW-COMPAT-001
Verify dynamic command availability across BitBake capability fixtures.

RAW-LIVE-001
Validate representative Raw commands against real supported Poky/BitBake.

RAW-DOC-001
Document Raw Mode.

RAW-001
Parent completion gate.

Split these further if a task cannot reasonably be implemented and verified in one coherent commit.

==================================================
TEST REQUIREMENTS

Add extensive tests.

CATALOG TESTS

Verify:

* unique command IDs
* valid category references
* nonempty descriptions
* parameter placeholders match templates
* every executable entry has an execution classification
* every advanced command has capability requirements where applicable
* no shell operator is encoded accidentally
* no duplicate aliases unless deliberately modeled
* reference section exists

PARAMETER TESTS

Verify:

* recipe selector
* target selector
* task selector
* numeric input
* enumerated input
* free text bounds
* invalid characters
* argv construction
* empty required fields
* optional fields

ARGV TESTS

Given:

bitbake -f -c <task> <recipe>

and:

task=compile
recipe=busybox

must produce exactly:

["bitbake", "-f", "-c", "compile", "busybox"]

Test hostile input such as:

busybox; rm -rf /
$(command)
foo | bar
`command`

No shell execution may occur.

UI TESTS

Use Ratatui TestBackend.

Test:

* Raw Mode Navigator entry
* category list
* category scrolling
* command scrolling
* selection
* exact Inspector help change
* available state
* unavailable state/reason
* parameter dialog
* recipe picker
* editable parameter
* argv preview
* output view
* favorites
* history
* search
* narrow terminal
* medium terminal
* wide terminal

INPUT TESTS

Verify:

* arrows/j/k
* Enter
* Esc
* Tab/Shift+Tab
* search
* favorite shortcut
* run
* edit
* cancel
* detach
* mouse selection
* wheel scrolling

JOB TESTS

Verify:

* stdout
* stderr
* exit 0
* exit nonzero
* cancellation
* daemon loss
* reconnect
* bounded output

