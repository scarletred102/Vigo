# libsodium for Vigo Browser

Version: TBD (latest stable, currently 1.0.20)
Source: https://github.com/jedisct1/libsodium
License: ISC

## Vendoring Instructions

1. Download the latest stable release from the libsodium GitHub releases
2. Extract to this directory (`third_party/libsodium/`)
3. Update the `BUILD.gn` to compile all `.c` source files
4. Verify: `include/sodium.h` should be the public header

## Used For

- X25519 key agreement (`crypto_box_curve25519xsalsa20poly1305`)
- Ed25519 signatures (`crypto_sign_ed25519`)
- XChaCha20-Poly1305 AEAD (`crypto_aead_xchacha20poly1305_ietf`)
- HKDF-SHA256 key derivation (`crypto_kdf`)
- Argon2id password hashing (`crypto_pwhash`)
- Secure memory (`sodium_mlock`, `sodium_memzero`)
