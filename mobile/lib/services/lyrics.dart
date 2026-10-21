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

enum LyricsKind { lrc, ttml, text }

class LyricsResult {
  const LyricsResult({
    required this.kind,
    required this.text,
    required this.source,
  });

  final LyricsKind kind;
  final String text;
  final LyricsSource source;

  bool get synced => kind != LyricsKind.text;
}

enum LyricsSource { embedded }

final _stampRe = RegExp(
  r'\[[ \t]*(\d{1,3}):(\d{1,2})(?::(\d{1,4}))?(?:[.,](\d{1,4}))?[ \t]*\]',
);
final _hasStampRe = RegExp(
  r'\[[ \t]*\d{1,3}:\d{1,2}(?::\d{1,4})?(?:[.,]\d{1,4})?[ \t]*\]',
);
final _wordTagRe = RegExp(
  r'<[ \t]*(\d{1,3}):(\d{1,2})(?::(\d{1,4}))?(?:[.,](\d{1,4}))?[ \t]*>',
);
final _wordTagSplitRe = RegExp(
  r'<[ \t]*\d{1,3}:\d{1,2}(?::\d{1,4})?(?:[.,]\d{1,4})?[ \t]*>',
);
final _offsetRe = RegExp(
  r'^\[offset:\s*([+-]?\d+)\s*\]',
  caseSensitive: false,
);
final _metadataRe = RegExp(
  r'^\[(ar|ti|al|au|by|offset|re|ve|length|created|tool|version|application):',
  caseSensitive: false,
);
final _instrumentalWordRe = RegExp(
  r'^(?:'
  r'instrumental|inst\.?|interlude|intermission|outro|intro|solo|break|bridge|'
  r'build[\s._-]?up|fade[\s._-]?(?:out|in)|prelude|postlude|reprise|'
  r'silence|soundcheck|music'
  r')$',
  caseSensitive: false,
);
final _instrumentalPhraseRe = RegExp(
  r'^(?:'
  r'(?:instrumental|inst\.?|music)[\s._-]+(?:break|solo|section|pass|part|interlude|intro|outro)|'
  r'(?:guitar|piano|string|sax|synth|drum|flute|violin|cello|trumpet|organ)[\s._-]*solo|'
  r'no\s+lyrics?|end\s+of\s+song'
  r')$',
  caseSensitive: false,
);
final _symbolsOnlyRe = RegExp(r'^[\s\p{P}\p{S}]+$', unicode: true);
final _wrapOpenRe = RegExp(r'^[[({<【（\s]*');
final _wrapCloseRe = RegExp(r'[\s:;,.·…~～\-–—_)\]}>）】]*$');
final _wsRe = RegExp(r'\s+');
const _lastLineGraceSecs = 10.0;
final _ttmlRootRe = RegExp(r'<(?:ttml|tt)(?=[\s>/])', caseSensitive: false);

double _seconds(RegExpMatch m) {
  final a = int.parse(m.group(1)!);
  final b = int.parse(m.group(2)!);
  final third = m.group(3);
  final frac = m.group(4);
  final f = frac == null ? 0.0 : double.parse('0.$frac');
  if (third == null) return (a * 60 + b + f).toDouble();
  if (third.length <= 2) {
    return (a * 3600 + b * 60 + int.parse(third) + f).toDouble();
  }
  return (a * 60 + b + double.parse('0.$third')).toDouble();
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

    final body = raw.replaceAll(_stampRe, '').replaceAll(_wsRe, ' ').trim();
    if (stamps.isEmpty) {
      final plain = body
          .replaceAll(_wordTagRe, '')
          .replaceAll(_wsRe, ' ')
          .trim();
      if (plain.isEmpty) continue;
      out.add(LyricLine(time: -1, text: plain));
      continue;
    }

    final words = _parseEnhancedWords(body, offset);
    final clean = body
        .replaceAll(_wordTagRe, '')
        .replaceAll(_wsRe, ' ')
        .trim();
    for (final s in stamps) {
      out.add(LyricLine(time: s, text: clean, words: words));
    }
  }

  out.sort((a, b) => a.time.compareTo(b.time));
  return _withEstimatedWords(out);
}

bool isTtmlText(String text) => _ttmlRootRe.hasMatch(text);

LyricsKind lyricsKindOf(String text) {
  if (isTtmlText(text)) return LyricsKind.ttml;
  if (_stampRe.hasMatch(text)) return LyricsKind.lrc;
  return LyricsKind.text;
}

