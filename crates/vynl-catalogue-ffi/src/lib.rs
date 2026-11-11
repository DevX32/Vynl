use std::sync::OnceLock;

use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::jstring;
use vynl_catalogue::CatalogueClient;

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

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vynl_vynl_mobile_CatalogueBridge_nativeSearch(
    mut env: JNIEnv,
    _class: JClass,
    query: JString,
) -> jstring {
    let query = read_string(&mut env, &query);
    let client = client();
    let result = runtime().block_on(async move { client.search(&query).await });
    finish(
        &mut env,
        result.map(|value| serde_json::to_value(value).unwrap_or_default()),
    )
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vynl_vynl_mobile_CatalogueBridge_nativeAlbum(
    mut env: JNIEnv,
    _class: JClass,
    id: JString,
) -> jstring {
    let id = read_string(&mut env, &id);
    let client = client();
    let result = runtime().block_on(async move { client.album(&id).await });
    finish(
        &mut env,
        result.map(|value| serde_json::to_value(value).unwrap_or_default()),
    )
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vynl_vynl_mobile_CatalogueBridge_nativeArtist(
    mut env: JNIEnv,
    _class: JClass,
    id: JString,
) -> jstring {
    let id = read_string(&mut env, &id);
    let client = client();
    let result = runtime().block_on(async move { client.artist(&id).await });
    finish(
        &mut env,
        result.map(|value| serde_json::to_value(value).unwrap_or_default()),
    )
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vynl_vynl_mobile_CatalogueBridge_nativePlaylist(
    mut env: JNIEnv,
    _class: JClass,
    id: JString,
) -> jstring {
    let id = read_string(&mut env, &id);
    let client = client();
    let result = runtime().block_on(async move { client.playlist(&id).await });
    finish(
        &mut env,
        result.map(|value| serde_json::to_value(value).unwrap_or_default()),
    )
}
