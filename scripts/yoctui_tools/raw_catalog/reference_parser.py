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