class _TtmlNode {
  _TtmlNode.element(this.local, this.attrs) : raw = null;
  _TtmlNode.text(this.raw) : local = '#text', attrs = const {};

  final String local;
  final Map<String, String> attrs;
  final String? raw;
  final List<_TtmlNode> children = [];

  bool get isText => raw != null;
}

class _TtmlWord {
  const _TtmlWord(this.time, this.text);

  final double time;
  final String text;
}

final _xmlCommentRe = RegExp(r'<!--[\s\S]*?-->');
final _xmlTagRe = RegExp(
  r"<(/?)([A-Za-z_][A-Za-z0-9_:.\-]*)((?:\x22[^\x22]*\x22|'[^']*'|[^>\x22'])*)>",
);
final _xmlAttrRe = RegExp(
  r"([A-Za-z_][A-Za-z0-9_:.\-]*)\s*=\s*(?:\x22([^\x22]*)\x22|'([^']*)')",
);
final _ttmlClockRe =
    RegExp(r'^(?:(\d+):)?(\d{1,2}):(\d{1,2})(?:[.:](\d{1,3}))?$');
final _ttmlSuffixRe = RegExp(r's$', caseSensitive: false);
final _ttmlRomanRe = RegExp(r'translation|romanization|romaji|translit');
final _ttmlBgRe = RegExp(r'(^|[-_])bg$|background');
final _ttmlSpaceRe = RegExp(r'[ \t\r\n]');

String _ttmlLocal(String name) {
  final i = name.indexOf(':');
  return (i < 0 ? name : name.substring(i + 1)).toLowerCase();
}

Map<String, String> _ttmlAttrs(String src) {
  final out = <String, String>{};
  for (final m in _xmlAttrRe.allMatches(src)) {
    out[m.group(1)!.toLowerCase()] = m.group(2) ?? m.group(3) ?? '';
  }
  return out;
}

String? _ttmlAttr(Map<String, String> attrs, String name) {
  final direct = attrs[name];
  if (direct != null) return direct;
  final local = _ttmlLocal(name);
  for (final entry in attrs.entries) {
    if (_ttmlLocal(entry.key) == local) return entry.value;
  }
  return null;
}

double _ttmlClock(String raw) {
  final value = raw.trim();
  final m = _ttmlClockRe.firstMatch(value);
  if (m != null) {
    final hours = double.tryParse(m.group(1) ?? '0') ?? 0;
    final minutes = double.tryParse(m.group(2)!);
    final seconds = double.tryParse(m.group(3)!);
    if (minutes != null && seconds != null) {
      final frac = m.group(4);
      final ms = frac == null || frac.isEmpty
          ? 0.0
          : double.tryParse('0.$frac') ?? 0;
      return hours * 3600 + minutes * 60 + seconds + ms;
    }
  }
  final plain = double.tryParse(value.replaceAll(_ttmlSuffixRe, ''));
  return plain != null && plain.isFinite ? plain : 0;
}

String _ttmlRoleOf(_TtmlNode node) =>
    (_ttmlAttr(node.attrs, 'ttm:role') ??
            _ttmlAttr(node.attrs, 'role') ??
            '')
        .toLowerCase();

String _decodeEntities(String input) => input
    .replaceAll('&lt;', '<')
    .replaceAll('&gt;', '>')
    .replaceAll('&quot;', '"')
    .replaceAll('&apos;', "'")
    .replaceAllMapped(
      RegExp(r'&#x([0-9a-fA-F]+);'),
      (m) => String.fromCharCode(int.parse(m.group(1)!, radix: 16)),
    )
    .replaceAllMapped(
      RegExp(r'&#(\d+);'),
      (m) => String.fromCharCode(int.parse(m.group(1)!)),
    )
    .replaceAll('&amp;', '&');

String _ttmlNormalise(String value) =>
    _decodeEntities(value).replaceAll(_wsRe, ' ').trim();

_TtmlNode _ttmlParse(String text) {
  final root = _TtmlNode.element('#root', const {});
  final stack = <_TtmlNode>[root];
  var cursor = 0;

  void pushText(String value) {
    if (value.isNotEmpty) stack.last.children.add(_TtmlNode.text(value));
  }

  for (final m in _xmlTagRe.allMatches(text)) {
    pushText(text.substring(cursor, m.start));
    cursor = m.end;

    final local = _ttmlLocal(m.group(2)!);
    if (m.group(1) == '/') {
      if (stack.length > 1 && stack.last.local == local) stack.removeLast();
      continue;
    }

    final attrSrc = m.group(3) ?? '';
    final node = _TtmlNode.element(local, _ttmlAttrs(attrSrc));
    stack.last.children.add(node);
    if (!attrSrc.trimRight().endsWith('/')) stack.add(node);
  }

  pushText(text.substring(cursor));
  return root;
}

