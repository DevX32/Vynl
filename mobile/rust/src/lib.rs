pub mod client;
pub mod lyrics;
pub mod types;

use std::sync::OnceLock;

use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::jstring;

use client::CatalogueClient;

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
static CLIENT: OnceLock<CatalogueClient> = OnceLock::new();
static RUNTIME_FAILED: OnceLock<()> = OnceLock::new();
static CLIENT_FAILED: OnceLock<()> = OnceLock::new();

fn client() -> Option<&'static CatalogueClient> {
    if let Some(client) = CLIENT.get() {
        return Some(client);
    }
    if CLIENT_FAILED.get().is_some() {
        return None;
    }
    match std::panic::catch_unwind(CatalogueClient::new) {
        Ok(built) => Some(CLIENT.get_or_init(|| built)),
        Err(_) => {
            let _ = CLIENT_FAILED.set(());
            None
        }
    }
}

fn runtime() -> Option<&'static tokio::runtime::Runtime> {
    if let Some(runtime) = RUNTIME.get() {
        return Some(runtime);
    }
    if RUNTIME_FAILED.get().is_some() {
        return None;
    }
    match std::panic::catch_unwind(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .thread_name("vynl-catalogue")
            .build()
            .expect("could not start the catalogue runtime")
    }) {
        Ok(built) => Some(RUNTIME.get_or_init(|| built)),
        Err(_) => {
            let _ = RUNTIME_FAILED.set(());
            None
        }
    }
}

fn error_json(message: &str) -> String {
    serde_json::json!({ "error": message }).to_string()
}

fn to_jstring(env: &mut JNIEnv, text: &str) -> jstring {
    match env.new_string(text) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn read_string(env: &mut JNIEnv, value: &JString) -> String {
    env.get_string(value)
        .map(|text| text.into())
        .unwrap_or_else(|_| String::new())
}

fn finish(env: &mut JNIEnv, result: Result<serde_json::Value, String>) -> jstring {
    let text = match result {
        Ok(value) => value.to_string(),
        Err(message) => error_json(&message),
    };
    to_jstring(env, &text)
}

enum Ask {
    Search,
    Album,
    Artist,
    Playlist,
    Stream,
    Taste,
    Lyrics,
}

fn ask(env: &mut JNIEnv, query: &str, kind: Ask) -> jstring {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(client) = client() else {
            return Err("the catalogue client failed to start".to_string());
        };
        let Some(runtime) = runtime() else {
            return Err("the catalogue runtime failed to start".to_string());
        };
        let query = query.to_string();
        let request = async move {
            match kind {
                Ask::Search => client
                    .search(&query)
                    .await
                    .map(|value| serde_json::to_value(value).unwrap_or_default()),
                Ask::Album => client
                    .album(&query)
                    .await
                    .map(|value| serde_json::to_value(value).unwrap_or_default()),
                Ask::Artist => client
                    .artist(&query)
                    .await
                    .map(|value| serde_json::to_value(value).unwrap_or_default()),
                Ask::Playlist => client
                    .playlist(&query)
                    .await
                    .map(|value| serde_json::to_value(value).unwrap_or_default()),
                Ask::Stream => client.stream(&query).await.map(|stream| {
                    serde_json::json!({
                        "url": stream.url,
                        "mime": stream.mime,
                    })
                }),
                Ask::Taste => {
                    let seeds: Vec<String> = serde_json::from_str(&query).unwrap_or_default();
                    client
                        .taste(&seeds)
                        .await
                        .map(|tracks| serde_json::json!({ "tracks": tracks }))
                }
                Ask::Lyrics => {
                    let lookup: lyrics::LyricsLookup =
                        serde_json::from_str(&query).unwrap_or(lyrics::LyricsLookup {
                            title: String::new(),
                            artist: String::new(),
                            album: String::new(),
                            duration: 0.0,
                        });
                    Ok(match lyrics::fetch(&lookup).await {
                        Some(result) => serde_json::json!({
                            "kind": result.kind,
                            "text": result.text,
                        }),
                        None => serde_json::json!({ "kind": "", "text": "" }),
                    })
                }
            }
        };
        match runtime.block_on(async {
            tokio::time::timeout(std::time::Duration::from_secs(30), request).await
        }) {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(error)) => Err(error),
            Err(_) => Err("the catalogue request timed out".to_string()),
        }
    }))
    .unwrap_or_else(|_| Err("the catalogue request failed unexpectedly".to_string()));
    finish(env, result)
}

macro_rules! bridge {
    ($short:ident, $long:ident, $kind:expr) => {
        #[unsafe(no_mangle)]
        pub extern "system" fn $short(mut env: JNIEnv, _class: JClass, query: JString) -> jstring {
            let query = read_string(&mut env, &query);
            ask(&mut env, &query, $kind)
        }

        #[unsafe(no_mangle)]
        pub extern "system" fn $long(mut env: JNIEnv, _class: JClass, query: JString) -> jstring {
            let query = read_string(&mut env, &query);
            ask(&mut env, &query, $kind)
        }
    };
}

bridge!(
    Java_com_vynl_vynl_1mobile_CatalogueBridge_nativeSearch,
    Java_com_vynl_vynl_1mobile_CatalogueBridge_nativeSearch__Ljava_lang_String_2,
    Ask::Search
);

bridge!(
    Java_com_vynl_vynl_1mobile_CatalogueBridge_nativeAlbum,
    Java_com_vynl_vynl_1mobile_CatalogueBridge_nativeAlbum__Ljava_lang_String_2,
    Ask::Album
);

bridge!(
    Java_com_vynl_vynl_1mobile_CatalogueBridge_nativeArtist,
    Java_com_vynl_vynl_1mobile_CatalogueBridge_nativeArtist__Ljava_lang_String_2,
    Ask::Artist
);

bridge!(
    Java_com_vynl_vynl_1mobile_CatalogueBridge_nativePlaylist,
    Java_com_vynl_vynl_1mobile_CatalogueBridge_nativePlaylist__Ljava_lang_String_2,
    Ask::Playlist
);

bridge!(
    Java_com_vynl_vynl_1mobile_CatalogueBridge_nativeStream,
    Java_com_vynl_vynl_1mobile_CatalogueBridge_nativeStream__Ljava_lang_String_2,
    Ask::Stream
);

bridge!(
    Java_com_vynl_vynl_1mobile_CatalogueBridge_nativeTaste,
    Java_com_vynl_vynl_1mobile_CatalogueBridge_nativeTaste__Ljava_lang_String_2,
    Ask::Taste
);

bridge!(
    Java_com_vynl_vynl_1mobile_CatalogueBridge_nativeLyrics,
    Java_com_vynl_vynl_1mobile_CatalogueBridge_nativeLyrics__Ljava_lang_String_2,
    Ask::Lyrics
);
