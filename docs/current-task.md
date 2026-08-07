# Current Task

## Task

**ID:** UTIL-MENU-001
**Title:** Add contextual utility menus and expert argument forms
**Status:** NOT_STARTED

## Objective

Add a Utilities workspace and contextual typed menus, with safe expert argv
forms, exact previews, warnings, history, cancellation, and output inspection.

## Verification

```bash
cargo test -p yoctui-model utility_menu
cargo test -p yoctui-app utility_menu
cargo test -p yoctui-ui utility_menu
cargo test -p yoctui -- utility_menu
```

## Definition of done

- Common operations use typed forms; expert operations use the shared shell-free
  runner and preserve exact previews, warnings, history, and cancellation.

## Next task

After completion, select `UTIL-DEVTOOL-001`.
