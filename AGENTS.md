# Repository Guidelines

## Project Structure & Module Organization
- `src/`: Vue 3 + TypeScript frontend (`App.vue`, `ChatApp.vue`, `SettingsApp.vue`, composables, and global styles).
- `src/assets/pets/`: pet animation assets (GIFs) grouped by pet/action.
- `src-tauri/src/`: Rust backend for desktop integration, config, and LLM/tool modules (`llm/tools/*`).
- `src-tauri/capabilities/` and `src-tauri/gen/schemas/`: Tauri capability and schema definitions.
- `资源/`: product and design documents (Chinese) used for planning.
- Build outputs: `dist/` (frontend) and Tauri target artifacts.

## Build, Test, and Development Commands
- `npm run dev`: start Vite dev server for frontend-only work.
- `npm run build`: type-check/transpile and build frontend into `dist/`.
- `npm run tauri dev`: run the desktop app with hot reload (frontend + Rust backend).
- `npm run tauri build`: build distributable desktop binaries.
- `cargo check --manifest-path src-tauri/Cargo.toml`: fast Rust compile check for backend changes.

## Coding Style & Naming Conventions
- TypeScript/Vue: 2-space indentation, `camelCase` for vars/functions, `PascalCase` for Vue components.
- Rust: follow `rustfmt` defaults, `snake_case` for modules/functions, `PascalCase` for structs/enums.
- Composables follow `useXxx` naming (for example `usePetState.ts`).
- Keep modules focused: UI state in `src/composables`, OS/tooling logic in `src-tauri/src/llm/tools`.

## Testing Guidelines
- No full automated test suite is committed yet; validate changes with targeted checks:
  - Frontend: `npm run build` and manual UI smoke test in `npm run tauri dev`.
  - Rust backend: `cargo check --manifest-path src-tauri/Cargo.toml`.
- When adding tests, place frontend tests near source files and Rust tests in `#[cfg(test)]` modules.

## Commit & Pull Request Guidelines
- Existing history mixes short Chinese summaries and Conventional Commits (for example `feat: ...`).
- Preferred format going forward: `<type>: <brief description>` (`feat`, `fix`, `refactor`, `docs`, `chore`).
- Keep commits scoped and atomic; include affected area when helpful (for example `fix(llm-tools): handle timeout`).
- PRs should include: purpose, key changes, verification steps, and screenshots/GIFs for UI or animation changes.
