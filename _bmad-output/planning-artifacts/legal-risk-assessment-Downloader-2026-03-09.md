---
date: 2026-03-09
author: fierce
status: draft
type: companion
parent: product-brief-Downloader-2026-03-08.md
audit: audit-10-expert-product-strategy-2026-03-09.md
findings_addressed: [10, 11]
---

# Legal Risk Assessment: Downloader

**Date:** 2026-03-09
**Purpose:** Document legal and ethical risks of batch downloading academic content, current mitigations, and recommended additions.
**Audit findings addressed:** #10 (no legal risk assessment), #11 (GDPR not addressed for team phase)

---

## Scope

This assessment covers:
- Publisher Terms of Service (ToS) risks for automated batch downloading
- Open Access vs licensed content handling
- Institutional proxy and authentication considerations
- Current technical mitigations in the codebase
- Recommended additions for risk reduction
- GDPR and privacy considerations for future team/enterprise features

This is a product risk assessment, not legal advice. Consult qualified legal counsel before making compliance decisions.

---

## Publisher ToS Analysis

### General Landscape

Most major academic publishers prohibit or restrict automated/systematic downloading in their Terms of Service. The specific language varies, but the pattern is consistent: bulk automated access is treated differently from individual manual access.

### Publisher-Specific Analysis

#### Elsevier (ScienceDirect)

| Aspect | Detail |
|--------|--------|
| ToS Position | Prohibits systematic downloading, text mining without agreement |
| Open Access | CC-licensed OA articles allow broader use including text mining |
| Licensed Access | Institutional subscriptions restrict automated bulk access |
| Risk Level | **High** for licensed content, **Low** for OA content |
| Downloader Exposure | ScienceDirect resolver exists; must respect access boundaries |

#### Springer Nature

| Aspect | Detail |
|--------|--------|
| ToS Position | Restricts systematic electronic data retrieval without permission |
| Open Access | SpringerOpen and BioMed Central content under CC licenses |
| Licensed Access | Requires institutional or individual subscription |
| Risk Level | **Medium-High** for licensed, **Low** for OA |
| Downloader Exposure | Springer resolver exists; similar boundary requirements |

#### Wiley

| Aspect | Detail |
|--------|--------|
| ToS Position | Prohibits systematic downloading or automated scraping |
| Open Access | Wiley Open Access under CC licenses |
| Licensed Access | Institutional subscriptions with usage restrictions |
| Risk Level | **Medium-High** for licensed, **Low** for OA |
| Downloader Exposure | No dedicated resolver; accessed via Direct resolver |

#### IEEE

| Aspect | Detail |
|--------|--------|
| ToS Position | IEEE Xplore ToS restricts automated access and bulk downloading |
| Open Access | IEEE Open Access articles available under CC |
| Licensed Access | IEEE membership or institutional subscription |
| Risk Level | **High** for licensed content, **Low** for OA |
| Downloader Exposure | IEEE resolver exists; must handle access restrictions |

#### ACM

| Aspect | Detail |
|--------|--------|
| ToS Position | ACM Digital Library ToS restricts systematic downloading |
| Open Access | ACM Open Access initiative covers many proceedings |
| Licensed Access | ACM membership or institutional subscription |
| Risk Level | **Medium** for licensed, **Low** for OA |
| Downloader Exposure | No dedicated resolver; accessed via Direct resolver |

---

## Open Access vs Licensed Content

### Clear Categories

| Content Type | Automated Access Risk | Notes |
|-------------|----------------------|-------|
| Gold OA (CC-BY, CC-BY-SA) | **Low** | License explicitly permits redistribution and text mining |
| Green OA (preprints, author manuscripts) | **Low** | Self-archived by authors, typically no publisher restrictions |
| Bronze OA (free to read, no license) | **Medium** | Publisher grants free reading but may restrict automated access |
| Hybrid (subscription + OA option) | **Varies** | Depends on whether specific article is OA |
| Subscription/Licensed | **High** | Publisher ToS typically restricts automated access |

### Downloader's Position

Downloader does not distinguish between OA and licensed content during acquisition. It attempts to download whatever the URL resolves to. This is both a strength (simplicity) and a risk (may violate ToS for licensed content).

**Recommendation:** Consider adding optional OA-awareness: detect known OA repositories (arXiv, PubMed Central, DOAJ-listed journals) and flag non-OA sources in the completion summary for user awareness.

---

## Institutional Proxy and Authentication

### Current State

- Downloader supports cookie jar passthrough for authenticated sessions
- Resolvers detect auth requirements and return `NeedsAuth` status
- No built-in proxy authentication or institutional SSO support

### Risk Considerations

- Institutional proxy access is typically governed by the institution's license agreement with publishers
- Automated bulk downloading through institutional proxies may violate both publisher ToS AND institutional acceptable use policies
- Libraries negotiate access terms that typically assume human-scale usage

### Recommendations

1. Document clearly that institutional proxy usage is the user's responsibility
2. Add a user acknowledgment step when proxy/cookie authentication is configured
3. Provide guidance on appropriate rate limits for institutional access
4. Consider per-publisher rate limit profiles (see below)

---

## Current Technical Mitigations

### robots.txt Compliance (`download/robots.rs`)

- Fetches and caches robots.txt per origin with 24-hour TTL
- Respects `User-agent: *` and `Disallow` rules
- Returns `RobotsDecision::Allowed` or `RobotsDecision::Disallowed`
- 404 response treated as "no restrictions" (standard behavior)
- **Assessment:** Good baseline. Demonstrates good faith in respecting publisher preferences.

### Per-Domain Rate Limiter (`download/rate_limiter.rs`)

