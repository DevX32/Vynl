# Vynl

The phone app, written in Kotlin with Jetpack Compose, with its catalogue in
Rust inside the app. It searches, browses and streams on its own: the desktop
does not have to be running.

## What it is made of

```
app/src/main/kotlin/com/vynl/vynl_mobile/
  CatalogueBridge.kt      the rust catalogue, called through jni
  MainActivity.kt         the entry point
  data/                   the models, the catalogue client and the phone's store
  playback/               the media player and the service that keeps it alive
  remote/                 the link to the desktop
  ui/theme/               the colours, the fonts and the shapes
  ui/components/          the shared widgets
  ui/screens/             the screens
  ui/shell/               the tabs and the mini player
  update/                 reading the release manifest and installing an apk

rust/                     the catalogue, in rust, inside the app
  src/client.rs           search, album, artist, playlist, stream and the mix
  src/types.rs            the data the phone reads
  src/lyrics.rs           the lyrics, from the same providers the desktop uses
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
cd mobile
.\gradlew.bat :app:assembleDebug
```

The release build reads the signing key from `mobile/key.properties`. Without
it the release falls back to the debug key.

The debug build keeps the real application id with a `.debug` suffix, so it
lives beside a release install on the same phone.

## What it does

- the shell with the four tabs, the mini player and the splash reveal
- the theme, with the same colours, fonts, shapes and motion as the design
- the catalogue: search, albums, artists, playlists and the mix
- playback through Media3, with the audio resolved by the rust catalogue
- the row menu: go to album, go to artist, add to playlist and download
- downloaded tracks play from the phone with no network
- the player page: the cover backdrop, the seek bar, the transport
- the lyrics: synced lines and the karaoke words inside them
- the phone's own store: playlists, the library and the downloaded files
- the settings, the accent picker, the updater and the system installer
- pairing with the desktop, its remote control and its library
