def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    generated = generate_files()
    if args.check:
        actual_paths = {OUTPUT}
        if OUTPUT_DIR.exists():
            actual_paths.update(OUTPUT_DIR.glob("*.rs"))
        stale = actual_paths != set(generated) or any(
            not path.exists() or path.read_text(encoding="utf-8") != source
            for path, source in generated.items()
        )
        if stale:
            print(f"generated Raw catalog is stale: run {Path(__file__).relative_to(ROOT)}", file=sys.stderr)
            return 1
        return 0
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    for stale in OUTPUT_DIR.glob("*.rs"):
        stale.unlink()
    for path, source in generated.items():
        path.write_text(source, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
