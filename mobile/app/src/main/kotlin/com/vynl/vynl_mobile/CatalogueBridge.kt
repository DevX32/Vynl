package com.vynl.vynl_mobile

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

        @JvmStatic
        external fun nativeTaste(seeds: String): String

        @JvmStatic
        external fun nativeLyrics(lookup: String): String
    }
}
