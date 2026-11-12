//! The catalogue the phone app talks to.
//!
//! The code lives inside the app: the phone builds its own rust and links it
//! in, so nothing outside this folder has to change for the phone to browse.
//! The desktop keeps its own copy of the same client.

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

fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .thread_name("vynl-catalogue")
            .build()
            .expect("could not start the catalogue runtime")
    })
}

fn client() -> &'static CatalogueClient {
    CLIENT.get_or_init(CatalogueClient::new)
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

/// The kind of answer a call wants back.
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
    let client = client();
    let query = query.to_string();
    let result = runtime().block_on(async move {
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
    });
    finish(env, result)
}

// The jni name of a class turns every character that is not a letter or a
// digit into its code, so the underscore in `vynl_mobile` becomes `_1`.
// Without that the phone cannot find the method at all.
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
