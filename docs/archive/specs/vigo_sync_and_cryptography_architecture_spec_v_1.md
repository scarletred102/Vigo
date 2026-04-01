# Sync & Cryptography Architecture Specification — Vigo v1.0

**Product:** Vigo  
**Document Version:** 1.0  
**Status:** Architecture Locked — Required for Implementation  
**Owner:** Cryptography Lead / Sync Systems Lead  
**Last Updated:** [Insert Date]

---

## Executive summary
Vigo's sync system provides **end-to-end encrypted (E2E) synchronization** of user data (bookmarks, passwords, history, settings, open tabs) across desktop devices while preserving zero-knowledge server guarantees: servers store only ciphertext and cannot decrypt user data. The design provides multiple device enrolment and recovery flows, integrates with platform authenticators (Windows Hello, Touch ID / Secure Enclave) for convenient strong-authentication, and supports optional self-hosted sync endpoints for privacy-conscious users and enterprise deployments.

Security and usability trade-offs are explicit: secure default flows favor zero-knowledge, while recovery options (recovery passphrase, recovery token) are available with clear risk disclosures.

This document specifies primitives, protocols, data models, server APIs, device registration and pairing flows, key rotation and revocation, storage layout, threat model and mitigations, testing requirements, and operational guidance.

---

## Table of contents
1. Goals & Constraints
2. Threat Model & Assumptions
3. Cryptographic Primitives (recommended)
4. High-level Design
5. Device Onboarding Flows (Password, Passkey, Recovery Token)
6. Sync Data Model & Storage Format
7. Server API Surface
8. Key Management & Rotation
9. Recovery & Account Recovery Models
10. Revocation & Remote Wipe
11. Metadata Minimization & Privacy
12. Integration with Platform Authenticators
13. Backup Export / Import Format
14. Operational & Scalability Considerations
15. Monitoring, Testing & Compliance
16. Appendices (Sequence diagrams, JSON schemas)

---

## 1. Goals & Constraints
**Goals**
- Provide E2E encrypted sync for bookmarks, passwords, history, settings, and open tabs.
- Server holds **zero-knowledge** encrypted blobs; no plaintext stored server-side.
- Support multi-device sync, secure device addition, and secure device revocation.
- Integrate with Windows Hello / Touch ID for UX-friendly authentication and key protection.
- Offer optional self-hosting of the sync server for users/enterprises.

**Constraints**
- Cross-platform implementation (Windows, macOS, Linux) using WebCrypto where possible and libsodium/portable libraries where native APIs are richer.
- Minimize user friction for onboarding and recovery.
- Protect against server compromise, passive and active network attackers, and stolen-device scenarios.

---

## 2. Threat Model & Assumptions
**Adversaries**
- Malicious server operator or server compromise (must not be able to read user data).
- Network-level MITM (TLS is required for transport but not trusted for confidentiality of content).
- Local device compromise (attacker with OS-level access).  
- Stolen device with enrolled credentials but without biometric/OS unlock.

**Assumptions**
- Clients execute correct crypto (no compromised client build).  
- TLS used for all transport; servers authenticated.  
- Platform authenticators provide hardware-backed key protection where available.

**Security Objectives**
- Confidentiality: server unable to decrypt sync blobs.
- Integrity/authenticity: clients detect tampering of synced blobs.
- Forward secrecy: compromise of server keys should not reveal historical content.
- Exfiltration resistance: stolen device without OS biometric/PIN cannot easily decrypt vaults.

---

## 3. Cryptographic Primitives (recommended)
Prefer modern, well-reviewed primitives and libraries. Exact choices should be validated by the crypto lead and audited.

- **Asymmetric key agreement:** X25519 (Curve25519) for ECDH.
- **Signatures:** Ed25519 for signing device registration and messages.
- **Symmetric AEAD:** XChaCha20-Poly1305 or AES-256-GCM (XChaCha20 preferred on platforms lacking AES hardware acceleration).  
- **KDF / HKDF:** HKDF-SHA256 for key derivation.  
- **Password-based KDF:** Argon2id for deriving keys from user passphrases.  
- **Authenticated public key operations / wrappers:** libsodium or appropriate WebCrypto combos.

Rationale: strong, performant, and portable across native and JS environments.

---

## 4. High-level Design

### Root concepts
- **Root Key (K_root):** The user’s master encryption key material. Never sent to server in plaintext. Derived or wrapped per-device.
- **Device Key Pair (DK_priv, DK_pub):** Each device has an asymmetric key pair; private key stored in OS keystore when possible. DK_pub is registered with the server.
- **Wrapped Root Key:** K_root is encrypted (wrapped) separately for each registered device using that device’s public key (hybrid ECDH + AEAD), allowing each device to recover K_root.
- **Collection Keys:** Derived per data-type (bookmarks, passwords, history) using HKDF from K_root to limit blast radius.
- **Record Encryption:** Each record encrypted with AEAD using a per-record key derived from the collection key + record id + nonce.

