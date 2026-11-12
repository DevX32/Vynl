package com.vynl.vynl_mobile.data

enum class LyricsStatus {
    Loading,
    Ready,
    Missing,
    Failed,
}

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
    val endMs: Long? = null,
)

data class Lyrics(
    val kind: String,
    val text: String,
    val lines: List<LyricLine>,
) {
    val isSynced: Boolean
        get() = kind == "lrc" || kind == "ttml"

    /** True when at least one line is timed. */
    val synced: Boolean
        get() = isSynced && lines.any { it.timeMs >= 0 }

    /** The line that should be showing at this moment, or -1 when idle. */
    fun activeIndex(positionMs: Long): Int {
        if (lines.isEmpty()) return -1
        var index = -1
        for ((position, line) in lines.withIndex()) {
            if (line.timeMs < 0) continue
            if (line.timeMs <= positionMs) index = position else break
        }
        if (index == lines.size - 1 && index >= 0 && positionMs - lines[index].timeMs > LAST_LINE_GRACE_MS) {
            return -1
        }
        return index
    }

    val hasDisplayable: Boolean
        get() = lines.isNotEmpty() || stripStamps(text).isNotBlank()

    companion object {
        private const val LAST_LINE_GRACE_MS = 10_000L

        fun parse(kind: String, text: String): Lyrics {
            if (kind == "ttml") {
                return Lyrics(kind, text, withEstimatedWords(parseTtml(text)))
            }
            return Lyrics(kind, text, withEstimatedWords(parseLrc(text)))
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
                    for (time in parseStamp(stamp)) {
                        val body = trimmed.substring(close + 1)
                        lines += LyricLine(time, stripTags(body), wordsOf(body))
                    }
                }
            }
            return lines.sortedBy { it.timeMs }
        }

        private val P_REGEX = Regex("<p\\b([^>]*)>(.*?)</p>", RegexOption.DOT_MATCHES_ALL)
        private val SPAN_REGEX = Regex("<span\\b([^>]*)>(.*?)</span>", RegexOption.DOT_MATCHES_ALL)
        private val ATTR_REGEX = { name: String -> Regex("\\b$name\\s*=\\s*\"([^\"]*)\"") }

        private fun ttmlAttr(attrs: String, name: String): String? =
            ATTR_REGEX(name).find(attrs)?.groupValues?.get(1)

        /** TTML times: plain seconds ("9.589"), mm:ss.mmm ("02:42.857") or hh:mm:ss.mmm ("00:00:16.620"). */
        private fun ttmlTime(value: String?): Long? {
            if (value == null) return null
            val base = value.trim()
            val trimmed = if (base.endsWith("s")) base.substring(0, base.length - 1) else base
            if (trimmed.isEmpty()) return null
            if (!trimmed.contains(':')) {
                return trimmed.toDoubleOrNull()?.let { (it * 1000).toLong() }
            }
            val parts = trimmed.split(':')
            val seconds = parts.last().trim().toDoubleOrNull() ?: return null
            val minutes = parts[parts.size - 2].trim().toDoubleOrNull() ?: return null
            val hours = if (parts.size >= 3) parts[parts.size - 3].trim().toDoubleOrNull() ?: 0.0 else 0.0
            return ((hours * 3600.0 + minutes * 60.0 + seconds) * 1000).toLong()
        }

        private fun parseTtml(text: String): List<LyricLine> {
            val lines = mutableListOf<LyricLine>()
            for (match in P_REGEX.findAll(text)) {
                val attrs = match.groupValues[1]
                val time = ttmlTime(ttmlAttr(attrs, "begin")) ?: continue
                val body = match.groupValues[2]
                val words = SPAN_REGEX.findAll(body).mapNotNull { span ->
                    val spanAttrs = span.groupValues[1]
                    val begin = ttmlTime(ttmlAttr(spanAttrs, "begin")) ?: return@mapNotNull null
                    val end = ttmlTime(ttmlAttr(spanAttrs, "end"))
                    val text2 = stripTags(span.groupValues[2]).trim()
                    if (text2.isEmpty()) null else LyricWord(begin, text2, end)
                }.toList()
                val clean = stripTags(body)
                if (clean.isBlank() && words.isEmpty()) continue
                lines += LyricLine(time, clean, words)
            }
            return lines.sortedBy { it.timeMs }
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

        private fun withEstimatedWords(lines: List<LyricLine>): List<LyricLine> {
            if (lines.isEmpty()) return lines
            return lines.mapIndexed { index, line ->
                if (line.words.isEmpty() && line.timeMs >= 0L && !isInstrumental(line.text)) {
                    val end = nextTimedLine(lines, index, line.timeMs)
                    line.copy(words = distributeWords(line.text, line.timeMs, end))
                } else {
                    line
                }
            }
        }

        private fun nextTimedLine(lines: List<LyricLine>, index: Int, after: Long): Long {
            for (i in index + 1 until lines.size) {
                if (lines[i].timeMs > after) return lines[i].timeMs
            }
            return after + 4_000L
        }

        private fun distributeWords(text: String, start: Long, end: Long): List<LyricWord> {
            val parts = text.trim().split(Regex("\\s+")).filter { it.isNotEmpty() }
            if (parts.isEmpty()) return emptyList()
            val span = (end - start).coerceAtLeast(1L)
            val slice = span.toDouble() / parts.size
            return parts.mapIndexed { index, part ->
                LyricWord((start + index * slice).toLong(), part)
            }
        }
    }
}