Iterable<_TtmlNode> _ttmlDescendants(_TtmlNode node, String local) sync* {
  for (final child in node.children) {
    if (child.isText) continue;
    if (child.local == local) yield child;
    yield* _ttmlDescendants(child, local);
  }
}

String _ttmlRawText(_TtmlNode node) {
  final out = StringBuffer();
  for (final child in node.children) {
    if (child.isText) {
      out.write(child.raw);
    } else if (child.local == 'br') {
      out.write(' ');
    } else {
      out.write(_ttmlRawText(child));
    }
  }
  return out.toString();
}

bool _ttmlEndsWithSpace(StringBuffer buf) {
  if (buf.isEmpty) return false;
  final value = buf.toString();
  return _ttmlSpaceRe.hasMatch(value[value.length - 1]);
}

void _ttmlAddWord(
  List<_TtmlWord> words,
  bool separated,
  double time,
  String text,
) {
  if (words.isNotEmpty && !separated) {
    final last = words.last;
    words[words.length - 1] = _TtmlWord(last.time, last.text + text);
    return;
  }
  words.add(_TtmlWord(time, text));
}

void _ttmlWalk(
  _TtmlNode node,
  StringBuffer full,
  List<_TtmlWord> words,
  bool inBg,
) {
  for (final child in node.children) {
    if (child.isText) {
      full.write(child.raw);
      continue;
    }
    if (child.local == 'br') {
      full.write(' ');
      continue;
    }

    final bg = inBg || _ttmlBgRe.hasMatch(_ttmlRoleOf(child));
    final beginRaw = _ttmlAttr(child.attrs, 'begin');
    if (child.local == 'span' && beginRaw != null) {
      final raw = _ttmlRawText(child);
      final text = _ttmlNormalise(raw);
      if (text.isEmpty) continue;
      final separated = _ttmlEndsWithSpace(full);
      full.write(raw);
      if (!bg) _ttmlAddWord(words, separated, _ttmlClock(beginRaw), text);
      continue;
    }

    _ttmlWalk(child, full, words, bg);
  }
}

void _ttmlCollectLine(_TtmlNode p, List<LyricLine> out) {
  final beginRaw = _ttmlAttr(p.attrs, 'begin');
  if (beginRaw == null) return;
  final role = _ttmlRoleOf(p);
  if (_ttmlRomanRe.hasMatch(role) || _ttmlBgRe.hasMatch(role)) return;

  final full = StringBuffer();
  final raw = <_TtmlWord>[];
  _ttmlWalk(p, full, raw, false);

  final text = _ttmlNormalise(full.toString());
  if (text.isEmpty) return;

  out.add(
    LyricLine(
      time: _ttmlClock(beginRaw),
      text: text,
      words: raw.isEmpty
          ? null
          : [for (final w in raw) LyricWord(time: w.time, text: w.text)],
    ),
  );
}

List<LyricLine> parseTtmlText(String text) {
  final root = _ttmlParse(text.replaceAll(_xmlCommentRe, ''));
  final out = <LyricLine>[];
  for (final body in _ttmlDescendants(root, 'body')) {
    for (final p in _ttmlDescendants(body, 'p')) {
      _ttmlCollectLine(p, out);
    }
  }
  if (out.isEmpty) return const <LyricLine>[];
  out.sort((a, b) => a.time.compareTo(b.time));
  return _withEstimatedWords(out);
}

final _linesCache = Expando<List<LyricLine>>('vynl.lyricLines');

List<LyricLine> _parsePlainText(String text) {
  final out = <LyricLine>[];
  for (final raw in text.split(RegExp(r'\r?\n'))) {
    final line = raw
        .replaceAll(_stampRe, '')
        .replaceAll(_wordTagRe, '')
        .replaceAll(_wsRe, ' ')
        .trim();
    if (line.isNotEmpty) out.add(LyricLine(time: -1, text: line));
  }
  return out;
}

List<LyricLine> toLyricLines(LyricsResult result) {
  final cached = _linesCache[result];
  if (cached != null) return cached;
  final lines = switch (result.kind) {
    LyricsKind.lrc => parseLrcText(result.text),
    LyricsKind.ttml => parseTtmlText(result.text),
    LyricsKind.text => _lrcFromText(result.text) ?? _parsePlainText(result.text),
  };
  _linesCache[result] = lines;
  return lines;
}

