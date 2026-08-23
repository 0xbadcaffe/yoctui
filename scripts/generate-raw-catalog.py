#!/usr/bin/env python3
"""Generate the structured Raw Mode catalog from the immutable reference snapshot."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shlex
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "docs/reference/bitbake-cheatsheet-wrynose-6.0-bitbake-2.18.md"
OUTPUT = ROOT / "crates/yoctui-model/src/raw_catalog_builtin.rs"
EXPECTED_SHA256 = "ad95ecfa6a17691fa2a6d12f598f01fbd33de524c2a08ebccd218ef5fe88dd47"

PLACEHOLDER_RE = re.compile(r"<([^<>]+)>")
NUMBERED_HEADING_RE = re.compile(r"^(\d+)\.\s+(.*)$")
NON_ID_RE = re.compile(r"[^a-z0-9]+")


@dataclass
class Entry:
    line: int
    category_id: str
    heading: str
    description: str
    command: str
    executable: bool


@dataclass
class Category:
    id: str
    heading: str
    label: str
    number: int | None
    entries: list[Entry] = field(default_factory=list)


@dataclass(frozen=True)
class Parameter:
    id: str
    label: str
    placeholder: str
    kind: str


def quoted(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def slug(value: str) -> str:
    value = value.lower().replace("2.18", "2-18").replace("6.0", "6-0")
    value = NON_ID_RE.sub("-", value).strip("-")
    return value or "entry"


def is_direct_bitbake(command: str) -> bool:
    if not command.startswith("bitbake "):
        return False
    return not any(operator in command for operator in (" | ", " > ", " && ", " || ", "; "))


def read_reference() -> tuple[list[Category], list[Entry]]:
    digest = hashlib.sha256(SOURCE.read_bytes()).hexdigest()
    if digest != EXPECTED_SHA256:
        raise SystemExit(
            f"Raw reference SHA-256 changed: expected {EXPECTED_SHA256}, got {digest}"
        )

    categories: list[Category] = []
    entries: list[Entry] = []
    current_category: Category | None = None
    current_heading = ""
    description = ""
    in_bash = False

    for line_number, line in enumerate(SOURCE.read_text(encoding="utf-8").splitlines(), 1):
        if line == "```bash":
            in_bash = True
            description = ""
            continue
        if in_bash and line == "```":
            in_bash = False
            description = ""
            continue

        if not in_bash and line.startswith("# "):
            heading = line[2:]
            if line_number == 1:
                continue
            match = NUMBERED_HEADING_RE.match(heading)
            number = int(match.group(1)) if match else None
            label = match.group(2) if match else heading
            prefix = f"section-{number:02d}" if number is not None else "section"
            current_category = Category(
                id=f"{prefix}-{slug(label)}",
                heading=heading,
                label=label.replace("`", ""),
                number=number,
            )
            categories.append(current_category)
            current_heading = heading
            continue
        if not in_bash and line.startswith("##"):
            current_heading = line.lstrip("#").strip()
            continue

        if not in_bash:
            continue
        if line.startswith("# "):
            description = line[2:]
            continue
        if not line:
            continue
        if current_category is None or not description:
            raise SystemExit(f"unclassified Raw reference command at line {line_number}: {line}")
        entry = Entry(
            line=line_number,
            category_id=current_category.id,
            heading=current_heading,
            description=description,
            command=line,
            executable=is_direct_bitbake(line),
        )
        current_category.entries.append(entry)
        entries.append(entry)
        description = ""

    categories.append(
        Category(
            id="favorites",
            heading="Favorites",
            label="Favorites",
            number=None,
        )
    )
    return categories, entries


def parameter_kind(identifier: str, placeholder: str) -> str:
    value = f"{identifier} {placeholder}".lower()
    if any(token in value for token in ("file", "path", ".conf", ".json", ".bb")):
        return "File"
    if "recipe" in value:
        return "Recipe"
    if "image" in value:
        return "Image"
    if "target" in value:
        return "Target"
    if "task" in value:
        return "Task"
    if identifier in {"ui", "user-interface"}:
        return "UserInterface"
    if identifier in {"seconds", "number", "count"}:
        return "Integer"
    if identifier in {"config", "multiconfig"}:
        return "Multiconfig"
    return "Text"


def make_parameter(identifier: str, placeholder: str) -> Parameter:
    identifier = slug(identifier)
    label = identifier.replace("-", " ").title()
    return Parameter(
        id=identifier,
        label=label,
        placeholder=placeholder,
        kind=parameter_kind(identifier, placeholder),
    )


FIXED_PARAMETERS = {
    "path/to/recipe.bb": ("recipe-file", "path/to/recipe.bb"),
    "bitbake-events.json": ("event-log", "bitbake-events.json"),
    "events.json": ("event-log", "events.json"),
    "pre.conf": ("prefile", "pre.conf"),
    "post.conf": ("postfile", "post.conf"),
    "experiment.conf": ("postfile", "experiment.conf"),
}


def segments_for(token: str) -> tuple[list[tuple[str, str]], list[Parameter]]:
    matches = list(PLACEHOLDER_RE.finditer(token))
    if not matches:
        for fixed, (identifier, placeholder) in FIXED_PARAMETERS.items():
            if fixed not in token:
                continue
            start = token.index(fixed)
            segments = []
            if start:
                segments.append(("literal", token[:start]))
            segments.append(("parameter", identifier))
            if start + len(fixed) < len(token):
                segments.append(("literal", token[start + len(fixed) :]))
            return segments, [make_parameter(identifier, placeholder)]
        return [("literal", token)], []

    segments: list[tuple[str, str]] = []
    parameters: list[Parameter] = []
    cursor = 0
    for match in matches:
        if match.start() > cursor:
            segments.append(("literal", token[cursor : match.start()]))
        identifier = slug(match.group(1))
        segments.append(("parameter", identifier))
        parameters.append(make_parameter(identifier, match.group(0)))
        cursor = match.end()
    if cursor < len(token):
        segments.append(("literal", token[cursor:]))
    return segments, parameters


def command_parts(command: str) -> tuple[list[Parameter], list[str]]:
    tokens = shlex.split(command)
    if not tokens or tokens[0] != "bitbake":
        raise ValueError(f"not a direct BitBake command: {command}")
    parameters: dict[str, Parameter] = {}
    arguments: list[str] = []
    raw_tokens = tokens[1:]
    for token in raw_tokens:
        if token == "":
            arguments.append("RawArgument::Empty")
            continue
        segments, found = segments_for(token)
        for parameter in found:
            existing = parameters.get(parameter.id)
            if existing is not None and existing != parameter:
                raise ValueError(
                    f"parameter {parameter.id} has conflicting placeholders in {command}"
                )
            parameters[parameter.id] = parameter
        if not found:
            arguments.append(
                f"RawArgument::Literal {{ value: {quoted(token)}.into() }}"
            )
        elif len(segments) == 1 and segments[0][0] == "parameter":
            arguments.append(
                "RawArgument::Parameter { parameter: "
                f"RawParameterId::new({quoted(segments[0][1])}).unwrap() }}"
            )
        elif (
            len(segments) == 2
            and segments[0][0] == "literal"
            and segments[1][0] == "parameter"
            and segments[0][1].startswith("-")
        ):
            arguments.append(
                "RawArgument::JoinedParameter { "
                f"prefix: {quoted(segments[0][1])}.into(), "
                f"parameter: RawParameterId::new({quoted(segments[1][1])}).unwrap() }}"
            )
        else:
            rendered_segments = []
            for kind, value in segments:
                if kind == "literal":
                    rendered_segments.append(
                        f"RawArgumentSegment::Literal {{ value: {quoted(value)}.into() }}"
                    )
                else:
                    rendered_segments.append(
                        "RawArgumentSegment::Parameter { parameter: "
                        f"RawParameterId::new({quoted(value)}).unwrap() }}"
                    )
            arguments.append(
                "RawArgument::Composed { segments: vec!["
                + ", ".join(rendered_segments)
                + "] }"
            )
    return list(parameters.values()), arguments


def capabilities(command: str) -> list[str]:
    tokens = shlex.split(command)[1:]
    required = ["BitBakeRawCli"]

    def add(capability: str) -> None:
        if capability not in required:
            required.append(capability)

    def has(short: str, long: str) -> bool:
        return short in tokens or any(token == long or token.startswith(f"{long}=") for token in tokens)

    if has("-s", "--show-versions"):
        add("BitBakeRawShowVersions")
    if "-c" in tokens or any(token.startswith("--cmd=") for token in tokens):
        add("BitBakeRawTaskExecution")
    if "devshell" in tokens:
        add("DevShell")
    if "menuconfig" in tokens:
        add("MenuConfig")
    if "populate_sdk" in tokens:
        add("SdkPopulate")
    if "populate_sdk_ext" in tokens:
        add("SdkExtensible")
    if "testimage" in tokens or "testimage_auto" in tokens:
        add("TestImage")
    if "listtasks" in tokens:
        add("BitBakeTaskList")
    if has("-f", "--force"):
        add("BitBakeForceTask")
    if has("-C", "--clear-stamp"):
        add("BitBakeRawClearStamp")
    if has("-e", "--environment"):
        add("BitBakeEnvironmentDump")
    if has("-g", "--graphviz"):
        add("BitBakeGraphGeneration")
    if has("-n", "--dry-run"):
        add("BitBakeRawDryRun")
    if has("-p", "--parse-only"):
        add("BitBakeRawParseOnly")
    if has("-k", "--continue"):
        add("BitBakeRawContinue")
    if has("-P", "--profile"):
        add("BitBakeRawProfile")
    if has("-S", "--dump-signatures"):
        add("BitBakeRawDumpSignatures")
    if "--revisions-changed" in tokens:
        add("BitBakeRawRevisionsChanged")
    if has("-b", "--buildfile"):
        add("BitBakeRawBuildFile")
    if any(token.startswith("-D") for token in tokens) or "--debug" in tokens:
        add("BitBakeRawDebug")
    if has("-l", "--log-domains"):
        add("BitBakeRawLogDomains")
    if has("-v", "--verbose"):
        add("BitBakeRawVerbose")
    if any(token in {"-q", "-qq"} for token in tokens) or "--quiet" in tokens:
        add("BitBakeRawQuiet")
    if has("-w", "--write-log"):
        add("BitBakeRawEventLog")
    if has("-u", "--ui"):
        add("BitBakeRawUi")
    if has("-B", "--bind"):
        add("BitBakeRawServerBind")
    if has("-T", "--idle-timeout"):
        add("BitBakeRawServerIdleTimeout")
    if any(token.startswith("--remote-server=") for token in tokens):
        add("BitBakeRawServerRemote")
    if any(token.startswith("--token=") for token in tokens):
        add("BitBakeRawServerToken")
    if "--observe-only" in tokens:
        add("BitBakeRawServerObserve")
    if "--status-only" in tokens:
        add("BitBakeServerStatus")
    if "--server-only" in tokens:
        add("BitBakeServerStart")
    if "--kill-server" in tokens or "-m" in tokens:
        add("BitBakeServerStop")
    if has("-r", "--read"):
        add("BitBakeRawConfigRead")
    if has("-R", "--postread"):
        add("BitBakeRawConfigPostRead")
    if has("-I", "--ignore-deps"):
        add("BitBakeRawIgnoreDeps")
    if any(token.startswith("mc:") for token in tokens):
        add("BitBakeRawMulticonfig")
    if any("--runall=" in token for token in tokens):
        add("BitBakeRawRunAll")
    if any("--runonly=" in token for token in tokens):
        add("BitBakeRawRunOnly")
    if "--no-setscene" in tokens:
        add("BitBakeRawNoSetscene")
    if "--skip-setscene" in tokens:
        add("BitBakeRawSkipSetscene")
    if "--setscene-only" in tokens:
        add("BitBakeRawSetsceneOnly")
    return required


def interaction(command: str) -> str:
    interactive_markers = (
        " -u ",
        "--ui=",
        " -c devshell ",
        " -c pydevshell ",
        " -c menuconfig ",
    )
    padded = f" {command} "
    return "InteractivePty" if any(marker in padded for marker in interactive_markers) else "NoninteractiveJob"


def safety(command: str, capability_names: list[str]) -> str:
    padded = f" {command} "
    if any(marker in padded for marker in (" -c clean ", " -c cleansstate ", " -c cleanall ")):
        return "Destructive"
    if any("Server" in capability for capability in capability_names):
        return "ServerLifecycle"
    if any(capability in {
        "BitBakeRawShowVersions",
        "BitBakeEnvironmentDump",
        "BitBakeGraphGeneration",
        "BitBakeRawDumpSignatures",
        "BitBakeTaskList",
        "BitBakeRawDryRun",
        "BitBakeRawParseOnly",
    } for capability in capability_names):
        return "Inspection"
    return "Build"


def category_kind(category: Category) -> str:
    if category.id == "favorites":
        return "Favorites"
    if category.number in {27, 28}:
        return "CompanionTools"
    if category.number == 29:
        return "Conceptual"
    if not any(entry.executable for entry in category.entries):
        return "ReferenceOnly"
    return "Executable"


def render_parameter(parameter: Parameter) -> str:
    return (
        "RawParameter { "
        f"id: RawParameterId::new({quoted(parameter.id)}).unwrap(), "
        f"label: {quoted(parameter.label)}.into(), "
        f"placeholder: {quoted(parameter.placeholder)}.into(), "
        f"kind: RawParameterKind::{parameter.kind}, "
        "presence: RawParameterPresence::Required }"
    )


def render_command(entry: Entry) -> str:
    command_id = f"{entry.category_id}.l{entry.line:04d}"
    reference_id = f"wrynose-6-0.l{entry.line:04d}"
    common = f"""        RawCommand {{
            id: RawCommandId::new({quoted(command_id)}).unwrap(),
            category: RawCategoryId::new({quoted(entry.category_id)}).unwrap(),
            label: {quoted(entry.command)}.into(),
            description: {quoted(entry.description)}.into(),
            reference: RawReference {{
                id: RawReferenceId::new({quoted(reference_id)}).unwrap(),
                heading: {quoted(entry.heading)}.into(),
                command: {quoted(entry.command)}.into(),
                description: {quoted(entry.description)}.into(),
            }},
"""
    if not entry.executable:
        kind = "ShellPipeline" if entry.command.startswith("bitbake ") else "CompanionTool"
        reason = (
            "Requires a shell pipeline or redirection; Raw Mode never invokes a shell."
            if kind == "ShellPipeline"
            else "Uses a companion tool or shell command; Raw Mode executes only structured BitBake argv."
        )
        return (
            common
            + "            parameters: vec![],\n"
            + "            execution: RawExecutionPolicy::ReferenceOnly {\n"
            + f"                kind: RawReferenceKind::{kind},\n"
            + f"                reason: {quoted(reason)}.into(),\n"
            + "            },\n"
            + "        },\n"
        )

    parameters, arguments = command_parts(entry.command)
    capability_names = capabilities(entry.command)
    capability_rows = ", ".join(f"CapabilityId::{name}" for name in capability_names)
    parameters_text = ",\n                ".join(render_parameter(item) for item in parameters)
    arguments_text = ",\n                    ".join(arguments)
    return (
        common
        + "            parameters: vec![\n"
        + (f"                {parameters_text}\n" if parameters else "")
        + "            ],\n"
        + "            execution: RawExecutionPolicy::Executable {\n"
        + "                template: RawExecutableTemplate {\n"
        + "                    executable: RawExecutable::BitBake,\n"
        + "                    arguments: vec![\n"
        + (f"                    {arguments_text}\n" if arguments else "")
        + "                    ],\n"
        + "                    capabilities: RawCapabilityRequirement::All {\n"
        + f"                        capabilities: vec![{capability_rows}],\n"
        + "                    },\n"
        + f"                    interaction: RawInteractionMode::{interaction(entry.command)},\n"
        + f"                    safety: RawSafetyClass::{safety(entry.command, capability_names)},\n"
        + "                },\n"
        + "            },\n"
        + "        },\n"
    )


def generate() -> str:
    categories, entries = read_reference()
    executable_count = sum(entry.executable for entry in entries)
    category_rows = "".join(
        f"""        RawCategory {{
            id: RawCategoryId::new({quoted(category.id)}).unwrap(),
            label: {quoted(category.label)}.into(),
            reference_heading: {quoted(category.heading)}.into(),
            kind: RawCategoryKind::{category_kind(category)},
        }},
"""
        for category in categories
    )
    command_rows = "".join(render_command(entry) for entry in entries)
    generated = f"""// @generated by scripts/generate-raw-catalog.py; do not edit by hand.
use crate::*;

pub const RAW_REFERENCE_SHA256: &str = {quoted(EXPECTED_SHA256)};
pub const RAW_BUILTIN_CATEGORY_COUNT: usize = {len(categories)};
pub const RAW_BUILTIN_COMMAND_COUNT: usize = {len(entries)};
pub const RAW_BUILTIN_EXECUTABLE_COUNT: usize = {executable_count};

impl RawCatalog {{
    pub fn builtin() -> Self {{
        Self {{
            version: RAW_CATALOG_VERSION,
            categories: vec![
{category_rows}            ],
            commands: vec![
{command_rows}            ],
        }}
    }}
}}

#[cfg(test)]
mod raw_catalog_tests {{
    use super::*;

    fn command(line: usize) -> RawCommand {{
        RawCatalog::builtin()
            .commands
            .into_iter()
            .find(|command| command.reference.id.as_str() == format!("wrynose-6-0.l{{line:04}}"))
            .unwrap()
    }}

    #[test]
    fn raw_catalog_builtin_is_complete_bounded_and_valid() {{
        let catalog = RawCatalog::builtin();
        catalog.validate().unwrap();
        assert_eq!(catalog.categories.len(), RAW_BUILTIN_CATEGORY_COUNT);
        assert_eq!(catalog.commands.len(), RAW_BUILTIN_COMMAND_COUNT);
        assert_eq!(
            catalog.commands.iter().filter(|command| matches!(command.execution, RawExecutionPolicy::Executable {{ .. }})).count(),
            RAW_BUILTIN_EXECUTABLE_COUNT
        );
    }}

    #[test]
    fn raw_catalog_preserves_exact_help_and_structured_templates() {{
        let task = command(167);
        assert_eq!(task.description, "Execute one named task for a recipe.");
        let RawExecutionPolicy::Executable {{ template }} = task.execution else {{ panic!("task command must execute") }};
        assert_eq!(template.display_template(&task.parameters).as_deref(), Some("bitbake -c <task> <recipe>"));

        let joined = command(145);
        let RawExecutionPolicy::Executable {{ template }} = joined.execution else {{ panic!("UI command must execute") }};
        assert!(matches!(template.arguments[0], RawArgument::JoinedParameter {{ .. }}));

        let composed = command(87);
        let RawExecutionPolicy::Executable {{ template }} = composed.execution else {{ panic!("task syntax must execute") }};
        assert!(matches!(template.arguments[0], RawArgument::Composed {{ .. }}));
    }}

    #[test]
    fn raw_catalog_marks_interactive_destructive_empty_and_reference_only_entries() {{
        let devshell = command(607);
        let RawExecutionPolicy::Executable {{ template }} = devshell.execution else {{ panic!("devshell must execute") }};
        assert_eq!(template.interaction, RawInteractionMode::InteractivePty);

        let cleanall = command(593);
        let RawExecutionPolicy::Executable {{ template }} = cleanall.execution else {{ panic!("cleanall must execute") }};
        assert_eq!(template.safety, RawSafetyClass::Destructive);

        let empty_log = command(294);
        let RawExecutionPolicy::Executable {{ template }} = empty_log.execution else {{ panic!("empty event-log argv must execute") }};
        assert!(template.arguments.contains(&RawArgument::Empty));

        assert!(matches!(command(847).execution, RawExecutionPolicy::ReferenceOnly {{ kind: RawReferenceKind::ShellPipeline, .. }}));
        assert!(matches!(command(1932).execution, RawExecutionPolicy::ReferenceOnly {{ kind: RawReferenceKind::CompanionTool, .. }}));
    }}

    #[test]
    fn raw_catalog_does_not_present_conceptual_or_companion_sections_as_executable() {{
        let catalog = RawCatalog::builtin();
        assert_eq!(catalog.category(&RawCategoryId::new("section-29-quick-conceptual-reference").unwrap()).unwrap().kind, RawCategoryKind::Conceptual);
        assert_eq!(catalog.category(&RawCategoryId::new("section-28-yocto-companion-commands").unwrap()).unwrap().kind, RawCategoryKind::CompanionTools);
        assert_eq!(catalog.category(&RawCategoryId::new("favorites").unwrap()).unwrap().kind, RawCategoryKind::Favorites);
    }}
}}
"""
    formatted = subprocess.run(
        ["rustfmt", "--edition", "2024", "--emit", "stdout"],
        input=generated,
        text=True,
        check=True,
        capture_output=True,
    )
    return formatted.stdout


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    generated = generate()
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text(encoding="utf-8") != generated:
            print(f"generated Raw catalog is stale: run {Path(__file__).relative_to(ROOT)}", file=sys.stderr)
            return 1
        return 0
    OUTPUT.write_text(generated, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