### Data flow (summary)
1. Device generates DK keypair and (optionally) K_root (if first device).
2. K_root wrapped for each device and uploaded to server; server stores encrypted blobs associated with device ids.
3. When a device needs to sync, it authenticates to server (WebAuthn / TLS client auth / session token). Device downloads encrypted blobs, unwraps K_root with local DK_priv, derives collection keys, decrypts records locally.
4. New record writes: device encrypts record with per-record AEAD and uploads ciphertext to server. Server stores and replicates blobs but cannot decrypt.

---

## 5. Device Onboarding Flows
Multiple onboarding flows to balance UX and security.

### 5.1 Passphrase-Based (User-chosen master passphrase)
**Use case:** user without platform authenticator or wanting a simple recovery.

Flow:
1. User chooses a strong passphrase at account creation (U_p).  
2. Client derives a passphrase key: K_pass = Argon2id(U_p, salt, params).  
3. Client generates K_root (random) and encrypts it under K_pass via AEAD: Enc_pass = AEAD(K_pass, K_root, meta).  
4. Client also generates DK keypair and wraps K_root for DK_pub (see 4).  
5. Upload Enc_pass and wrapped-root to server.  

Recovery: On new device, user provides passphrase; client derives K_pass and decrypts K_root; then device registers DK_pub and wraps K_root for itself.

**Security note:** Passphrase-based recovery is only as strong as user-chosen passphrase. Use strong passphrase guidance and recommend passphrase managers or recovery keys.

---

### 5.2 Passkey / Platform Authenticator-Based (recommended)
**Use case:** devices with Windows Hello / Touch ID; offers strong UX and protection.

Flow:
1. Device generates DK keypair inside platform authenticator or stores DK_priv in OS keystore protected by biometric/PIN.  
2. New device authenticates to server using WebAuthn challenge-response (proves possession of DK_priv).  
3. For first device, create K_root locally. For additional devices, request wrapped K_root blobs from server and perform ECDH unwrap to get K_root.  
4. Server enforces user confirmation of new-device enrollment (e.g., existing device push notification) optionally.

**Security note:** Device private keys are hardware-backed when available; attackers without biometric/PIN cannot use DK_priv.

---

### 5.3 Direct Device Pairing (QR / Transfer)
**Use case:** quick-add device using existing device as transfer agent (offline or secure in-band).

Flow:
1. Existing device generates ephemeral ECDH keypair and derives shared secret with new device scanned via QR code.  
2. Existing device encrypts K_root with ephemeral secret and transfers ciphertext to new device.  
3. New device unwraps and registers DK_pub with server.

This flow is useful when passphrase is unavailable and platform authenticator is absent.

---

## 6. Sync Data Model & Storage Format

### 6.1 Collections & Keys
- **K_root** -> HKDF -> K_bookmarks, K_passwords, K_history, K_settings, K_tabs
- Each collection key used to derive per-record keys: K_record = HKDF(K_collection, record_id || context)

### 6.2 Record Envelope (JSON)
```json
{
  "record_id": "uuid",
  "collection": "passwords",
  "ciphertext": "base64...",
  "aad": "base64...", // includes record metadata (non-sensitive) signed or included in AAD
  "nonce": "base64...",
  "version": 1
}
```
- `ciphertext` is AEAD-encrypted bytes of the record payload.
- `aad` should include non-sensitive metadata such as record schema version and optionally hashed identifiers to support deduplication without revealing plaintext.

### 6.3 Server Storage
- Server stores per-user collections: map of record_id -> envelope.  
- Server stores per-device wrapped K_root blobs (WrappedRoot[device_id]) and device public keys.  
- Server stores minimal plaintext metadata: last-modified timestamps and content-hash (blinded) for conflict detection.

---

## 7. Server API Surface (REST / JSON over HTTPS)
All endpoints authenticated (TLS + session). Example endpoints:

- `POST /v1/register_device` — register device pubkey, device metadata; returns device_id.
- `GET /v1/wrapped_root/{user}` — returns list of wrapped-root blobs for devices (for device recovery flow).
- `POST /v1/wrapped_root/{user}` — upload wrapped root for a device.
- `GET /v1/collections/{collection}` — list record_ids + envelope metadata for a collection.
- `GET /v1/collections/{collection}/{record_id}` — fetch a record envelope.
- `PUT /v1/collections/{collection}/{record_id}` — upload/update encrypted record envelope.
- `DELETE /v1/collections/{collection}/{record_id}` — delete record.
- `POST /v1/revoke_device/{device_id}` — revoke device and the server marks wrapped root as revoked.
- `POST /v1/rotate_root` — initiate root rotation operation (see section 8).

Server must authenticate clients and rate-limit endpoints.

---

## 8. Key Management & Rotation

### 8.1 Root Rotation
- Periodically (or on-demand) generate new K_root' and rewrap it for all active device public keys.
- Rotation is performed client-side by a device that has K_root; it encrypts K_root' for each device and uploads wrapped blobs in a `rotate_root` transaction.
- Old wrapped blobs kept until all clients confirm rotation or for a grace period.

### 8.2 Per-Record Keys
- Per-record keys derived; no separate rotation necessary beyond collection/key rotation.

### 8.3 Device Key Rotation
- Devices may rotate DK keypair via `register_device` and `revoke_device` flows.  

