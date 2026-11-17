package com.vynl.vynl_mobile.data

fun highResCover(url: String?): String? {
    val trimmed = url?.trim() ?: return null
    if (trimmed.isEmpty()) return null

    val start = trimmed.indexOf("=w")
    if (start < 0) return trimmed

    val wStart = start + 2
    var wEnd = wStart
    while (wEnd < trimmed.length && trimmed[wEnd] in '0'..'9') wEnd++
    if (wEnd == wStart) return trimmed
    val width = trimmed.substring(wStart, wEnd).toLongOrNull() ?: return trimmed
    if (!trimmed.startsWith("-h", wEnd)) return trimmed

    val hStart = wEnd + 2
    var hEnd = hStart
    while (hEnd < trimmed.length && trimmed[hEnd] in '0'..'9') hEnd++
    if (hEnd == hStart) return trimmed
    val height = trimmed.substring(hStart, hEnd).toLongOrNull() ?: return trimmed

    val tail = trimmed.substring(hEnd)
    val params = tail.split('-').filter { it.isNotEmpty() }.distinct()
    val dashCount = tail.count { it == '-' }

    val newWidth = maxOf(width, 512)
    val newHeight = maxOf(height, 512)
    if (newWidth == width && newHeight == height && params.size == dashCount) {
        return trimmed
    }

    val out = StringBuilder(trimmed.substring(0, start))
    out.append("=w").append(newWidth).append("-h").append(newHeight)
    for (param in params) {
        out.append('-').append(param)
    }
    return out.toString()
}
