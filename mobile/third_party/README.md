# Vendored third-party packages

These packages are checked into the repository and wired in via `path`
dependencies in `mobile/pubspec.yaml` rather than being resolved from pub.dev.

| Directory | Package | Version | Licence | Upstream |
| --- | --- | --- | --- | --- |
| `audio_service/` | `audio_service` | 0.18.19 | MIT | https://pub.dev/packages/audio_service |
| `just_audio_background/` | `just_audio_background` | 0.0.1-beta.17 | MIT | https://pub.dev/packages/just_audio_background |

Both are Copyright (c) Ryan Heise and contributors, MIT licensed.

## Why they are vendored

`audio_service` 0.18.19 declares `compileSdk = 35` in its
`android/build.gradle.kts`. AGP 9.1.0 cannot resolve that value:

```
Cannot query the value of this provider because it has no value available
```

There is no way to override it from a consuming project. Attempting to set
`compileSdk` from the app module fails too, with:

```
It is too late to set compileSdk
```

Vendoring is the only way to build against this version. `pub` does not
support patching a transitive dependency, so the source is copied in and
referenced by path.

## Local modifications

Both packages are otherwise unmodified against their published versions.
The only changes are:

- **`audio_service`** — one line in `android/build.gradle.kts`, `compileSdk`
  raised from 35 to 36.
- **`just_audio_background`** — `MediaControl.stop` removed from the
  notification's custom actions, so no Stop button appears on the
  notification or the lock screen. Vynl has no stop action of its own to
  offer, and an inert button is worse than none.

Each vendored directory holds only what the Android build consumes. The
`example/`, `test/`, `darwin/`, `ios/`, `macos/`, `web/`, `windows/` and
`linux/` trees, along with `CHANGELOG.md` and `README.md`, were left behind —
Vynl is Android-only.

To pick up a newer upstream release, copy the package over this directory,
re-apply the two changes above, and confirm the `compileSdk` restriction
still applies before dropping the vendored copy.

## Licence

Each package keeps its upstream MIT `LICENSE` alongside its source. Both are
Copyright (c) Ryan Heise and the project contributors, so redistribution
inside this repository is permitted provided the licence text travels with
the code.
