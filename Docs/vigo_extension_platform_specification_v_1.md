# Extension Platform Specification (EPS) — Vigo v1.0

**Product:** Vigo  
**Document Version:** 1.0  
**Status:** Specification Locked — Required for Extension Integration  
**Owner:** Extensions Platform Lead  
**Last Updated:** [Insert Date]

---

# 1. Purpose

This specification defines Vigo's extension platform for v1.0. It covers API compatibility, permission and security model, packaging and signing, store policy, runtime sandboxing, developer tooling, and migration guidance for Chrome/Firefox extensions. The goal is to maximize compatibility while enforcing stronger security and privacy constraints than raw Chromium.

---

# 2. Strategic Goals

- **Compatibility:** Run Chrome Web Store extensions with high fidelity.  
- **Security:** Enforce least-privilege and runtime consent for sensitive APIs.  
- **Transparency:** Provide clear permission UI and extension audit trails.  
- **Performance:** Prevent extensions from degrading core browser responsiveness.  
- **Ecosystem:** Offer a developer portal, SDK, and extension store for signed add-ons.

---

# 3. Extension Runtime Architecture

## 3.1 Process Model
- Each extension runs in its own renderer-like process or shared extension process pool depending on resource constraints.  
- Heavy extensions may run as separate native helper processes (native messaging).  

## 3.2 Capability Isolation
- Extensions interact with browser core through a controlled IPC surface.  
- No extension may access browser core memory or the credential vault directly.

---

# 4. API Compatibility & Mappings

## 4.1 WebExtensions API
- Support the standard WebExtensions API set (chrome.* / browser.*) with compatibility shims for common differences.  
- Provide explicit mappings for APIs that interact with sensitive functionality (e.g., cookies, bookmarks, history).

## 4.2 Deviations & Restrictions
- API calls that expose cross-origin data are restricted by default.  
- WebRequest blocking is supported but with a declarativeNetRequest preferred model to limit runtime hook abuse.  

---

# 5. Permission Model

## 5.1 Permission Types
- **Basic Permissions:** tabs, storage, contextMenus — granted at install.  
- **Sensitive Permissions:** cookies, bookmarks, history, webRequest blocking — require runtime confirmation and explicit justification displayed in the store.  
- **Power Permissions:** nativeMessaging, proxy, devtools — require elevated review and per-install consent.

## 5.2 Granular & Runtime Approvals
- Permission grants can be scoped by origin and time (e.g., allow access to example.com for 24 hours).  
- Users can revoke permissions at any time through the Extensions UI.

---

# 6. Declarative vs Imperative APIs

- Prefer **declarative** APIs (declarativeNetRequest) that express intents rather than giving extensions live access to all traffic.  
- Imperative hooks allowed only for developer-reviewed extensions or when user explicitly enables.

---

# 7. Extension Packaging & Signing

## 7.1 Package Format
- ZIP-based CRX-compatible package with Vigo manifest fields.  

## 7.2 Signing & Verification
- All published extensions must be signed by Vigo’s extension authority.  
- Extensions installed from outside the store are flagged and require explicit user confirmation and temporary developer mode enabling.

---

# 8. Store & Review Process

## 8.1 Submission Requirements
- Developer identity verification.  
- Privacy policy required for extensions requesting sensitive permissions.  
- Automated static analysis and manual review for initial publish.

## 8.2 Automated Vetting
- Static code scanning for known malicious patterns.  
- Permissions-to-behavior heuristics to detect over-privileged extensions.  

## 8.3 Manual Review & Escalation
- High-risk extensions enter manual review queue.  
- Escalate to security team for nativeMessaging, proxy, or payment-related features.

---

# 9. Runtime Security Controls

- Per-origin permission evaluation for webRequest and cookie access.  
- Rate limiting for extension-initiated network requests to prevent abuse.  
- CPU & memory usage throttles; extensions exceeding T thresholds may be suspended with user notification.  

---

# 10. Native Messaging & Native Helpers

- Native messaging allowed via signed host manifests.  
- Vigo will provide native helper SDKs for common integration patterns (e.g., hardware control).  
- Native helpers must be signed and validated during installation.

---

# 11. Privacy & Data Handling

- Extensions must disclose data collection in the manifest and privacy policy.  
- By default, the browser blocks third-party telemetry from extensions unless the extension provides opt-in consent flows.

---

# 12. Developer Tooling & SDK

- **Extension CLI:** Packaging, signing, and linting tools.  
- **Local Sandbox:** Run extensions in dev mode with simulated permission prompts and telemetry mocks.  
- **Debugger:** Extension live inspector integrated into DevTools.  
- **Guides & Samples:** Migration guide from Chrome extensions focusing on permission minimization.

---

# 13. Migration & Compatibility Guidance

- Provide automated compatibility checker for Chrome extensions that flags deprecated APIs and suggests declarative alternatives.  
- Offer polyfills for common Chrome-only APIs where safe.

---

# 14. Abuse & Incident Response

- Extension takedown process with automated blocking for known-malicious signatures.  
- User-level reporting flow; telemetry to correlate reports with extension behavior.  
- Developer appeal and remediation workflow.

---

# 15. Performance SLAs for Extensions

- Extension CPU usage should not exceed 15% of a single core sustained in idle conditions.  
- Memory budgets applied per extension; alerts when thresholds crossed.  
- Extensions causing severe regressions will be auto-disabled in canary/channel builds pending review.

---

# 16. Observability & Audit Logs

- Transparent audit logs for extension permission grants / revocations.  
- Admin-viewable event logs for enterprise-managed installs (future scope).  

---

# 17. Legal & Policy Requirements

- Require compliance with applicable privacy laws (GDPR, CCPA) for published extensions.  
- Copyright and export controls enforced during review for native binaries.

---

# 18. Roadmap & Phasing

**Phase 1 (V1):** Chrome extension compatibility, permission UI, signing, basic review flow.  
**Phase 2 (V1.1):** Developer portal, automated vetting improvements, native helper SDKs.  
**Phase 3 (V1.2):** Public Vigo Extension Store, curated collections, revenue models for developers.

---

# 19. Appendix

- Manifest schema (v1) — includes additional Vigo-specific fields (privacy rationale, resource budgets).  
- Sample permission prompts and UX copy.  
- CLI command references.

---

**End of Extension Platform Specification (EPS) v1.0**
