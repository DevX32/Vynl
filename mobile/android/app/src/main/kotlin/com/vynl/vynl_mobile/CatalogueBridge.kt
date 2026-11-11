package com.vynl.vynl_mobile

/**
 * The shared catalogue client, compiled from the same Rust code the desktop
 * uses. It answers searches, albums, artists and playlists on the phone, so
 * the phone can browse without the desktop running.
 *
 * Every call returns a JSON string: the payload on success, or
 * `{"error": "..."}` when the request could not be answered.
 */
class CatalogueBridge {
    companion object {
        @Volatile
        private var loaded = false

        @Volatile
        private var available = false

        init {
            load()
        }

        private fun load() {
            if (loaded) return
            loaded = true
            try {
                System.loadLibrary("vynl_catalogue")
                available = true
            } catch (error: UnsatisfiedLinkError) {
                available = false
            }
        }

        /** Whether the native library is present on this build. */
        fun isAvailable(): Boolean {
            load()
            return available
        }

        @JvmStatic
        external fun nativeSearch(query: String): String

        @JvmStatic
        external fun nativeAlbum(id: String): String

        @JvmStatic
        external fun nativeArtist(id: String): String

        @JvmStatic
        external fun nativePlaylist(id: String): String

        @JvmStatic
        external fun nativeStream(id: String): String
    }
}