fun wordEndAt(words: List<LyricWord>, index: Int): Long {
    val word = words[index]
    word.endMs?.takeIf { it > word.timeMs }?.let { return it }
    if (index + 1 < words.size) {
        val next = words[index + 1]
        if (next.timeMs > word.timeMs) return next.timeMs
    }
    return word.timeMs + 450L
}

fun wordFill(words: List<LyricWord>, index: Int, positionMs: Long): Float {
    val word = words[index]
    if (positionMs <= word.timeMs) return 0f
    val end = wordEndAt(words, index)
    val span = end - word.timeMs
    if (span <= 0L) return 1f
    return ((positionMs - word.timeMs).toFloat() / span).coerceIn(0f, 1f)
}

private val SYMBOLS_ONLY = Regex("^[\\s\\p{P}\\p{S}]+$")
private val WRAP_OPEN = Regex("^[\\[({<【（\\s]+")
private val WRAP_CLOSE = Regex("[\\s:;,.·…~～\\-–—_)\\]}>）】]+$")
private val STAMP_RE = Regex("\\[[ \\t]*(\\d{1,3}):(\\d{1,2})(?::(\\d{1,4}))?(?:[.,](\\d{1,4}))?[ \\t]*]")
private val WORD_TAG_RE = Regex("<[ \\t]*(\\d{1,3}):(\\d{1,2})(?::(\\d{1,4}))?(?:[.,](\\d{1,4}))?[ \\t]*>")
private val WHITESPACE_RE = Regex("\\s+")
private val INSTRUMENTAL_WORD = Regex(
    "^(?:instrumental|inst\\.?|interlude|intermission|outro|intro|solo|break|bridge|" +
        "build[\\s._-]?up|fade[\\s._-]?(?:out|in)|prelude|postlude|reprise|" +
        "silence|soundcheck|music)$",
    RegexOption.IGNORE_CASE,
)
private val INSTRUMENTAL_PHRASE = Regex(
    "^(?:(?:instrumental|inst\\.?|music)[\\s._-]+(?:break|solo|section|pass|part|interlude|intro|outro)|" +
        "(?:guitar|piano|string|sax|synth|drum|flute|violin|cello|trumpet|organ)[\\s._-]*solo|" +
        "no\\s+lyrics?|end\\s+of\\s+song)$",
    RegexOption.IGNORE_CASE,
)

/** A line that carries no words, only music. */
fun isInstrumental(text: String): Boolean {
    val value = text.trim()
    if (value.isEmpty()) return true
    if (SYMBOLS_ONLY.matches(value)) return true
    val unwrapped = WRAP_OPEN.replace(value, "").let { WRAP_CLOSE.replace(it, "") }.trim()
    if (unwrapped.isEmpty()) return true
    if (SYMBOLS_ONLY.matches(unwrapped)) return true
    return INSTRUMENTAL_WORD.matches(unwrapped) || INSTRUMENTAL_PHRASE.matches(unwrapped)
}

/** The text without any time tags, as paragraphs separated by empty lines. */
fun stripStamps(text: String): String =
    text.lines()
        .joinToString("\n") { line ->
            line.replace(STAMP_RE, "").replace(WORD_TAG_RE, "")
        }
        .replace(Regex("\n{3,}"), "\n\n")
        .trim()

/** Plain lyrics grouped into stanzas, blank lines as the breaks. */
fun stanzasOf(text: String): List<List<String>> {
    val stanzas = mutableListOf<List<String>>()
    var current = mutableListOf<String>()
    for (raw in text.lines()) {
        val line = raw.replace(STAMP_RE, "").replace(WORD_TAG_RE, "").replace(WHITESPACE_RE, " ").trim()
        if (line.isEmpty()) {
            if (current.isNotEmpty()) {
                stanzas += current
                current = mutableListOf()
            }
            continue
        }
        current += line
    }
    if (current.isNotEmpty()) stanzas += current
    return stanzas
}
