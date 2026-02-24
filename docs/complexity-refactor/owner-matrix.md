# Complexity Refactor Owner Matrix

Date: 2026-02-19

## Global roles

| Role | Owner | Responsibilities |
| --- | --- | --- |
| Tech Lead (DRI) | Platform Lead | Scope, sequencing, merge readiness, rollback decision owner |
| QA Owner | Test Engineering Lead | Regression matrix, flaky tracking, release confidence sign-off |
| Architect Reviewer | Staff Engineer | Boundary decisions, dependency direction, long-term maintainability review |
| Incident Escalation | Engineering Manager | Stop-the-line authorization and cross-team coordination |

## Phase ownership

| Phase | DRI | Required approvers | QA sign-off | Escalation path |
| --- | --- | --- | --- | --- |
| Phase 0 governance/artifacts | Platform Lead | Architect Reviewer | QA Owner | Engineering Manager |
| Baseline tests and output contracts | QA Owner | Tech Lead | QA Owner | Engineering Manager |
| `main.rs` runtime shell + helper extraction | Platform Lead | Architect Reviewer | QA Owner | Engineering Manager |
| Download engine decomposition | Platform Lead | Architect Reviewer | QA Owner | Engineering Manager |
| Queue/DB seam introduction | Platform Lead | Architect Reviewer | QA Owner | Engineering Manager |
| Error model hardening | Platform Lead | Architect Reviewer | QA Owner | Engineering Manager |
| Non-functional regression gates and rollout | QA Owner | Tech Lead + Architect Reviewer | QA Owner | Engineering Manager |
