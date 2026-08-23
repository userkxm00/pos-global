# POS Global — External Reference Catalog

This catalog records external open-source projects that may inform future POS Global design, website, mobile, or agent workflows.

## Rules

- References are not dependencies by default.
- Do not copy a repository wholesale into POS Global.
- Reuse code or assets only after license/third-party notice review and only when the implementation is appropriate for POS Global.
- Prefer extracting design patterns and rebuilding them inside our own design system.
- Every reference must remain subordinate to `V2_RULES.md`, accessibility, performance, security, i18n/RTL, offline-first requirements, and the global multi-industry architecture.
- A reference may influence a later task only when that task explicitly calls for it; agents must not proactively add unrelated libraries or features.

## Current references

### MengTo / threeui

Repository: https://github.com/MengTo/threeui

**Potential use:**
- Website visual language and interactive sections.
- POS UI inspiration for polished states, transitions, cards, dialogs, navigation, empty/loading states, and micro-interactions.
- Future design-system exploration where appropriate.

**Constraints:**
- Do not make POS interactions overly decorative or slower for cashiers.
- Preserve keyboard-first, touch-friendly, accessibility, RTL, and low-distraction POS workflows.
- Review `LICENSE`, `ASSET-LICENSES.md`, `FONT-LICENSES.md`, and `THIRD_PARTY_NOTICES.md` before reusing code or assets.

### MengTo / Skills

Repository: https://github.com/MengTo/Skills

**Potential use:**
- Reference for organizing reusable agent skills and workflow instructions.
- Future improvement of agent-facing development workflows.

**Constraints:**
- Do not import external agent rules wholesale.
- POS Global authority remains `V2_RULES.md` + `AGENT_SYSTEM.md` + repository task instructions.

### MengTo / react-native-for-designers

Repository: https://github.com/MengTo/react-native-for-designers

**Potential use:**
- Future mobile UX and interaction reference for the Android/iOS companion.
- Mobile layout and component ideas that can inform the future mobile design system.

**Constraints:**
- Reference only for now; do not add React Native to the current desktop POS stack.
- Future mobile implementation remains aligned with the project's Tauri 2/mobile strategy and shared contracts.

### Lower-priority MengTo references

The following repositories may be useful as inspiration for specific future website/design tasks, but are not current implementation dependencies:

- `MengTo/Spring` — interaction/visual reference.
- `MengTo/DesignerNewsApp` — historical app/UI reference.
- `MengTo/AppStoreSketch` — product/design presentation reference.
- `MengTo/codux-course` — learning/design workflow reference.
- `MengTo/gatsby-starter-designcode` — website starter/reference.

These remain optional references and should only be consulted when a concrete task benefits from them.
