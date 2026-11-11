def cache_profile_assignments(encoded):
    """Validate literal values before giving them to BitBake's native parser."""
    from urllib.parse import urlsplit

    if not encoded:
        return ""
    settings = json.loads(encoded)
    names = {
        "sstate_directory",
        "downloads_directory",
        "mirror_url",
        "hash_server",
        "hash_upstream",
    }
    if not isinstance(settings, dict) or set(settings) != names:
        raise RuntimeError("invalid sstate cache profile fields")
    for value in settings.values():
        if (
            not isinstance(value, str)
            or len(value.encode()) > 2048
            or any(ord(c) < 32 or 127 <= ord(c) <= 159 or c in '$"\\`' for c in value)
        ):
            raise RuntimeError("cache settings require bounded literal values")

    def directory(value):
        return value.startswith("/") and value != "/" and ".." not in value.split("/")

    def endpoint(value, schemes):
        if not value.isascii() or any(c in value for c in "@?#; "):
            return False
        try:
            parsed = urlsplit(value)
            return (
                parsed.scheme in schemes
                and bool(parsed.hostname)
                and (parsed.port is None or parsed.port > 0)
            )
        except ValueError:
            return False

    def hash_endpoint(value):
        return (
            endpoint(value, ("ws", "wss"))
            or (value.startswith("unix://") and directory(value[7:]))
            or (
                "/" not in value
                and ":" in value
                and endpoint("tcp://" + value, ("tcp",))
                and urlsplit("tcp://" + value).port is not None
            )
        )

    for name in ("sstate_directory", "downloads_directory"):
        if settings[name] and not directory(settings[name]):
            raise RuntimeError("cache paths must be absolute without parent components")
    mirror = settings["mirror_url"]
    if mirror:
        if not (
            endpoint(mirror, ("http", "https"))
            or (
                mirror.startswith("file://")
                and directory(mirror[7:])
                and not any(c in mirror for c in "@?#; ")
            )
        ):
            raise RuntimeError("invalid cache mirror; use .netrc for credentials")
        if mirror.split("/").count("PATH") > int(mirror.endswith("/PATH")):
            raise RuntimeError("PATH may only terminate a mirror template")
    server, upstream = settings["hash_server"], settings["hash_upstream"]
    if server and server != "auto" and not hash_endpoint(server):
        raise RuntimeError("invalid hash server endpoint")
    if upstream and (server != "auto" or not hash_endpoint(upstream)):
        raise RuntimeError("hash upstream requires a local auto hash server")
    if not any(settings.values()):
        raise RuntimeError("empty cache profile")
    assignments = []
    for field, variable in (
        ("sstate_directory", "SSTATE_DIR"),
        ("downloads_directory", "DL_DIR"),
    ):
        if settings[field]:
            assignments.append((variable, settings[field]))
    if mirror:
        template = mirror if mirror.endswith("/PATH") else mirror.rstrip("/") + "/PATH"
        assignments.append(
            ("SSTATE_MIRRORS", f"file://.* {template};downloadfilename=PATH")
        )
    if server:
        assignments.extend(
            (("BB_HASHSERVE", server), ("BB_SIGNATURE_HANDLER", "OEEquivHash"))
        )
    if upstream:
        assignments.append(("BB_HASHSERVE_UPSTREAM", upstream))
    return "".join(f'{key} = "{value}"\n' for key, value in assignments)