List<LyricLine>? _lrcFromText(String text) {
  if (!_hasStampRe.hasMatch(text)) return null;
  final lines = parseLrcText(text);
  return lines.any((l) => l.time >= 0) ? lines : null;
}

final _remoteCache = <String, LyricsResult>{};

LyricsResult? lyricsFromRemote(String trackId, String text) {
  if (trackId.isEmpty || text.trim().isEmpty) return null;
  final cached = _remoteCache[trackId];
  if (cached != null) return cached;
  final result = LyricsResult(
    kind: lyricsKindOf(text),
    text: text,
    source: LyricsSource.embedded,
  );
  _remoteCache[trackId] = result;
  return result;
}

final _embeddedCache = <String, LyricsResult>{};

LyricsResult? lyricsFromTrack(String? embedded) {
  final trimmed = embedded?.trim() ?? '';
  if (trimmed.isEmpty) return null;
  final cached = _embeddedCache[trimmed];
  if (cached != null) return cached;
  final result = LyricsResult(
    kind: lyricsKindOf(trimmed),
    text: trimmed,
    source: LyricsSource.embedded,
  );
  _embeddedCache[trimmed] = result;
  return result;
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
  if (idx == lines.length - 1 &&
      t - lines[idx].time > _lastLineGraceSecs) {
    return -1;
  }
  return idx;
}

double wordEndAt(List<LyricWord> words, int i) {
  final w = words[i];
  if (i + 1 < words.length) {
    final next = words[i + 1];
    if (next.time > w.time) return next.time;
  }
  return w.time + 0.45;
}

double wordFill(List<LyricWord> words, int i, double t) {
  final w = words[i];
  if (t <= w.time) return 0;
  final end = wordEndAt(words, i);
  final span = end - w.time;
  if (span <= 0) return 1;
  final f = (t - w.time) / span;
  return f < 0 ? 0 : (f > 1 ? 1 : f);
}

List<LyricWord> _distributeWords(String text, double start, double end) {
  final parts = text.split(RegExp(r'\s+')).where((s) => s.isNotEmpty).toList();
  if (parts.isEmpty) return const [];
  final slice = (end - start) / parts.length;
  return [
    for (var i = 0; i < parts.length; i++)
      LyricWord(time: start + i * slice, text: parts[i]),
  ];
}

List<LyricLine> _withEstimatedWords(List<LyricLine> lines) {
  return [
    for (var i = 0; i < lines.length; i++)
      if (lines[i].words == null &&
          lines[i].time >= 0 &&
          !isInstrumental(lines[i].text))
        LyricLine(
          time: lines[i].time,
          text: lines[i].text,
          words: _distributeWords(
            lines[i].text,
            lines[i].time,
            i + 1 < lines.length && lines[i + 1].time > lines[i].time
                ? lines[i + 1].time
                : lines[i].time + 4,
          ),
        )
      else
        lines[i],
  ];
}

bool isInstrumental(String text) {
  final t = text.trim();
  if (t.isEmpty) return true;
  if (_symbolsOnlyRe.hasMatch(t)) return true;
  final unwrapped = t
      .replaceAll(_wrapOpenRe, '')
      .replaceAll(_wrapCloseRe, '')
      .trim();
  if (unwrapped.isEmpty) return true;
  if (_symbolsOnlyRe.hasMatch(unwrapped)) return true;
  return _instrumentalWordRe.hasMatch(unwrapped) ||
      _instrumentalPhraseRe.hasMatch(unwrapped);
}

final _stanzaCache = <String, List<List<String>>>{};

List<List<String>> stanzasOf(String text) {
  final cached = _stanzaCache[text];
  if (cached != null) return cached;
  final stanzas = <List<String>>[];
  var current = <String>[];
  for (final raw in text.split(RegExp(r'\r?\n'))) {
    final line = raw
        .replaceAll(_stampRe, '')
        .replaceAll(_wordTagRe, '')
        .replaceAll(_wsRe, ' ')
        .trim();
    if (line.isEmpty) {
      if (current.isNotEmpty) {
        stanzas.add(current);
        current = <String>[];
      }
      continue;
    }
    current.add(line);
  }
  if (current.isNotEmpty) stanzas.add(current);
  _stanzaCache[text] = stanzas;
  return stanzas;
}