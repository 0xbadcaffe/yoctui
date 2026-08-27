# Current Task

## Task

**ID:** UX-CONCEPT-EDITOR-MENU-001
**Title:** Compose the recipe editor with the application menu
**Status:** IN_PROGRESS

## Objective

Compose the production recipe editor with the focus-trapped F10 application
menu visible over it, preserving menu ownership, exact disabled reasons,
validation, diff state, and editor actions.

## Dependencies

- UX-CONCEPT-ACCEPTANCE-001 — DONE

## Definition of done

- The real recipe editor and F10 menu coexist at 160×50.
- The menu owns focus and traps input while open.
- Disabled actions expose exact reasons.
- Validation, diff state, and save/build/return actions remain visible.
- Focused production-renderer and input-routing tests pass.

## Verification

```bash
cargo test -p yoctui-ui concept_editor_application_menu
cargo test -p yoctui-app ux_menu
```
