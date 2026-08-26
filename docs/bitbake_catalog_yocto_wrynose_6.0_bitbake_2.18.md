# BitBake Cheat Sheet — Yocto Project 6.0 “Wrynose” / BitBake 2.18

> **Target:** Yocto Project 6.0.x “Wrynose” LTS  
> **Verified against:** Yocto 6.0.2 / BitBake 2.18 documentation  
> **Last verified:** 2026-08-15  
>
> This is a command-oriented cheat sheet. Commands are intentionally shown with a `#` comment immediately above them so the file can also be used as a readable shell reference.
>
> The exact tasks available with `-c` depend on the active metadata, inherited classes, recipe type, and layers. Always use `bitbake -c listtasks <recipe>` when in doubt.

---

## Table of Contents

1. [Version and help](#1-version-and-help)
2. [Basic builds](#2-basic-builds)
3. [Complete BitBake 2.18 CLI option reference](#3-complete-bitbake-218-cli-option-reference)
4. [Recipe task execution](#4-recipe-task-execution)
5. [Normal OpenEmbedded recipe tasks](#5-normal-openembedded-recipe-tasks)
6. [Manually invoked tasks](#6-manually-invoked-tasks)
7. [Image-related tasks](#7-image-related-tasks)
8. [Kernel-related tasks](#8-kernel-related-tasks)
9. [Cleaning and rebuilding](#9-cleaning-and-rebuilding)
10. [Parse-only and dry-run](#10-parse-only-and-dry-run)
11. [Environment and variable inspection](#11-environment-and-variable-inspection)
12. [Dependency graphs](#12-dependency-graphs)
13. [Signatures and rebuild analysis](#13-signatures-and-rebuild-analysis)
14. [Shared-state / setscene control](#14-shared-state-setscene-control)
15. [Debug and logging](#15-debug-and-logging)
16. [Server/client options](#16-serverclient-options)
17. [Configuration injection](#17-configuration-injection)
18. [Multiconfig](#18-multiconfig)
19. [`--runall` and `--runonly`](#19---runall-and---runonly)
20. [Fetch/source debugging](#20-fetchsource-debugging)
21. [Compile/configure debugging](#21-compileconfigure-debugging)
22. [Package debugging](#22-package-debugging)
23. [Image debugging](#23-image-debugging)
24. [Useful paths and variables](#24-useful-paths-and-variables)
25. [Practical debugging workflows](#25-practical-debugging-workflows)
26. [Commands worth memorizing](#26-commands-worth-memorizing)
27. [BitBake setup — Wrynose-era companion tool](#27-bitbake-setup-wrynose-era-companion-tool)
28. [Yocto companion commands](#28-yocto-companion-commands)
29. [Quick conceptual reference](#29-quick-conceptual-reference)
30. [Official references](#30-official-references)

---

# 1. Version and help

```bash
# Show the installed BitBake version.
bitbake --version

# Show complete help for the exact BitBake version currently in PATH.
bitbake --help

# Short form of --help.
bitbake -h

# Confirm which BitBake executable your shell is using.
command -v bitbake

# Show the shell-resolved BitBake executable and any aliases/functions.
type -a bitbake
```

For Wrynose 6.0.2, the matching BitBake release branch is **2.18**.

---

# 2. Basic builds

```bash
# Build a recipe using its default task, normally do_build.
bitbake <recipe>

# Build an image.
bitbake <image>

# Build Yocto's minimal reference image.
bitbake core-image-minimal

# Build several targets in one BitBake invocation.
bitbake <target1> <target2> <target3>

# Execute a specific task using target:task syntax.
bitbake <recipe>:do_<task>

# Execute different tasks for different recipes in one invocation.
bitbake <recipe1>:do_<task1> <recipe2>:do_<task2>

# Build everything eligible for the special world target.
bitbake world

# Continue building independent world targets after failures.
bitbake -k world
```

---

# 3. Complete BitBake 2.18 CLI option reference

This section covers the **entire documented `bitbake -h` option set for BitBake 2.18**.

## 3.1 General options

```bash
# Show current and preferred versions of all known recipes.
bitbake -s

# Long form of -s.
bitbake --show-versions

# Dump the global BitBake datastore/environment.
bitbake -e

# Dump the fully expanded datastore/environment for one recipe.
bitbake -e <recipe>

# Long form of -e.
bitbake --environment <recipe>

# Generate dependency information in Graphviz DOT format.
bitbake -g <target>

# Long form of -g.
bitbake --graphviz <target>

# Use the normal terminal UI.
bitbake -u knotty <target>

# Use the ncurses UI.
bitbake -u ncurses <target>

# Use the task dependency explorer UI.
bitbake -u taskexp -g <target>

# Use the ncurses task dependency explorer UI.
bitbake -u taskexp_ncurses -g <target>

# Use TeamCity-oriented output.
bitbake -u teamcity <target>

# Long form for selecting a UI.
bitbake --ui=<ui> <target>

# Show the BitBake version and exit.
bitbake --version

# Show command-line help and exit.
bitbake -h

# Long form of -h.
bitbake --help
```

## 3.2 Task control options

```bash
# Force the requested target/task to execute by invalidating its existing stamp.
bitbake -f -c <task> <recipe>

# Long form of -f.
bitbake --force -c <task> <recipe>

# Execute one named task for a recipe.
bitbake -c <task> <recipe>

# Long form of -c.
bitbake --cmd=<task> <recipe>

# Invalidate a named task stamp, then execute the target's normal default task.
bitbake -C <task> <recipe>

# Long form of -C.
bitbake --clear-stamp=<task> <recipe>

# Run a named task for every applicable recipe in the target's task graph.
bitbake <target> --runall=<task>

# Run only a named task throughout the target graph plus dependencies required by those tasks.
bitbake <target> --runonly=<task>

# Do not run setscene tasks and ignore sstate restoration.
bitbake --no-setscene <target>

# Skip new setscene execution but keep output already restored from sstate.
bitbake --skip-setscene <target>

# Execute only setscene tasks and no normal build tasks.
bitbake --setscene-only <target>
```

## 3.3 Execution control options

```bash
# Perform a dry run: construct what would run without executing real tasks.
bitbake -n <target>

# Long form of -n.
bitbake --dry-run <target>

# Parse all recipes/metadata and stop before task execution.
bitbake -p

# Long form of -p.
bitbake --parse-only

# Continue executing independent tasks after an error when possible.
bitbake -k <target>

# Long form of -k.
bitbake --continue <target>

# Profile BitBake itself and save profiling reports.
bitbake -P <target>

# Long form of -P.
bitbake --profile <target>

# Dump task signature information without executing tasks.
bitbake -S none <target>

# Long form using the "none" signature handler.
bitbake --dump-signatures=none <target>

# Compare signatures with previous local/sstate signatures to explain rebuilds.
bitbake -S printdiff <target>

# Long form using the "printdiff" signature handler.
bitbake --dump-signatures=printdiff <target>

# Set BitBake's exit status based on whether floating upstream revisions changed.
bitbake --revisions-changed <target>

# Execute tasks directly from one .bb file without normal inter-recipe dependency handling.
bitbake -b path/to/recipe.bb

# Long form of -b.
bitbake --buildfile=path/to/recipe.bb

# Execute a named task directly from one .bb file.
bitbake -b path/to/recipe.bb -c <task>
```

> `-b/--buildfile` is primarily a debugging tool. It intentionally does **not** handle dependencies from other recipes normally.

## 3.4 Logging and output control options

```bash
# Enable BitBake debug level 1.
bitbake -D <target>

# Enable BitBake debug level 2.
bitbake -DD <target>

# Enable BitBake debug level 3.
bitbake -DDD <target>

# Enable BitBake debug level 4.
bitbake -DDDD <target>

# Long form of one debug increment.
bitbake --debug <target>

# Enable debug logging for specific logging domains.
bitbake -l <debug-domain> <target>

# Long form of -l.
bitbake --log-domains=<debug-domain> <target>

# Enable shell-task tracing with set -x and print bb.note() messages to stdout.
bitbake -v <target>

# Long form of -v.
bitbake --verbose <target>

# Reduce terminal log output.
bitbake -q <target>

# Reduce terminal output further.
bitbake -qq <target>

# Long form of -q.
bitbake --quiet <target>

# Write build events to a BitBake JSON event log.
bitbake -w bitbake-events.json <target>

# Long form of -w.
bitbake --write-log=bitbake-events.json <target>

# Let BitBake automatically choose the event-log filename.
bitbake -w '' <target>
```

## 3.5 Server options

```bash
# Bind the BitBake XML-RPC server to a specified address/name.
bitbake -B <address> --server-only

# Long form of -B.
bitbake --bind=<address> --server-only

# Set the server inactivity timeout in seconds.
bitbake -T <seconds> <target>

# Disable automatic server unloading due to inactivity.
bitbake -T -1 <target>

# Long form of -T.
bitbake --idle-timeout=<seconds> <target>

# Connect to an already running BitBake server.
bitbake --remote-server=<server>

# Terminate any running BitBake server associated with the environment.
bitbake -m

# Long form of -m.
bitbake --kill-server

# Supply an XML-RPC connection token when connecting to a remote server.
bitbake --token=<token> --remote-server=<server>

# Connect to a server as an observation-only client.
bitbake --observe-only --remote-server=<server>

# Query only the status of a remote BitBake server.
bitbake --status-only --remote-server=<server>

# Start only the BitBake cooker/server process without a normal UI client.
bitbake --server-only
```

## 3.6 Configuration options

```bash
# Read an extra configuration file before bitbake.conf.
bitbake -r <prefile.conf> <target>

# Long form of -r.
bitbake --read=<prefile.conf> <target>

# Read an extra configuration file after bitbake.conf.
bitbake -R <postfile.conf> <target>

# Long form of -R.
bitbake --postread=<postfile.conf> <target>

# Assume a dependency is already provided, equivalent to ASSUME_PROVIDED.
bitbake -I <dependency> <target>

# Long form of -I.
bitbake --ignore-deps=<dependency> <target>
```

---

# 4. Recipe task execution

```bash
# List every task defined for a recipe.
bitbake -c listtasks <recipe>

# Execute a specific task.
bitbake -c <task> <recipe>

# Equivalent ordering commonly used by Yocto developers.
bitbake <recipe> -c <task>

# Force a specific task to run.
bitbake -f -c <task> <recipe>

# Run a task and show verbose shell execution.
bitbake -v -c <task> <recipe>

# Force and trace a task.
bitbake -v -f -c <task> <recipe>

# Invalidate a task stamp and rebuild the recipe normally from that point as dependencies require.
bitbake -C <task> <recipe>
```

Task names passed to `-c` omit the `do_` prefix:

```text
do_compile      -> bitbake -c compile <recipe>
do_install      -> bitbake -c install <recipe>
do_package      -> bitbake -c package <recipe>
```

---

# 5. Normal OpenEmbedded recipe tasks

These tasks are defined by OpenEmbedded/Yocto metadata rather than by the bare BitBake executable itself.

## 5.1 Build

```bash
# Execute the normal top-level recipe build task.
bitbake -c build <recipe>

# Build a recipe using the default task, normally equivalent to reaching do_build.
bitbake <recipe>
```

## 5.2 Fetch

```bash
# Fetch the source files specified by SRC_URI.
bitbake -c fetch <recipe>

# Force the fetch task to execute again.
bitbake -f -c fetch <recipe>
```

## 5.3 Unpack

```bash
# Unpack fetched source into UNPACKDIR / the recipe working area.
bitbake -c unpack <recipe>
```

## 5.4 Patch

```bash
# Locate and apply patches specified by recipe metadata.
bitbake -c patch <recipe>
```

## 5.5 Configure

```bash
# Configure the source tree according to the recipe and inherited build classes.
bitbake -c configure <recipe>

# Force configuration to rerun.
bitbake -f -c configure <recipe>
```

## 5.6 Compile

```bash
# Compile the recipe.
bitbake -c compile <recipe>

# Force compilation to rerun.
bitbake -f -c compile <recipe>

# Trace shell commands while compiling.
bitbake -v -c compile <recipe>
```

## 5.7 Compile ptests

```bash
# Compile the software's runtime ptest suite when supported.
bitbake -c compile_ptest_base <recipe>
```

## 5.8 Configure ptests

```bash
# Configure the software's ptest suite when supported.
bitbake -c configure_ptest_base <recipe>
```

## 5.9 Install

```bash
# Install recipe output into the staging destination directory ${D}.
bitbake -c install <recipe>

# Force the install task to rerun.
bitbake -f -c install <recipe>
```

## 5.10 Install ptests

```bash
# Install ptest files into the recipe's package staging area when supported.
bitbake -c install_ptest_base <recipe>
```

## 5.11 Package

```bash
# Analyze ${D} and split installed files into packages according to PACKAGES and FILES.
bitbake -c package <recipe>
```

## 5.12 Package QA

```bash
# Run package QA checks.
bitbake -c package_qa <recipe>
```

## 5.13 Recipe QA

```bash
# Run metadata-only recipe QA checks.
bitbake -c recipe_qa <recipe>
```

## 5.14 Package data

```bash
# Write package metadata into the global package-data area.
bitbake -c packagedata <recipe>
```

## 5.15 Package backends

```bash
# Create RPM package files when RPM packaging is enabled.
bitbake -c package_write_rpm <recipe>

# Create IPK package files when IPK packaging is enabled.
bitbake -c package_write_ipk <recipe>

# Create DEB package files when Debian packaging is enabled.
bitbake -c package_write_deb <recipe>
```

## 5.16 Populate sysroot

```bash
# Stage the recipe's development/build files into its sysroot output.
bitbake -c populate_sysroot <recipe>
```

## 5.17 Prepare recipe sysroot

```bash
# Populate recipe-sysroot and recipe-sysroot-native from DEPENDS.
bitbake -c prepare_recipe_sysroot <recipe>
```

## 5.18 License data

```bash
# Collect and stage license information for the recipe.
bitbake -c populate_lic <recipe>
```

## 5.19 Deploy

```bash
# Deploy recipe output when the recipe implements do_deploy.
bitbake -c deploy <recipe>
```

## 5.20 Remove work

```bash
# Run the rm_work cleanup task when available/enabled.
bitbake -c rm_work <recipe>
```

---

# 6. Manually invoked tasks

## 6.1 Check SRC_URI

```bash
# Validate the recipe's SRC_URI entries.
bitbake -c checkuri <recipe>
```

## 6.2 Clean

```bash
# Remove recipe output from do_unpack forward while preserving sstate.
bitbake -c clean <recipe>
```

## 6.3 Clean shared state

```bash
# Remove recipe output and the recipe's local sstate cache entries.
bitbake -c cleansstate <recipe>
```

## 6.4 Clean everything

```bash
# Remove recipe output, local sstate, and downloaded source files.
bitbake -c cleanall <recipe>
```

> **Do not use `cleanall` routinely.** Current Yocto documentation explicitly recommends using a forced fetch when the goal is simply to fetch again.

```bash
# Preferred way to make BitBake perform the fetch again.
bitbake -f -c fetch <recipe>
```

## 6.5 Development shell

```bash
# Open an interactive shell with the recipe's complete build environment.
bitbake -c devshell <recipe>
```

## 6.6 Python development shell

```bash
# Open an interactive Python shell connected to the BitBake datastore/environment.
bitbake -c pydevshell <recipe>
```

## 6.7 List image features

```bash
# List IMAGE_FEATURES values available to an image recipe.
bitbake -c list_image_features <image>
```

## 6.8 List tasks

```bash
# List all tasks defined for a target.
bitbake -c listtasks <recipe>
```

## 6.9 Package feed index

```bash
# Build/update the package feed index through the package-index recipe.
bitbake package-index
```

---

# 7. Image-related tasks

```bash
# Create the root filesystem for an image.
bitbake -c rootfs <image>

# Run the image-generation stage.
bitbake -c image <image>

# Run final image completion/post-processing.
bitbake -c image_complete <image>

# Create bootable live-image content when supported.
bitbake -c bootimg <image>

# Bundle an initramfs with the kernel when configured.
bitbake -c bundle_initramfs virtual/kernel

# Build an installable standard SDK for an image.
bitbake -c populate_sdk <image>

# Build an extensible SDK for an image when supported.
bitbake -c populate_sdk_ext <image>

# Boot an image and run runtime image tests when configured.
bitbake -c testimage <image>

# Run the automatic image-test task when TESTIMAGE_AUTO is enabled.
bitbake -c testimage_auto <image>
```

---

# 8. Kernel-related tasks

Some of these also apply to other Linux-style Kconfig recipes such as BusyBox.

```bash
# Build the selected kernel provider.
bitbake virtual/kernel

# Open the kernel configuration menu.
bitbake -c menuconfig virtual/kernel

# Generate a configuration-difference fragment after menuconfig changes.
bitbake -c diffconfig virtual/kernel

# Save a minimal defconfig-style configuration.
bitbake -c savedefconfig virtual/kernel

# Validate the resulting kernel configuration against requested fragments/features.
bitbake -f -c kernel_configcheck virtual/kernel

# Assemble and merge kernel configuration fragments.
bitbake -c kernel_configme virtual/kernel

# Collect kernel metadata, features, patches, and configuration fragments.
bitbake -c kernel_metadata virtual/kernel

# Prepare/check out the kernel source tree in the form expected by subsequent tasks.
bitbake -c kernel_checkout virtual/kernel

# Validate configured source/metadata branches.
bitbake -c validate_branches virtual/kernel

# Compile the kernel.
bitbake -c compile virtual/kernel

# Compile kernel modules.
bitbake -c compile_kernelmodules virtual/kernel

# Populate the shared kernel work directory used by out-of-tree/module builds.
bitbake -c shared_workdir virtual/kernel

# Run the kernel image size check when KERNEL_IMAGE_MAXSIZE is configured.
bitbake -c sizecheck virtual/kernel

# Strip configured nonessential sections from vmlinux when supported.
bitbake -c strip virtual/kernel

# Deploy kernel artifacts.
bitbake -c deploy virtual/kernel
```

---

# 9. Cleaning and rebuilding

## 9.1 `clean`

```bash
# Remove normal recipe work/output while leaving reusable sstate intact.
bitbake -c clean <recipe>
```

A subsequent build may be restored almost immediately from sstate:

```bash
# Rebuild after clean; BitBake may restore output from sstate instead of recompiling.
bitbake <recipe>
```

## 9.2 `cleansstate`

```bash
# Remove recipe work/output plus local sstate entries.
bitbake -c cleansstate <recipe>
```

Use with care in shared build environments.

Current Yocto guidance prefers `-f` when the goal is simply to force execution:

```bash
# Force the normal requested task to execute without deleting shared caches.
bitbake -f -c compile <recipe>
```

## 9.3 `cleanall`

```bash
# Remove work/output, sstate, and source downloads for the recipe.
bitbake -c cleanall <recipe>
```

Avoid routine use, especially with a shared `DL_DIR`.

## 9.4 Force vs clear-stamp

```bash
# Force only do_compile to execute.
bitbake -f -c compile <recipe>

# Invalidate do_compile and then run the recipe's normal default build path.
bitbake -C compile <recipe>
```

Conceptually:

```text
-f -c compile
    Force do_compile itself.

-C compile
    Invalidate the compile stamp, then run the normal target.
    Downstream tasks are rebuilt as required by the task graph.
```

---

# 10. Parse-only and dry-run

```bash
# Parse configuration and all recipe metadata, then exit.
bitbake -p

# Long form.
bitbake --parse-only

# Construct the requested target's execution plan but execute no real tasks.
bitbake -n <target>

# Long form.
bitbake --dry-run <target>

# Dry-run an image build.
bitbake -n core-image-minimal
```

Conceptually:

```text
bitbake -p
    configuration
        ↓
    recipe parsing
        ↓
       STOP

bitbake -n <target>
    configuration
        ↓
    recipe parsing
        ↓
    provider resolution
        ↓
    dependency/task graph
        ↓
    determine what would run
        ↓
       STOP
```

---

# 11. Environment and variable inspection

## 11.1 Full datastore

```bash
# Dump the global BitBake datastore.
bitbake -e

# Dump the final expanded datastore for one recipe.
bitbake -e <recipe>

# Save the recipe datastore to a file.
bitbake -e <recipe> > /tmp/<recipe>.env
```

`bitbake -e` is especially valuable because its output includes assignment history showing where many variables were set or changed.

## 11.2 Common variables

```bash
# Show the recipe work directory.
bitbake -e <recipe> | grep '^WORKDIR='

# Show the unpack directory used by current metadata.
bitbake -e <recipe> | grep '^UNPACKDIR='

# Show the source directory.
bitbake -e <recipe> | grep '^S='

# Show the build directory.
bitbake -e <recipe> | grep '^B='

# Show the install destination staging directory.
bitbake -e <recipe> | grep '^D='

# Show source URIs.
bitbake -e <recipe> | grep '^SRC_URI='

# Show the selected source revision.
bitbake -e <recipe> | grep '^SRCREV='

# Show build-time recipe dependencies.
bitbake -e <recipe> | grep '^DEPENDS='

# Show runtime dependency variables.
bitbake -e <recipe> | grep '^RDEPENDS'

# Show package configuration selections.
bitbake -e <recipe> | grep '^PACKAGECONFIG='

# Show package names emitted by the recipe.
bitbake -e <recipe> | grep '^PACKAGES='

# Show the machine configuration.
bitbake -e | grep '^MACHINE='

# Show the distro configuration.
bitbake -e | grep '^DISTRO='

# Show the top build temporary directory.
bitbake -e | grep '^TMPDIR='

# Show the downloads cache.
bitbake -e | grep '^DL_DIR='

# Show the shared-state cache.
bitbake -e | grep '^SSTATE_DIR='

# Show BitBake's metadata search path.
bitbake -e | grep '^BBPATH='

# Show patterns used to locate recipe files.
bitbake -e | grep '^BBFILES='

# Show file:// search paths for a recipe.
bitbake -e <recipe> | grep '^FILESPATH='

# Show compiler selection.
bitbake -e <recipe> | grep '^CC='

# Show C compiler flags.
bitbake -e <recipe> | grep '^CFLAGS='

# Show C++ compiler flags.
bitbake -e <recipe> | grep '^CXXFLAGS='

# Show linker flags.
bitbake -e <recipe> | grep '^LDFLAGS='

# Show configure arguments typically used by Autotools recipes.
bitbake -e <recipe> | grep '^EXTRA_OECONF='

# Search the entire expanded recipe datastore for a string.
bitbake -e <recipe> | grep -n '<string>'
```

## 11.3 Find where a variable came from

```bash
# Show the final SRC_URI plus nearby assignment-history comments.
bitbake -e <recipe> | grep -B30 -A5 '^SRC_URI='

# Show the final PACKAGECONFIG plus nearby history.
bitbake -e <recipe> | grep -B30 -A5 '^PACKAGECONFIG='

# Save two environments for comparison.
bitbake -e <recipe> > before.env

# Save the environment again after changing metadata.
bitbake -e <recipe> > after.env

# Compare the two expanded datastores.
diff -u before.env after.env
```

---

# 12. Dependency graphs

```bash
# Generate dependency graph files for a recipe or image.
bitbake -g <target>

# Generate a graph while treating virtual/kernel as already provided.
bitbake -g -I virtual/kernel <target>

# Ignore several dependencies to reduce graph clutter.
bitbake -g -I virtual/kernel -I <dependency> <target>

# Convert the task dependency graph to PDF with Graphviz.
dot -Tpdf task-depends.dot -o task-depends.pdf

# Convert the task dependency graph to SVG.
dot -Tsvg task-depends.dot -o task-depends.svg

# Inspect the simple list of providers BitBake plans to build.
cat pn-buildlist

# Inspect the generated DOT task graph.
less task-depends.dot
```

Typical generated files include:

```text
task-depends.dot
pn-buildlist
```

---

# 13. Signatures and rebuild analysis

BitBake task signatures determine whether an existing result can be reused.

```bash
# Dump task signature construction data without executing tasks.
bitbake -S none <target>

# Compare current task signatures to the newest local/sstate signatures.
bitbake -S printdiff <target>

# Investigate why one recipe unexpectedly wants to rebuild.
bitbake -S printdiff <recipe>

# Investigate rebuild reasons for an entire image.
bitbake -S printdiff <image>
```

Use `printdiff` when asking:

- Why did this recipe rebuild?
- Which variable changed?
- Which dependent task changed?
- Why is sstate not being reused?

---

# 14. Shared-state / setscene control

## 14.1 Ignore sstate restoration

```bash
# Do not run setscene tasks; ignore sstate and build everything required normally.
bitbake --no-setscene <target>
```

## 14.2 Skip setscene execution

```bash
# Do not execute new setscene tasks, but retain output already restored previously.
bitbake --skip-setscene <target>
```

## 14.3 Setscene only

```bash
# Execute only setscene tasks and no normal tasks.
bitbake --setscene-only <target>
```

Conceptually:

```text
--no-setscene
    Ignore sstate restoration.
    Build required tasks normally.

--skip-setscene
    Skip new setscene operations.
    Keep previously restored output.

--setscene-only
    Attempt only sstate/setscene restoration.
    Do not run normal build tasks.
```

---

# 15. Debug and logging

## 15.1 BitBake debug level

```bash
# Show bb.debug(1, ...) messages on stdout.
bitbake -D <target>

# Show debug levels 1 and 2.
bitbake -DD <target>

# Show debug levels 1 through 3.
bitbake -DDD <target>
```

## 15.2 Verbose task tracing

```bash
# Trace shell task execution and expose bb.note() messages.
bitbake -v <target>

# Trace shell execution for one compile task.
bitbake -v -c compile <recipe>

# Force and trace compilation.
bitbake -v -f -c compile <recipe>
```

## 15.3 Quiet mode

```bash
# Reduce terminal output.
bitbake -q <target>

# Reduce terminal output further.
bitbake -qq <target>
```

## 15.4 Logging domains

```bash
# Enable debug logging only for a selected BitBake logging domain.
bitbake -l <domain> <target>
```

## 15.5 Event log

```bash
# Write the BitBake event stream to JSON.
bitbake -w events.json <target>

# Ask BitBake to select the JSON event-log filename automatically.
bitbake -w '' <target>
```

## 15.6 Profiling BitBake itself

```bash
# Profile BitBake execution and save profiler reports.
bitbake -P <target>
```

---

# 16. Server/client options

Most users rarely need these manually.

```bash
# Start BitBake in server-only mode.
bitbake --server-only

# Bind a server-only instance to a specified address/name.
bitbake -B <address> --server-only

# Set a custom server idle timeout.
bitbake -T <seconds> <target>

# Prevent inactivity-based server unloading.
bitbake -T -1 <target>

# Connect to a remote BitBake server.
bitbake --remote-server=<server>

# Connect with an XML-RPC token.
bitbake --token=<token> --remote-server=<server>

# Connect as an observer only.
bitbake --observe-only --remote-server=<server>

# Query remote-server status only.
bitbake --status-only --remote-server=<server>

# Kill the running BitBake server.
bitbake -m
```

---

# 17. Configuration injection

These are useful for experiments without editing `local.conf`.

```bash
# Read experimental configuration before bitbake.conf.
bitbake -r pre.conf <target>

# Read experimental configuration after bitbake.conf.
bitbake -R post.conf <target>

# Build an image with a temporary post-configuration file.
bitbake -R experiment.conf <image>

# Generate a dependency graph while pretending one provider is already available.
bitbake -g -I virtual/kernel <image>
```

---

# 18. Multiconfig

Multiconfig allows one BitBake invocation to build targets using multiple independent configurations.

Configuration files normally live under:

```text
conf/multiconfig/
```

and are enabled through `BBMULTICONFIG`.

```bash
# Build a target from a named multiconfig.
bitbake mc:<config>:<target>

# Build an image using a multiconfig named board1.
bitbake mc:board1:<image>

# Build the same image for two multiconfigs.
bitbake mc:board1:<image> mc:board2:<image>

# Build a target in the default configuration plus two extra multiconfigs.
bitbake <image> mc:board1:<image> mc:board2:<image>

# Explicitly select the default configuration using the empty multiconfig name.
bitbake mc::<target>
```

Cross-multiconfig dependencies can be expressed in metadata through the `mcdepends` task flag.

---

# 19. `--runall` and `--runonly`

## 19.1 `--runall`

```bash
# Run do_fetch for every applicable recipe in the image task graph.
bitbake <image> --runall=fetch

# Run do_compile for every applicable recipe in the target graph.
bitbake <target> --runall=compile
```

`--runall` asks BitBake to execute that task for recipes in the graph even if the task would not otherwise have run.

## 19.2 `--runonly`

```bash
# Restrict execution to fetch tasks in the target graph plus dependencies required by those tasks.
bitbake <image> --runonly=fetch

# Restrict execution to compile tasks plus their required task dependencies.
bitbake <image> --runonly=compile
```

---

# 20. Fetch/source debugging

```bash
# Validate SRC_URI without carrying out a normal build.
bitbake -c checkuri <recipe>

# Fetch source.
bitbake -c fetch <recipe>

# Force source to be fetched again.
bitbake -f -c fetch <recipe>

# Unpack fetched source.
bitbake -c unpack <recipe>

# Apply patches.
bitbake -c patch <recipe>

# Show the configured source URIs.
bitbake -e <recipe> | grep '^SRC_URI='

# Show the configured source revision.
bitbake -e <recipe> | grep '^SRCREV='

# Show the source directory.
bitbake -e <recipe> | grep '^S='

# Show the unpack directory.
bitbake -e <recipe> | grep '^UNPACKDIR='

# Show the download cache directory.
bitbake -e | grep '^DL_DIR='

# Find fetch logs for the recipe.
find tmp/work -path "*<recipe>*" -name 'log.do_fetch*'

# Find unpack logs for the recipe.
find tmp/work -path "*<recipe>*" -name 'log.do_unpack*'

# Find patch logs for the recipe.
find tmp/work -path "*<recipe>*" -name 'log.do_patch*'
```

---

# 21. Compile/configure debugging

```bash
# Find the recipe's WORKDIR.
bitbake -e <recipe> | grep '^WORKDIR='

# Find its source tree.
bitbake -e <recipe> | grep '^S='

# Find its build directory.
bitbake -e <recipe> | grep '^B='

# Run only configuration.
bitbake -c configure <recipe>

# Force configuration to run.
bitbake -f -c configure <recipe>

# Run only compilation.
bitbake -c compile <recipe>

# Force compilation to run.
bitbake -f -c compile <recipe>

# Run compilation with shell tracing.
bitbake -v -f -c compile <recipe>

# Enter the exact recipe cross-build environment.
bitbake -c devshell <recipe>

# Find the compiler BitBake selected.
bitbake -e <recipe> | grep '^CC='

# Find CFLAGS.
bitbake -e <recipe> | grep '^CFLAGS='

# Find CXXFLAGS.
bitbake -e <recipe> | grep '^CXXFLAGS='

# Find LDFLAGS.
bitbake -e <recipe> | grep '^LDFLAGS='

# Locate compile logs.
find tmp/work -path "*<recipe>*" -name 'log.do_compile*'

# Locate configure logs.
find tmp/work -path "*<recipe>*" -name 'log.do_configure*'

# Locate generated shell wrappers for compile.
find tmp/work -path "*<recipe>*" -name 'run.do_compile*'

# Locate generated shell wrappers for configure.
find tmp/work -path "*<recipe>*" -name 'run.do_configure*'
```

Inside a `devshell`:

```bash
# Display the selected compiler.
echo "$CC"

# Display the selected C++ compiler.
echo "$CXX"

# Display C compiler flags.
echo "$CFLAGS"

# Display linker flags.
echo "$LDFLAGS"

# Inspect the complete environment.
env | sort

# Run make manually when the recipe uses Make.
make

# Show the current directory, normally related to the recipe build directory.
pwd
```

---

# 22. Package debugging

```bash
# Run installation into ${D}.
bitbake -c install <recipe>

# Run package splitting.
bitbake -c package <recipe>

# Run recipe metadata QA.
bitbake -c recipe_qa <recipe>

# Run package QA.
bitbake -c package_qa <recipe>

# Write package metadata.
bitbake -c packagedata <recipe>

# Show generated package names.
bitbake -e <recipe> | grep '^PACKAGES='

# Inspect FILES assignments.
bitbake -e <recipe> | grep '^FILES'

# Inspect runtime dependencies.
bitbake -e <recipe> | grep '^RDEPENDS'

# Show the install staging directory.
bitbake -e <recipe> | grep '^D='

# Find install logs.
find tmp/work -path "*<recipe>*" -name 'log.do_install*'

# Find package logs.
find tmp/work -path "*<recipe>*" -name 'log.do_package*'

# Find package QA logs.
find tmp/work -path "*<recipe>*" -name 'log.do_package_qa*'
```

---

# 23. Image debugging

```bash
# Build only the root filesystem stage.
bitbake -c rootfs <image>

# Build the image-generation stage.
bitbake -c image <image>

# Run final image completion/post-processing.
bitbake -c image_complete <image>

# Show IMAGE_FEATURES.
bitbake -e <image> | grep '^IMAGE_FEATURES='

# Show IMAGE_INSTALL.
bitbake -e <image> | grep '^IMAGE_INSTALL='

# Show configured output image types.
bitbake -e <image> | grep '^IMAGE_FSTYPES='

# Show image deploy directory.
bitbake -e <image> | grep '^DEPLOY_DIR_IMAGE='

# List available image features.
bitbake -c list_image_features <image>

# Find rootfs logs.
find tmp/work -path "*<image>*" -name 'log.do_rootfs*'

# Find image-generation logs.
find tmp/work -path "*<image>*" -name 'log.do_image*'
```

---

# 24. Useful paths and variables

```bash
# Show the complete top-level temporary build directory.
bitbake -e | grep '^TMPDIR='

# Show the work-directory base.
bitbake -e | grep '^BASE_WORKDIR='

# Show source download cache.
bitbake -e | grep '^DL_DIR='

# Show shared-state cache.
bitbake -e | grep '^SSTATE_DIR='

# Show package-data directory.
bitbake -e | grep '^PKGDATA_DIR='

# Show image deployment directory.
bitbake -e | grep '^DEPLOY_DIR_IMAGE='

# Show RPM deployment directory.
bitbake -e | grep '^DEPLOY_DIR_RPM='

# Show IPK deployment directory.
bitbake -e | grep '^DEPLOY_DIR_IPK='

# Show DEB deployment directory.
bitbake -e | grep '^DEPLOY_DIR_DEB='

# Show recipe-specific WORKDIR.
bitbake -e <recipe> | grep '^WORKDIR='

# Show recipe source directory.
bitbake -e <recipe> | grep '^S='

# Show recipe build directory.
bitbake -e <recipe> | grep '^B='

# Show recipe install staging directory.
bitbake -e <recipe> | grep '^D='

# Show recipe temporary task/log directory.
bitbake -e <recipe> | grep '^T='

# Show recipe-specific sysroot.
bitbake -e <recipe> | grep '^RECIPE_SYSROOT='

# Show native recipe sysroot.
bitbake -e <recipe> | grep '^RECIPE_SYSROOT_NATIVE='
```

Typical recipe task logs are under:

```text
${WORKDIR}/temp/
```

Common examples:

```text
log.do_fetch
log.do_unpack
log.do_patch
log.do_configure
log.do_compile
log.do_install
log.do_package
log.do_package_qa

run.do_fetch
run.do_unpack
run.do_patch
run.do_configure
run.do_compile
run.do_install
run.do_package
```

---

# 25. Practical debugging workflows

## 25.1 Metadata parse failure

```bash
# First confirm that all active metadata parses.
bitbake -p

# Increase BitBake debug output if parsing fails mysteriously.
bitbake -D -p
```

Then check:

```text
conf/local.conf
conf/bblayers.conf
conf/layer.conf
*.bb
*.bbappend
*.bbclass
*.inc
```

## 25.2 Recipe not found

```bash
# Show all known recipe names and selected versions.
bitbake -s

# Search the recipe list for the expected recipe.
bitbake -s | grep -i '<recipe>'

# Show active layers.
bitbake-layers show-layers

# Show recipes and the layers providing them.
bitbake-layers show-recipes '<recipe>'
```

## 25.3 `.bbappend` does not appear to apply

```bash
# Show all .bbappend files and what recipes they match.
bitbake-layers show-appends

# Search expanded recipe metadata for a value expected from the append.
bitbake -e <recipe> | grep -n '<expected-value>'
```

## 25.4 Fetch failure

```bash
# Validate source URIs first.
bitbake -c checkuri <recipe>

# Force only the fetch task.
bitbake -f -c fetch <recipe>

# Inspect configured URIs.
bitbake -e <recipe> | grep '^SRC_URI='

# Inspect configured source revision.
bitbake -e <recipe> | grep '^SRCREV='

# Locate the fetch log.
find tmp/work -path "*<recipe>*" -name 'log.do_fetch*'
```

## 25.5 Patch failure

```bash
# Fetch source.
bitbake -c fetch <recipe>

# Unpack source.
bitbake -c unpack <recipe>

# Run patching.
bitbake -c patch <recipe>

# Inspect source location.
bitbake -e <recipe> | grep '^S='

# Locate patch logs.
find tmp/work -path "*<recipe>*" -name 'log.do_patch*'
```

## 25.6 Configure failure

```bash
# Force configuration and show verbose shell execution.
bitbake -v -f -c configure <recipe>

# Open a recipe development shell.
bitbake -c devshell <recipe>

# Locate configure logs.
find tmp/work -path "*<recipe>*" -name 'log.do_configure*'
```

## 25.7 Compile failure

```bash
# Force compilation with verbose tracing.
bitbake -v -f -c compile <recipe>

# Enter a development shell.
bitbake -c devshell <recipe>

# Locate the compile log.
find tmp/work -path "*<recipe>*" -name 'log.do_compile*'

# Locate the generated compile script.
find tmp/work -path "*<recipe>*" -name 'run.do_compile*'
```

## 25.8 Packaging failure

```bash
# Re-run installation.
bitbake -f -c install <recipe>

# Re-run package splitting.
bitbake -f -c package <recipe>

# Re-run package QA.
bitbake -f -c package_qa <recipe>

# Inspect package assignments.
bitbake -e <recipe> | grep '^FILES'

# Inspect generated packages.
bitbake -e <recipe> | grep '^PACKAGES='
```

## 25.9 Unexpected rebuild

```bash
# Ask BitBake to compare task signatures.
bitbake -S printdiff <recipe>

# Save the expanded environment for manual comparison.
bitbake -e <recipe> > recipe.env
```

## 25.10 Suspect stale state

Prefer forcing the exact task first:

```bash
# Force compilation without deleting sstate.
bitbake -f -c compile <recipe>
```

If you truly need to remove local sstate for that recipe:

```bash
# Remove recipe work and local sstate.
bitbake -c cleansstate <recipe>
```

Then rebuild:

```bash
# Rebuild the recipe normally.
bitbake <recipe>
```

Avoid this reflex:

```bash
# Usually do NOT delete the entire tmp directory just because one recipe failed.
# rm -rf tmp
```

---

# 26. Commands worth memorizing

```bash
# Check whether the complete metadata set parses.
bitbake -p

# Build a recipe or image.
bitbake <target>

# See whether BitBake knows a recipe and which version is preferred.
bitbake -s | grep '<recipe>'

# List every task available to one recipe.
bitbake -c listtasks <recipe>

# Fetch source.
bitbake -c fetch <recipe>

# Unpack source.
bitbake -c unpack <recipe>

# Apply patches.
bitbake -c patch <recipe>

# Configure.
bitbake -c configure <recipe>

# Compile.
bitbake -c compile <recipe>

# Force compile.
bitbake -f -c compile <recipe>

# Install into ${D}.
bitbake -c install <recipe>

# Run package QA.
bitbake -c package_qa <recipe>

# Open the recipe development shell.
bitbake -c devshell <recipe>

# Dump the complete expanded recipe datastore.
bitbake -e <recipe>

# Find WORKDIR.
bitbake -e <recipe> | grep '^WORKDIR='

# Find the source directory.
bitbake -e <recipe> | grep '^S='

# See what BitBake would execute without executing it.
bitbake -n <target>

# Generate a dependency graph.
bitbake -g <target>

# Explain why task signatures changed.
bitbake -S printdiff <recipe>

# Continue independent work after one failure.
bitbake -k <target>
```

---

# 27. `bitbake-setup` — Wrynose-era companion tool

`bitbake-setup` is a newer top-level BitBake tool for creating and maintaining layer/build setups from configuration templates. It is separate from the `bitbake` command itself, but is important in current Wrynose-era workflows.

## 27.1 General help/options

```bash
# Show bitbake-setup help.
bitbake-setup --help

# Short help form.
bitbake-setup -h

# Enable bitbake-setup debug output.
bitbake-setup -d <subcommand>

# Print only errors.
bitbake-setup -q <subcommand>

# Select color handling: auto, always, or never.
bitbake-setup --color=auto <subcommand>

# Avoid network update checks and use only local cached information.
bitbake-setup --no-network <subcommand>

# Use an explicit global settings file.
bitbake-setup --global-settings <file> <subcommand>

# Override one setting for only this invocation.
bitbake-setup --setting default <setting-name> <value> <subcommand>
```

## 27.2 Initialize a setup

```bash
# Interactively initialize a new setup from available configuration templates.
bitbake-setup init

# Initialize from one explicit configuration-template file.
bitbake-setup init /path/to/config.conf.json

# Initialize from a remote configuration-template URI.
bitbake-setup init <configuration-template-uri>

# Initialize non-interactively when all required selections are supplied.
bitbake-setup init --non-interactive <config> <selection1> <selection2>

# Apply one or more source-override files during initialization.
bitbake-setup init --source-overrides <override.json> <config>

# Choose the generated setup directory name.
bitbake-setup init --setup-dir-name <name> <config>

# Skip a configuration selection where permitted.
bitbake-setup init --skip-selection <selection> <config>

# Use a locally managed source tree instead of cloning/managing that source.
bitbake-setup init -L <source-name> <local-path> <config>

# Long form for using a locally managed source tree.
bitbake-setup init --use-local-source <source-name> <local-path> <config>

# Explicitly generate the VS Code workspace integration file.
bitbake-setup init --init-vscode <config>

# Explicitly skip VS Code workspace generation.
bitbake-setup init --no-init-vscode <config>
```

## 27.3 List templates

```bash
# List configuration templates available in the configured registry.
bitbake-setup list

# Include expired/EOL configuration templates.
bitbake-setup list --with-expired

# Write the template list as JSON for automation.
bitbake-setup list --write-json <file.json>
```

## 27.4 Check setup status

```bash
# Show status for the currently initialized/sourced setup.
bitbake-setup status

# Check a setup explicitly by path.
bitbake-setup status --setup-dir <setup-directory>
```

## 27.5 Update a setup

```bash
# Update the current setup against its configuration template/upstream revisions.
bitbake-setup update

# Update and prompt before changing local.conf/bblayers.conf-style build configuration.
bitbake-setup update --update-bb-conf=prompt

# Update build configuration files automatically.
bitbake-setup update --update-bb-conf=yes

# Do not update build configuration files.
bitbake-setup update --update-bb-conf=no

# Abort if local layer commits cause rebase/update conflicts.
bitbake-setup update --rebase-conflicts-strategy=abort

# Back up a conflicting local layer directory and re-clone a clean copy.
bitbake-setup update --rebase-conflicts-strategy=backup

# Update an explicitly selected setup directory.
bitbake-setup update --setup-dir <setup-directory>
```

## 27.6 Install buildtools

```bash
# Install the required buildtools tarball into the current setup.
bitbake-setup install-buildtools

# Force buildtools to be reinstalled.
bitbake-setup install-buildtools --force

# Install buildtools into an explicitly selected setup.
bitbake-setup install-buildtools --setup-dir <setup-directory>
```

## 27.7 Settings

```bash
# List current bitbake-setup settings.
bitbake-setup settings list

# Set one local bitbake-setup setting.
bitbake-setup settings set default <setting-name> <value>

# Remove one local setting.
bitbake-setup settings unset default <setting-name>

# Set a setting in the global user settings file.
bitbake-setup settings set --global default <setting-name> <value>

# Remove a setting from the global user settings file.
bitbake-setup settings unset --global default <setting-name>
```

Common settings include:

```text
top-dir-prefix
top-dir-name
registry
dl-dir
use-full-setup-dir-name
common-sstate
```

---

# 28. Yocto companion commands

These are **not `bitbake` CLI options**, but they are so closely tied to normal BitBake work that they belong in a practical cheat sheet.

## 28.1 `bitbake-layers`

```bash
# Show configured layers and their priorities/paths.
bitbake-layers show-layers

# Show recipes and the layer providing each one.
bitbake-layers show-recipes

# Show providers for a specific recipe.
bitbake-layers show-recipes '<recipe>'

# Show active .bbappend files and what they match.
bitbake-layers show-appends

# Show recipes hidden/overlaid by higher-priority providers.
bitbake-layers show-overlayed

# Show recipe dependencies in layer-oriented form when supported by the active release.
bitbake-layers show-cross-depends

# Add a layer to bblayers.conf.
bitbake-layers add-layer /path/to/meta-layer

# Remove a layer from bblayers.conf.
bitbake-layers remove-layer /path/to/meta-layer

# Create a new empty layer skeleton.
bitbake-layers create-layer /path/to/meta-my-layer
```

## 28.2 `devtool`

```bash
# Show devtool help.
devtool --help

# Create a new recipe from an existing source tree or upstream source.
devtool add <recipe> <source>

# Extract a recipe into a development workspace and prepare it for modification.
devtool modify <recipe>

# Build a recipe currently under devtool control.
devtool build <recipe>

# Deploy a built recipe directly to a reachable target.
devtool deploy-target <recipe> root@<target-ip>

# Remove files previously deployed with devtool deploy-target.
devtool undeploy-target <recipe> root@<target-ip>

# Update recipe metadata from changes made in the devtool workspace.
devtool update-recipe <recipe>

# Finish recipe changes into a selected permanent layer and remove workspace handling.
devtool finish <recipe> /path/to/meta-layer

# Remove a recipe from the devtool workspace without deleting the permanent recipe.
devtool reset <recipe>

# Show recipes currently managed by devtool.
devtool status
```

## 28.3 Build environment initialization

Classic Poky/OE-Core workflow:

```bash
# Initialize the OpenEmbedded build environment in the default build directory.
source oe-init-build-env

# Initialize the OpenEmbedded build environment in a named build directory.
source oe-init-build-env build-myboard
```

`bitbake-setup` workflow:

```bash
# Source the init script generated inside a bitbake-setup build directory.
source <setup-directory>/build/init-build-env
```

---

# 29. Quick conceptual reference

## 29.1 Recipe build pipeline

A typical recipe flows approximately through:

```text
do_fetch
    ↓
do_unpack
    ↓
do_patch
    ↓
do_prepare_recipe_sysroot
    ↓
do_configure
    ↓
do_compile
    ↓
do_install
    ↓
do_package
    ↓
do_packagedata
    ↓
do_package_qa
    ↓
do_package_write_*
    ↓
do_build
```

The exact graph varies by recipe and inherited classes.

## 29.2 Image pipeline

A simplified image flow is:

```text
package generation
    ↓
do_rootfs
    ↓
do_image
    ↓
do_image_<fstype>
    ↓
do_image_complete
```

## 29.3 Important directory meanings

```text
DL_DIR
    Downloaded upstream source cache.

SSTATE_DIR
    Shared-state cache.

TMPDIR
    Main build-output tree.

WORKDIR
    Per-recipe/per-version/per-machine work area.

UNPACKDIR
    Area into which sources are unpacked in current metadata.

S
    Source directory.

B
    Build directory.

D
    Install staging root used by do_install.

T
    Recipe task temp/log/script directory.

RECIPE_SYSROOT
    Target-side recipe-specific sysroot.

RECIPE_SYSROOT_NATIVE
    Native build-tools recipe-specific sysroot.

DEPLOY_DIR_IMAGE
    Final machine/image deployment artifacts.
```

## 29.4 `-p` vs `-n`

```text
bitbake -p
    Parse metadata only.

bitbake -n <target>
    Parse + resolve the target/task graph + determine what would run.
    Execute no real build tasks.
```

## 29.5 `-f` vs `-C`

```text
bitbake -f -c compile <recipe>
    Force do_compile itself.

bitbake -C compile <recipe>
    Invalidate compile's stamp, then execute the normal build target.
```

## 29.6 `clean` vs `cleansstate` vs `cleanall`

```text
clean
    Removes normal recipe work/output.
    Keeps sstate.
        ↓
cleansstate
    Removes normal work/output + local sstate.
        ↓
cleanall
    Removes work/output + local sstate + downloaded source.

Least destructive                         Most destructive
```

For modern Yocto development, prefer forcing the exact task with `-f` when that solves the problem.

## 29.7 BitBake metadata file types

```text
*.bb
    Recipe.

*.bbappend
    Extension/override of a recipe.

*.bbclass
    Reusable class metadata.

*.inc
    Included metadata fragment.

*.conf
    Configuration.

layer.conf
    Layer configuration.

bblayers.conf
    Active layer list.

local.conf
    User/build configuration.
```

---

# 30. Official references

This cheat sheet was verified against the current official Yocto/BitBake documentation for Wrynose / BitBake 2.18:

- Yocto Project 6.0 “Wrynose” release notes  
  https://docs.yoctoproject.org/6.0/migration-guides/release-notes-6.0.html

- Yocto Project 6.0.2 release notes  
  https://docs.yoctoproject.org/dev/migration-guides/release-notes-6.0.2.html

- BitBake 2.18 User Manual  
  https://docs.yoctoproject.org/bitbake/

- BitBake command usage and complete CLI option reference  
  https://docs.yoctoproject.org/bitbake/bitbake-user-manual/bitbake-user-manual-intro.html

- BitBake setup documentation  
  https://docs.yoctoproject.org/bitbake/bitbake-user-manual/bitbake-user-manual-environment-setup.html

- Yocto/OpenEmbedded task reference  
  https://docs.yoctoproject.org/ref-manual/tasks.html

---

# One-screen emergency reference

```bash
# Verify metadata parses.
bitbake -p

# Build a target.
bitbake <target>

# Dry-run a target.
bitbake -n <target>

# Find a recipe/version.
bitbake -s | grep '<recipe>'

# List recipe tasks.
bitbake -c listtasks <recipe>

# Dump the recipe datastore.
bitbake -e <recipe>

# Find the recipe work directory.
bitbake -e <recipe> | grep '^WORKDIR='

# Fetch source.
bitbake -c fetch <recipe>

# Unpack source.
bitbake -c unpack <recipe>

# Apply patches.
bitbake -c patch <recipe>

# Configure.
bitbake -c configure <recipe>

# Compile.
bitbake -c compile <recipe>

# Force compile.
bitbake -f -c compile <recipe>

# Install.
bitbake -c install <recipe>

# Package.
bitbake -c package <recipe>

# Run package QA.
bitbake -c package_qa <recipe>

# Open a development shell.
bitbake -c devshell <recipe>

# Generate dependency graphs.
bitbake -g <target>

# Explain an unexpected rebuild.
bitbake -S printdiff <recipe>

# Continue independent tasks after a failure.
bitbake -k <target>

# Show active layers.
bitbake-layers show-layers

# Show matching appends.
bitbake-layers show-appends
```
