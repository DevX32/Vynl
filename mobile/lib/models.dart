class CatalogTrack {
  CatalogTrack({
    required this.id,
    required this.title,
    required this.artist,
    required this.album,
    this.year,
    required this.duration,
    this.trackNumber,
    this.lyrics,
    required this.ext,
    this.mtime,
    required this.hasCover,
    this.localAudioPath,
    this.localCoverPath,
  });

  final String id;
  final String title;
  final String artist;
  final String album;
  final int? year;
  final double duration;
  final int? trackNumber;
  final String? lyrics;
  final String ext;
  final int? mtime;
  final bool hasCover;
  final String? localAudioPath;
  final String? localCoverPath;

  bool get isDownloaded =>
      localAudioPath != null && localAudioPath!.isNotEmpty;

  factory CatalogTrack.fromJson(Map<String, dynamic> json) {
    return CatalogTrack(
      id: json['id'] as String,
      title: (json['title'] as String?) ?? 'Unknown',
      artist: (json['artist'] as String?) ?? 'Unknown',
      album: (json['album'] as String?) ?? '',
      year: (json['year'] as num?)?.toInt(),
      duration: (json['duration'] as num?)?.toDouble() ?? 0,
      trackNumber: (json['trackNumber'] as num?)?.toInt(),
      lyrics: json['lyrics'] as String?,
      ext: (json['ext'] as String?) ?? 'mp3',
      mtime: (json['mtime'] as num?)?.toInt(),
      hasCover: json['hasCover'] == true,
    );
  }

  CatalogTrack copyWith({
    String? localAudioPath,
    String? localCoverPath,
    bool clearAudio = false,
    bool clearCover = false,
  }) {
    return CatalogTrack(
      id: id,
      title: title,
      artist: artist,
      album: album,
      year: year,
      duration: duration,
      trackNumber: trackNumber,
      lyrics: lyrics,
      ext: ext,
      mtime: mtime,
      hasCover: hasCover,
      localAudioPath: clearAudio ? null : (localAudioPath ?? this.localAudioPath),
      localCoverPath: clearCover ? null : (localCoverPath ?? this.localCoverPath),
    );
  }
}

class SyncManifestEntry {
  SyncManifestEntry({
    required this.id,
    this.mtime,
    required this.size,
    required this.hasCover,
    required this.ext,
  });

  final String id;
  final int? mtime;
  final int size;
  final bool hasCover;
  final String ext;

  factory SyncManifestEntry.fromJson(Map<String, dynamic> json) {
    return SyncManifestEntry(
      id: json['id'] as String,
      mtime: (json['mtime'] as num?)?.toInt(),
      size: (json['size'] as num?)?.toInt() ?? 0,
      hasCover: json['hasCover'] == true,
      ext: (json['ext'] as String?) ?? 'mp3',
    );
  }
}

class PlaylistDto {
  PlaylistDto({
    required this.id,
    required this.name,
    required this.trackIds,
    this.cover,
    required this.createdAt,
    required this.updatedAt,
  });

  final String id;
  final String name;
  final List<String> trackIds;
  final String? cover;
  final int createdAt;
  final int updatedAt;

  factory PlaylistDto.fromJson(Map<String, dynamic> json) {
    return PlaylistDto(
      id: json['id'] as String,
      name: (json['name'] as String?) ?? 'Playlist',
      trackIds: (json['trackIds'] as List<dynamic>? ?? [])
          .map((e) => e.toString())
          .toList(),
      cover: json['cover'] as String?,
      createdAt: (json['createdAt'] as num?)?.toInt() ?? 0,
      updatedAt: (json['updatedAt'] as num?)?.toInt() ?? 0,
    );
  }
}

class PairingCredentials {
  PairingCredentials({
    required this.baseUrl,
    required this.token,
  });

  final String baseUrl;
  final String token;
}
