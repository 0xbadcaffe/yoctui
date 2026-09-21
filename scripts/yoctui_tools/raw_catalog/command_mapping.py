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
