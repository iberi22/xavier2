# Known Vulnerabilities & Warnings

*Last updated: 2026-05-06*

## Status Summary

All **actual CVE vulnerabilities** reported by Dependabot have been resolved.
The remaining items are informational warnings (unmaintained crates) and one unsoundness issue with no available patch.

---

## Resolved Vulnerabilities ✅

### Rust (Cargo.lock)
| Package | CVE/GHSA | Severity | Resolution |
|---------|----------|----------|------------|
| openssl | CVE-2026-42327 | High | Updated to 0.10.79 |
| openssl | CVE-2026-41676 | High | Updated to 0.10.79 |
| openssl | CVE-2026-41678 | High | Updated to 0.10.79 |
| openssl | CVE-2026-41681 | High | Updated to 0.10.79 |
| openssl | CVE-2026-41898 | High | Updated to 0.10.79 |
| openssl | CVE-2026-41677 | Low | Updated to 0.10.79 |
| rustls-webpki | GHSA-82j2-j2ch-gfr8 | High | Updated to 0.103.13 |
| rand | GHSA-cq8v-f236-94qc | Low | Updated to 0.8.6 / 0.9.4 |

### npm (package-lock.json)
| Package | CVE | Severity | Resolution |
|---------|-----|----------|------------|
| postcss | CVE-2026-41305 | Medium | Updated to 8.5.14 |
| astro | CVE-2026-41067 | Medium | Updated to 6.2.2 |
| @astrojs/node | CVE-2026-41322 | Medium | Updated to 10.0.5 |

---

## Unresolved: Informational Warnings Only (No CVE)

These are not security vulnerabilities. They are advisories about crate maintenance status
or theoretical soundness issues that require specific runtime conditions to trigger.

### Main workspace (Cargo.lock)
| Crate | Advisory | Type | Reason Unresolved |
|-------|----------|------|-------------------|
| `paste` 1.0.15 | RUSTSEC-2024-0436 | Unmaintained | Transitive via `ratatui`. No security vulnerability — crate still works. Replace requires `ratatui` to migrate away. |
| `proc-macro-error` 1.0.4 | RUSTSEC-2024-0370 | Unmaintained | Transitive via `teloxide` (→ `aquamarine`). No security vulnerability. |
| `rustls-pemfile` 1.0.4 | RUSTSEC-2025-0134 | Unmaintained | Transitive via `reqwest 0.11` (→ `teloxide-core`). No security vulnerability. |
| `lru` 0.12.5 | RUSTSEC-2026-0002 | Unsound | Transitive via `ratatui 0.29`. Fix requires `lru >= 0.16.3` but `ratatui` pins `^0.12`. No CVE assigned. Requires `ratatui` to update dependency. |

### panel-ui workspace (panel-ui/src-tauri/Cargo.lock)
| Crate | Advisory | Type | Reason Unresolved |
|-------|----------|------|-------------------|
| GTK3 bindings (atk, gdk, gtk, etc.) | RUSTSEC-2024-04xx | Unmaintained | Transitive via `tauri 2.x` / `wry`. Tauri actively maintained but uses GTK3 bindings. |
| `fxhash` 0.2.1 | RUSTSEC-2025-0057 | Unmaintained | Transitive via `tauri-utils`. |
| `unic-*` packages | RUSTSEC-2025-00xx | Unmaintained | Transitive via `tauri-utils`. |
| `glib` 0.18.5 | RUSTSEC-2024-0429 | Unsound | Transitive via Tauri GTK stack. Theoretical unsoundness. |
| `rand` 0.7.3 | RUSTSEC-2026-0097 | Unsound | Transitive via `phf_generator`. Pinned by old dependency. Not practical to fix without upstream update. |

### npm (transitive, no fix available)
| Package | Advisory | Severity | Reason Unresolved |
|---------|----------|----------|-------------------|
| `prismjs` < 1.30.0 | GHSA-x7hr-w5r2-h6wg | Moderate | Transitive via `@openuidev/react-ui` → `react-syntax-highlighter` → `refractor`. No patch available for prismjs. |

---

## Monitoring

- All unresolvable items are **informational only** (no CVE, no exploit path in our usage).
- Run `cargo audit` periodically to check if upstream updates have fixed any of these.
- Run `npm audit` to check npm transitive dependencies.
