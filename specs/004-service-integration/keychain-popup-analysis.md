# Keychain Popup Root Cause Analysis

**Status:** ROOT CAUSE IDENTIFIED
**Date:** 2026-01-22

## Problem

The macOS keychain access dialog appears repeatedly when using the `keyring` crate for credential storage:

> tusk wants to access key "dev.tusk.Tusk" in your keychain.

## Root Cause

**Unsigned development builds lack stable code signatures.**

From Zed's docs (`/Users/brandon/src/zed/docs/src/development.md`):

> "On macOS this is caused by the development build not having a stable identity. Even if you choose the 'Always Allow' option, the OS will still prompt you for your password again the next time something changes in the binary."

How macOS keychain ACLs work:

1. When you click "Always Allow", macOS records the app's **code signature** in the ACL
2. On subsequent access, macOS verifies the requesting app's signature matches the stored ACL
3. Development builds have **no stable signature** - each rebuild changes the binary
4. The ACL check fails because the new binary doesn't match the recorded signature
5. macOS prompts again

This affects both `kSecClassGenericPassword` (keyring crate) and `kSecClassInternetPassword` (Zed's FFI approach) equally. The choice of Security framework API is irrelevant to the popup behavior.

## Failed Approach: Lazy Initialization

1. Removed the startup `check_availability()` call
2. Used in-memory `HashMap` first
3. Added `AtomicBool` to track failures

**Result:** Failed. The root cause is code signing, not initialization timing.

## Zed's Solution

### Development Builds

File-based storage in `~/.config/zed/development_credentials`:

- `DevelopmentCredentialsProvider` stores credentials as JSON in a file
- No keychain access = no popups
- Selected when `ReleaseChannel::Dev` and `ZED_DEVELOPMENT_AUTH` is not set

Implementation: `/Users/brandon/src/zed/crates/credentials_provider/src/credentials_provider.rs`
Commit: `21bb7242ea` - "Add CredentialsProvider to silence keychain prompts in development"

### Release Builds

Real keychain via GPUI's Security framework FFI:

- Uses `kSecClassInternetPassword` with `kSecAttrServer` as the key
- Code-signed release builds have stable identity
- "Always Allow" persists because signature matches

Quote from Zed docs:
> "For all non-development release channels the system keychain is always used."

## Solution for Tusk

### Development

Implement file-based credential storage (like Zed's `DevelopmentCredentialsProvider`):

- Store credentials in a JSON file at `~/.config/tusk/dev_credentials.json`
- Select this provider when `cfg!(debug_assertions)` is true
- Override with `TUSK_USE_KEYCHAIN=1` environment variable for testing

### Production

**Proper code signing eliminates the popup issue.**

Requirements:
1. Sign the release build with a Developer ID certificate
2. Notarize the app
3. Optionally add `com.apple.security.keychain-access-groups` entitlement (not required, Zed doesn't use it)

Once signed, the keychain "Always Allow" selection persists correctly.

## Key Files

| File | Purpose |
|------|---------|
| `/Users/brandon/src/zed/crates/credentials_provider/src/credentials_provider.rs` | Zed's dual-provider implementation |
| `/Users/brandon/src/zed/crates/gpui/src/platform/mac/platform.rs:1029-1133` | Zed's Security framework FFI |
| `/Users/brandon/src/zed/docs/src/development.md:9-34` | Zed's documentation of this issue |
| `/Users/brandon/src/tusk/crates/tusk_core/src/services/credentials.rs` | Tusk's current keyring-based implementation |

## Implementation Tasks

See tasks T097-T103 in `/specs/004-service-integration/tasks.md` for the implementation plan.

## Conclusion

The `keyring` crate is not the problem. Direct Security framework FFI is not required. The issue is purely about code signing:

- **Development:** Use file-based storage to avoid keychain entirely
- **Production:** Code-sign the app so keychain ACLs persist
