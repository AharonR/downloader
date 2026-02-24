# Open Tasks

A single place to track open work: bugs, improvements, refactoring, and follow-up items. Add items here when they are out of scope for the current effort or discovered during review.

---

## Refactoring & code quality

- **Resolver: shared meta-tag regexes** — Move identical `TITLE_RE`, `DOI_RE`, and citation `YEAR_RE` from IEEE/Springer into `src/resolver/utils.rs` to reduce duplication and centralize regex maintenance.
- **Resolver: response-body error handling** — Deduplicate the “response body could not be parsed” (or similar) error pattern across resolvers; consider a small helper or macro for consistent messages.
- **Resolver: Crossref `extract_year`** — Crossref uses `extract_year(Option<&CrossrefDate>) -> Option<i32>`; document or align with the string-based `extract_year_from_str` pattern if useful for consistency.

---

## Bugs & fixes

*(Add items as they are discovered or deferred.)*

---

## Features & enhancements

*(Future product or UX improvements that are not in the current roadmap.)*

---

## Documentation & process

*(Docs updates, checklist improvements, or process changes.)*

---

## How to use

- **Add:** One bullet per task; optional `**Label:**` for area (e.g. module name).
- **Complete:** Move the bullet to a “Done” subsection at the bottom of the relevant section, or delete it and note in CHANGELOG if notable.
- **Scope:** Keep descriptions short; link to issues, plans, or code when helpful.