- Configurable minimum delay between requests to the same domain
- Optional jitter to avoid regular spacing and fingerprinting
- Cumulative delay tracking with warnings for excessive rate limiting
- Retry-After header parsing and respect (RFC 7231)
- Stale domain cleanup to prevent memory leaks
- **Assessment:** Strong implementation. Per-domain isolation prevents cross-domain delays. Jitter support shows sophistication.

### No Paywall Circumvention

- Resolvers do not attempt to bypass paywalls or access controls
- Auth requirement is surfaced to the user, not worked around
- No Sci-Hub integration, no proxy rotation, no CAPTCHA solving
- **Assessment:** Critical legal protection. This should remain a firm product boundary.

### No DRM Bypass

- No PDF decryption, no DRM stripping, no access-control circumvention
- Files are downloaded as-is from the source
- **Assessment:** DMCA/copyright safe harbor protection. Must remain absolute.

---

## Recommended Additions

### Priority 1: User ToS Acknowledgment

Add a first-run acknowledgment that the user is responsible for:
- Complying with publisher Terms of Service
- Ensuring they have legitimate access to content they download
- Using the tool in accordance with institutional policies

Implementation: CLI prompt on first run, stored in config. Not a legal shield, but demonstrates the tool's intent.

### Priority 2: Per-Publisher Rate Limit Profiles

Define conservative default rate limits per publisher domain:

| Publisher Domain | Suggested Default Delay |
|-----------------|------------------------|
| sciencedirect.com | 3-5 seconds |
| springer.com / link.springer.com | 3-5 seconds |
| ieeexplore.ieee.org | 3-5 seconds |
| wiley.com / onlinelibrary.wiley.com | 3-5 seconds |
| dl.acm.org | 3-5 seconds |
| arxiv.org | 1-2 seconds (more permissive, OA) |
| ncbi.nlm.nih.gov | 1-2 seconds (NIH fair use) |

These should be configurable but default to conservative values.

### Priority 3: Clear Documentation

- README section on "Responsible Use"
- Per-resolver documentation of what access patterns are used
- Guidance on institutional proxy usage
- FAQ on publisher ToS compliance

### Priority 4: Completion Summary Enhancements

- Flag sources that required authentication (already present as `NeedsAuth`)
- Distinguish OA vs potentially-licensed sources in summary
- Recommend user review for sources from known paywalled publishers

---

## Risk Matrix

| Publisher | Content Type | Current Mitigation | Risk Level | Recommended Action |
|-----------|-------------|-------------------|------------|-------------------|
| Elsevier | Licensed | robots.txt + rate limit | **High** | Per-publisher rate profile, ToS acknowledgment |
| Elsevier | OA | robots.txt + rate limit | **Low** | Maintain current mitigations |
| Springer | Licensed | robots.txt + rate limit | **Medium-High** | Per-publisher rate profile |
| Springer | OA | robots.txt + rate limit | **Low** | Maintain current mitigations |
| IEEE | Licensed | robots.txt + rate limit | **High** | Per-publisher rate profile, ToS acknowledgment |
| IEEE | OA | robots.txt + rate limit | **Low** | Maintain current mitigations |
| Wiley | Licensed | robots.txt + rate limit | **Medium-High** | Per-publisher rate profile |
| ACM | Licensed | robots.txt + rate limit | **Medium** | Per-publisher rate profile |
| arXiv | OA | robots.txt + rate limit | **Low** | Maintain; arXiv is OA-friendly |
| PubMed Central | OA | robots.txt + rate limit | **Low** | Maintain; NIH open access policy |
| YouTube | Varies | robots.txt + rate limit | **Medium** | YouTube ToS restricts automated access; document limitations |

---

## GDPR Considerations (Phase 3: Team Features)

### When GDPR Applies

GDPR becomes relevant when Downloader handles personal data in team/enterprise contexts:
- User accounts and authentication data
- Download history and usage logs per user
- Shared corpus access patterns
- Any cloud/hosted deployment serving EU users

### Key Requirements

| Requirement | Applicability | Notes |
|-------------|--------------|-------|
| Lawful basis for processing | Team features with user data | Legitimate interest or consent |
| Data minimization | Usage logging, telemetry | Collect only what's needed |
| Right to erasure | User accounts | Must support account deletion |
| Data portability | User data export | Standard format export |
| Privacy by design | Architecture | Build privacy into team features from the start |
| Data processing agreements | Cloud/hosted | Required for hosted service |

### Recommendations for Phase 3

1. Design team features with data minimization from the start
2. Keep user data local by default; cloud sync as opt-in
3. Implement data export and deletion capabilities early
4. Document data flows before building team infrastructure
5. Consider GDPR-compliant hosting locations for any cloud components

---

## Summary and Next Steps

### Current Risk Posture

Downloader's current mitigations (robots.txt compliance, rate limiting, no paywall circumvention, no DRM bypass) provide a reasonable baseline for an open-source research tool. The tool's design philosophy of surfacing access restrictions rather than circumventing them is the correct approach.

### Key Gaps

1. No user-facing ToS acknowledgment
2. Rate limits are uniform rather than per-publisher
3. No distinction between OA and licensed content in output
4. No documentation of responsible use expectations
5. GDPR not yet considered for Phase 3 architecture

### Recommended Timeline

| Action | Priority | Phase |
|--------|----------|-------|
| User ToS acknowledgment | P1 | Phase 1 |
| Responsible use documentation | P1 | Phase 1 |
| Per-publisher rate limit profiles | P2 | Phase 1-2 |
| OA vs licensed flagging in output | P3 | Phase 2 |
| GDPR-aware team architecture | P2 | Phase 3 design |
