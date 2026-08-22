# Reference snapshots

Files in this directory are preserved, human-readable source references used
to design and review Yoctui's typed integrations. They are not runtime command
databases and are never parsed as authority when Yoctui starts.

## BitBake Wrynose 6.0 / 2.18 cheatsheet

`bitbake-cheatsheet-wrynose-6.0-bitbake-2.18.md` is the supplied command
reference snapshot for Yocto Project Wrynose 6.0 and BitBake 2.18. Its content
is preserved verbatim, with source/version scope and upstream references inside
the document.

The snapshot provides traceability for the Raw Mode command catalog. Runtime
Raw Mode availability is determined exclusively from the connected build
environment's daemon-owned capability snapshot. A command appearing in this
reference does not make it available in another BitBake release, and shell
pipelines or conceptual examples in the snapshot are not Raw Mode commands.
