class LyricWord {
  const LyricWord({required this.time, required this.text});

  final double time;
  final String text;
}

class LyricLine {
  const LyricLine({required this.time, required this.text, this.words});

  final double time;
  final String text;
  final List<LyricWord>? words;
}

enum LyricsKind { lrc, text }

class LyricsResult {
  const LyricsResult({
    required this.kind,
    required this.text,
    required this.source,
  });

  final LyricsKind kind;
  final String text;
  final LyricsSource source;

  bool get synced => kind == LyricsKind.lrc;
}

enum LyricsSource { embedded }

final _stampRe = RegExp(r'\[(\d{1,2}):(\d{1,2})(?:[.:](\d{1,3}))?\]');
final _wordTagRe = RegExp(r'<(\d{1,2}):(\d{2})(?:[.:](\d{1,3}))?>');
final _wordTagSplitRe = RegExp(r'<\d{1,2}:\d{2}(?:[.:]\d{1,3})?>');
final _offsetRe = RegExp(
  r'^\[offset:\s*([+-]?\d+)\s*\]',
  caseSensitive: false,
);
final _metadataRe = RegExp(
  r'^\[(ar|ti|al|au|by|offset|re|ve|length|created|tool|version|application):',
  caseSensitive: false,
);
final _instrumentalRe = RegExp(
  r'^[(\[]?(instrumental|interlude|outro|intro|solo|break|bridge|build.?up|fade[.\s]?out|fade[.\s]?in|prelude|postlude|reprise)[)\]]?$',
  caseSensitive: false,
);
final _symbolsOnlyRe = RegExp(r'^[\s♪♫♩♬🎵🎶…~～·.\-—–]+$');

double _seconds(RegExpMatch m) {
  final min = int.parse(m.group(1)!);
  final sec = int.parse(m.group(2)!);
  final frac = m.group(3);
  final ms = frac == null ? 0.0 : double.parse(frac.padRight(3, '0')) / 1000;
  return (min * 60 + sec + ms).toDouble();
}

double readOffsetSecs(String text) {
  for (final raw in text.split(RegExp(r'\r?\n'))) {
    final m = _offsetRe.firstMatch(raw.trim());
    if (m != null) {
      final ms = int.tryParse(m.group(1)!);
      if (ms != null) return ms / 1000;
    }
  }
  return 0;
}

List<LyricWord>? _parseEnhancedWords(String body, double offset) {
  final stamps = <double>[];
  for (final m in _wordTagRe.allMatches(body)) {
    stamps.add(_seconds(m) + offset);
  }
  if (stamps.isEmpty) return null;

  final segments = body.split(_wordTagSplitRe);
  final words = <LyricWord>[];
  for (var i = 0; i < stamps.length; i++) {
    final text = (i + 1 < segments.length ? segments[i + 1] : '').trim();
    if (text.isNotEmpty) {
      words.add(LyricWord(time: stamps[i], text: text));
    }
  }
  return words.isEmpty ? null : words;
}

List<LyricLine> parseLrcText(String text) {
  final out = <LyricLine>[];
  final offset = readOffsetSecs(text);
  for (final raw in text.split(RegExp(r'\r?\n'))) {
    if (_metadataRe.hasMatch(raw.trim())) continue;

    final stamps = <double>[];
    for (final m in _stampRe.allMatches(raw)) {
      stamps.add(_seconds(m) + offset);
    }

    final body = raw.replaceAll(_stampRe, '').trim();
    if (stamps.isEmpty) {
      if (body.isEmpty) continue;
      out.add(LyricLine(time: -1, text: body));
      continue;
    }

    final words = _parseEnhancedWords(body, offset);
    final clean = body.replaceAll(_wordTagRe, '').trim();
    for (final s in stamps) {
      out.add(LyricLine(time: s, text: clean, words: words));
    }
  }

  out.sort((a, b) => a.time.compareTo(b.time));
  return out;
}

List<LyricLine> toLyricLines(LyricsResult result) {
  if (result.kind == LyricsKind.lrc) return parseLrcText(result.text);
  return result.text
      .split(RegExp(r'\r?\n'))
      .map((line) => LyricLine(time: -1, text: line))
      .toList();
}

final _remoteCache = <String, LyricsResult>{};

LyricsResult? lyricsFromRemote(String trackId, String text) {
  if (trackId.isEmpty || text.trim().isEmpty) return null;
  final cached = _remoteCache[trackId];
  if (cached != null) return cached;
  final result = LyricsResult(
    kind: _stampRe.hasMatch(text) ? LyricsKind.lrc : LyricsKind.text,
    text: text,
    source: LyricsSource.embedded,
  );
  _remoteCache[trackId] = result;
  return result;
}

LyricsResult? lyricsFromTrack(String? embedded) {
  final trimmed = embedded?.trim() ?? '';
  if (trimmed.isEmpty) return null;
  final looksSynced = _stampRe.hasMatch(trimmed);
  return LyricsResult(
    kind: looksSynced ? LyricsKind.lrc : LyricsKind.text,
    text: trimmed,
    source: LyricsSource.embedded,
  );
}

int activeLineIndex(List<LyricLine> lines, Duration position) {
  final t = position.inMilliseconds / 1000;
  var idx = -1;
  for (var i = 0; i < lines.length; i++) {
    final line = lines[i];
    if (line.time < 0) continue;
    if (line.time <= t) {
      idx = i;
    } else {
      break;
    }
  }
  return idx;
}

int activeWordIndex(List<LyricWord> words, Duration position) {
  final t = position.inMilliseconds / 1000;
  var idx = -1;
  for (var i = 0; i < words.length; i++) {
    if (words[i].time <= t) {
      idx = i;
    } else {
      break;
    }
  }
  return idx;
}

bool isInstrumental(String text) {
  final t = text.trim();
  if (t.isEmpty) return true;
  if (_symbolsOnlyRe.hasMatch(t)) return true;
  return _instrumentalRe.hasMatch(t);
}

List<List<String>> stanzasOf(String text) {
  final stanzas = <List<String>>[];
  var current = <String>[];
  for (final line in text.split(RegExp(r'\r?\n'))) {
    if (line.trim().isEmpty) {
      if (current.isNotEmpty) {
        stanzas.add(current);
        current = <String>[];
      }
      continue;
    }
    current.add(line.trim());
  }
  if (current.isNotEmpty) stanzas.add(current);
  return stanzas;
}