# Changelog

## 1.4.2 public source

- General media categories: comics, images, games, videos and documents.
- Korean and English folder aliases, neutral voice commands and sample tags.
- Genre/topic tag presentation and user-defined tag namespace search on desktop
  and mobile; Windows path search remains supported.
- Explicit library path entry, empty first launch and separate app identifiers.
- Optional remote host (`HEART_ENABLE_REMOTE=1`) and generic discovery name.
- OS-backed secure random generation that reports failure instead of falling
  back to predictable pairing/session credentials.
- Default image application fallback when Honeyview is not installed.
- Removed bundled external-site shortcuts and unused open-URL capability.
- New geometric icon source and generated desktop assets.
- Restored Android debug APK build script and retained Windows CI checks.
- Updated JavaScript dependency lockfile and patched the transitive cookie
  dependency with a 0.7.x override. Re-evaluate the override when upgrading Kit.
- Public-source checker, synthetic regression tests, contributor and security
  documentation, expanded ignore rules, and consistent version metadata.

The app identifiers changed to `org.heartlibrary.heart` and
`org.heartlibrary.heartremote`. Existing app data and pairing settings are not
automatically migrated. Keep previous data separately if it is needed.
