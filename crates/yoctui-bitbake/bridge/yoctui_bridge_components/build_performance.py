def performance_config_params(module, tinfoil_module):
    """Apply reviewed job-owned cache/PSI settings through BitBake's native postread API."""
    import hashlib
    import stat
    import tempfile

    encoded = os.environ.get("YOCTUI_BUILD_PERFORMANCE")
    cache = cache_profile_assignments(os.environ.get("YOCTUI_SSTATE_PROFILE"))
    if not encoded and not cache:
        return None
    version = getattr(module, "__version__", "")
    try:
        major, minor = (int(v) for v in version.split(".")[:2])
    except (ValueError, TypeError):
        raise RuntimeError("cannot verify BitBake pressure-control support") from None
    if (major, minor) < (2, 0):
        raise RuntimeError("Cache profiles require BitBake 2.0 or newer")
    content = "# Yoctui reviewed per-build optimization profile\n"
    if encoded:
        profile = json.loads(encoded)
        if (
            not isinstance(profile, dict)
            or set(profile) != {"cpu", "io", "memory"}
            or any(
                type(v) is not int or not 1 <= v <= 1000000 for v in profile.values()
            )
        ):
            raise RuntimeError("invalid build pressure profile")
        if (major, minor) < (2, 2):
            raise RuntimeError("Balanced requires BitBake 2.2 or newer")
        for name in ("cpu", "io", "memory"):
            with open(f"/proc/pressure/{name}", encoding="ascii") as pressure:
                if "some avg10=" not in pressure.read(4096):
                    raise RuntimeError(f"host PSI {name} is unavailable")
        content += "".join(
            f'BB_PRESSURE_MAX_{key.upper()} = "{profile[key]}"\n'
            for key in ("cpu", "io", "memory")
        )
    content += cache
    # A stable filename/content avoids reparsing solely because a temporary name changed.
    # An owner-only directory and O_NOFOLLOW protect against symlink substitution.
    directory = os.path.join(tempfile.gettempdir(), f"yoctui-performance-{os.getuid()}")
    try:
        os.mkdir(directory, 0o700)
    except FileExistsError:
        pass
    info = os.lstat(directory)
    if (
        not stat.S_ISDIR(info.st_mode)
        or info.st_uid != os.getuid()
        or stat.S_IMODE(info.st_mode) != 0o700
    ):
        raise RuntimeError("unsafe build performance cache directory")
    digest = hashlib.sha256(content.encode()).hexdigest()
    path = os.path.join(directory, digest + ".conf")
    try:
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    except FileExistsError:
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
        with os.fdopen(fd, "r", encoding="utf-8") as saved:
            if saved.read(16384) != content:
                raise RuntimeError("build performance profile content mismatch")
    else:
        with os.fdopen(fd, "w", encoding="utf-8") as output:
            output.write(content)
    # Set the constructor option: updateToServer reads params.options.postfile,
    # so assigning only params.postfile would leave the server configuration unchanged.
    return tinfoil_module.TinfoilConfigParameters(
        config_only=True, quiet=2, postfile=[path]
    )