### 8.4 Compromise Response
- If a device is compromised, user revokes device via UI; server flags wrapped root as revoked and future syncs exclude revoked device; optionally trigger root rotation.

---

## 9. Recovery & Account Recovery Models

Provide multiple recovery options balanced with security:

### 9.1 Recovery Passphrase (User-Managed)
- User provided passphrase-derived key encrypts K_root (Enc_pass). If user forgets passphrase and has no other device, recovery is impossible (zero-knowledge trade-off).

### 9.2 Recovery Token (Escrowed, Optional)
- Offer an optional encrypted recovery token (a random high-entropy blob) user must store offline. Token unwraps K_root.

### 9.3 Assisted Recovery (Social / Multi-Device Approval)
- Require approval from one or more existing devices (push confirmation) to authorize a new device and unwrap K_root. Useful for enterprise or multi-device-heavy users.

**Design note:** Escrowed recovery weakens zero-knowledge guarantees; make it opt-in with clear UX warnings.

---

## 10. Revocation & Remote Wipe

- Server maintains device registry with `revoked` flag.  
- On revocation, server refuses to provide wrapped-root entries for the revoked device.  
- User-initiated remote wipe: server sends tombstone conflict to collections to indicate deletion; clients honor server-sourced deletions with signed transactions.

---

## 11. Metadata Minimization & Privacy

- Store minimal metadata server-side: timestamps, content-hash (H = HMAC(K_root, ciphertext) or blinded hash).  
- Avoid storing plaintext record counts; if necessary, provide differential privacy for aggregated stats.
- Telemetry only on opt-in and aggregated.

---

## 12. Integration with Platform Authenticators (Windows Hello / Touch ID)

- Prefer storing DK_priv in platform keystore (TPM / Secure Enclave / Windows Hello container) where possible.
- Use WebAuthn attestation during device registration to prove hardware-backed key.
- Use platform-based unlocking to gate access to DK_priv for decrypting K_root; this prevents an attacker with disk access from decrypting without biometric/PIN.

Implementation notes:
- On Windows, register a CNG key protected by Passport/Windows Hello.
- On macOS, store DK_priv in Secure Enclave / Keychain with Touch ID requirement for use.
- On Linux, use GNOME Keyring / KWallet or prompt for passphrase and prefer hardware modules where available.

---

## 13. Backup Export / Import Format

Provide an encrypted, portable backup file:
- Format: `.vigo-backup` (JSON wrapper + binary chunk for ciphertext)
- Contains: encrypted K_root wrapped with user chosen passphrase-derived key and a backup UUID; signed by device key.
- Import requires passphrase or trusted-device handshake.

Security: backups must be AEAD protected with Argon2id-derived key and include creation timestamp and version.

---

## 14. Operational & Scalability Considerations

- Store per-user collections in a scalable KV store (Cassandra/RocksDB/Cloud KVS) with object blobs.  
- Use CDN or edge caching for read-heavy metadata (not ciphertext content unless allowed).  
- Enforce per-user rate-limits and throttles to mitigate enumeration/DoS.  
- Design server stateless when possible; keep device registry in durable store.

---

## 15. Monitoring, Testing & Compliance

### 15.1 Monitoring
- Monitor for anomalous `register_device` spikes, repeated failed unwrapping attempts, and high-rate record updates.  
- Alert on suspicious IP patterns and mass revocations.

### 15.2 Testing
- Cryptographic review and third-party audit prior to launch.  
- Fuzzing of serialization and key unwrap paths.  
- Penetration testing of server endpoints and rate-limit enforcement.  
- Interoperability tests across OS-specific keystore integrations.

### 15.3 Compliance
- Generate SBOM for sync server code.  
- Data residency options for enterprise / regional compliance (EU data zoning).  

---

## 16. Appendix

### 16.1 Device Registration Sequence (abridged)
1. Client generates DK keypair (if not present) in hardware keystore.  
2. Client signs registration request with DK_priv or WebAuthn assertion.  
3. Server returns device_id and stores DK_pub.  
4. If first device, client generates K_root and uploads wrapped K_root for device.  

### 16.2 Add Device (wrapped-root flow)
1. New device requests wrapped-root blobs.  
2. Server returns wrapped-root list encrypted for existing devices and/or passphrase-derived blob.  
3. New device performs unwrap using passphrase or pairing flow.

### 16.3 JSON Example — Wrapped Root Blob
```json
{
  "device_id":"dev-uuid",
  "wrapped_root":"base64(ciphertext)",
  "wrap_alg":"X25519-HKDF-XChaCha20Poly1305",
  "created_at":"...",
  "version":1
}
```

### 16.4 Record Envelope Example
(see Section 6.2)

---

## Final notes & tradeoffs
- This design maximizes confidentiality (zero-knowledge) while offering pragmatic recovery options.  
- Recovery options (passphrase, escrow token) reduce absolute security but are necessary for usable product; these must be opt-in and accompanied by clear UI warnings.  
- Hardware-backed keys (platform authenticators) dramatically improve resistance to stolen-device attacks; support for these flows should be a priority.

**End of Sync & Cryptography Architecture Specification — Vigo v1.0**

