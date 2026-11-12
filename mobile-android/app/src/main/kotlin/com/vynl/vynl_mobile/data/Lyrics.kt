package com.vynl.vynl_mobile.data

/**
 * The lyrics of a song, parsed into lines that carry their own time, and the
 * words inside a line that carry theirs.
 */
data class LyricLine(
    val timeMs: Long,
    val text: String,
    val words: List<LyricWord>,
)

data class LyricWord(
    val timeMs: Long,
    val text: String,
)

data class Lyrics(
    val kind: String,
    val text: String,
    val lines: List<LyricLine>,
) {
    val isSynced: Boolean
        get() = kind == "lrc" || kind == "ttml"

    /** The line that should be showing at this moment. */
    fun indexAt(positionMs: Long): Int {
        if (lines.isEmpty()) return -1
        var index = -1
        for ((position, line) in lines.withIndex()) {
            if (line.timeMs <= positionMs) index = position else break
        }
        return index
    }

    companion object {
        fun parse(kind: String, text: String): Lyrics {
            if (kind == "ttml") {
                return Lyrics(kind, text, parseTtml(text))
            }
            return Lyrics(kind, text, parseLrc(text))
        }

        private fun parseLrc(text: String): List<LyricLine> {
            val lines = mutableListOf<LyricLine>()
            for (raw in text.lines()) {
                val trimmed = raw.trim()
                if (trimmed.isEmpty()) continue
                if (trimmed.startsWith("[") && trimmed.contains(":")) {
                    val close = trimmed.indexOf(']')
                    if (close < 0) continue
                    val stamp = trimmed.substring(1, close)
                    // A line can carry more than one stamp.
                    for (time in parseStamp(stamp)) {
                        val body = trimmed.substring(close + 1)
                        lines += LyricLine(time, stripTags(body), wordsOf(body))
                    }
                }
            }
            return lines.sortedBy { it.timeMs }
        }

        private fun parseTtml(text: String): List<LyricLine> {
            val lines = mutableListOf<LyricLine>()
            val pattern = Regex("<p[^>]*begin=\"([^\"]+)\"[^>]*>(.*?)</p>", RegexOption.DOT_MATCHES_ALL)
            for (match in pattern.findAll(text)) {
                val time = parseIso(match.groupValues[1]) ?: continue
                val body = stripTags(match.groupValues[2])
                lines += LyricLine(time, body, emptyList())
            }
            return lines.sortedBy { it.timeMs }
        }

        private fun parseIso(value: String): Long? {
            val parts = value.split(':')
            if (parts.size < 2) return null
            val seconds = parts.last().trim().toDoubleOrNull() ?: return null
            val minutes = parts[parts.size - 2].trim().toDoubleOrNull() ?: return null
            return ((minutes * 60.0 + seconds) * 1000.0).toLong()
        }

        private fun parseStamp(stamp: String): List<Long> {
            val times = mutableListOf<Long>()
            for (part in stamp.split('[')) {
                val trimmed = part.trim().trimEnd(']').trim()
                val pieces = trimmed.split(':')
                if (pieces.size < 2) continue
                val seconds = pieces.last().replace(',', '.').toDoubleOrNull() ?: continue
                val minutes = pieces[pieces.size - 2].toDoubleOrNull() ?: continue
                times += ((minutes * 60.0 + seconds) * 1000.0).toLong()
            }
            return times
        }

        private fun stripTags(body: String): String =
            body.replace(Regex("<[^>]*>"), "").trim()

        /** The words of a line, with the time each one starts. */
        private fun wordsOf(body: String): List<LyricWord> {
            if (!body.contains('<')) return emptyList()
            val words = mutableListOf<LyricWord>()
            val pattern = Regex("<(\\d{1,3}):(\\d{1,2}(?:[.:]\\d{1,3})?)>([^<]*)")
            for (match in pattern.findAll(body)) {
                val minutes = match.groupValues[1].toDoubleOrNull() ?: continue
                val seconds = match.groupValues[2].replace(',', '.').toDoubleOrNull() ?: continue
                val text = match.groupValues[3].trim()
                if (text.isEmpty()) continue
                words += LyricWord(((minutes * 60.0 + seconds) * 1000.0).toLong(), text)
            }
            return words
        }
    }
}
