# Vynl for Android, in Compose

The phone app, written in Kotlin with Jetpack Compose on top of the same Rust
core the desktop uses. The flutter build in `mobile/` is the reference for the
look and the behaviour; this one is the native replacement.

## What it is made of

```
app/src/main/kotlin/com/vynl/vynl_mobile/
  CatalogueBridge.kt      the rust catalogue, called through jni
  MainActivity.kt         the entry point
  data/                   the models and the catalogue client
  playback/               the media player and the service that keeps it alive
  ui/theme/               the colours, the fonts and the shapes
  ui/components/          the shared widgets
  ui/screens/             the screens
  ui/shell/               the tabs and the mini player

rust/                     the catalogue, in rust, inside the app
  src/client.rs           search, album, artist, playlist, stream and the mix
  src/types.rs            the data the phone reads
  src/lib.rs              the bridge the kotlin side calls
```

The catalogue is rust and it belongs to the app: `rust/` holds the code, and
`cargo-ndk` builds it for android inside the gradle build, so the phone can
search, browse and stream on its own, without the desktop running.

The audio is played by Media3, in a `MediaSessionService`, so it keeps playing
when the app is closed and the lock screen and the notification work the way
android expects.

## Building it

The build needs the android sdk, the ndk and a jdk 17.

```powershell
$env:JAVA_HOME = "C:\Users\DevX\jdk\jdk-17.0.21+9"
$env:ANDROID_HOME = "$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_NDK_HOME = "$env:LOCALAPPDATA\Android\Sdk\ndk\28.2.13676358"
cd mobile-android
.\gradlew.bat :app:assembleDebug
```

The debug build keeps the real application id with a `.compose` suffix, so it
lives beside the flutter build while the two are compared.

## Where it stands

Done:

- the shell with the four tabs, the mini player and the splash reveal
- the theme, with the same colours, fonts, shapes and motion as the flutter app
- the catalogue: search, albums, artists, playlists and the mix
- playback through Media3, with the audio resolved by the rust catalogue

Still to do:

- the library and the playlists, which need the local store
- the downloads and the offline playback
- the lyrics
- the settings, the accent picker, the updater and the installer
- the pairing and the remote control
