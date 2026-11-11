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
    this.streamUrl,
    this.streamHeaders = const {},
    this.coverUrl,
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
  final Uri? streamUrl;
  final Map<String, String> streamHeaders;
  final String? coverUrl;

  bool get isDownloaded =>
      localAudioPath != null && localAudioPath!.isNotEmpty;

  bool get isStream => streamUrl != null;

  bool get isPlayable => isDownloaded || isStream;

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
      streamUrl: streamUrl,
      streamHeaders: streamHeaders,
      coverUrl: coverUrl,
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
    this.streamed = const [],
    this.hasCover = false,
    this.cover,
    required this.createdAt,
    required this.updatedAt,
  });

  final String id;
  final String name;
  final List<String> trackIds;
  final List<StreamTrack> streamed;

  final bool hasCover;
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
      streamed: (json['streamed'] as List<dynamic>? ?? [])
          .map((e) => StreamTrack.fromJson(e as Map<String, dynamic>))
          .toList(),
      hasCover: json['hasCover'] == true,
      createdAt: (json['createdAt'] as num?)?.toInt() ?? 0,
      updatedAt: (json['updatedAt'] as num?)?.toInt() ?? 0,
    );
  }
}

class StreamTrack {
  StreamTrack({
    required this.id,
    required this.title,
    required this.artist,
    this.album,
    this.duration,
    this.cover,
  });

  final String id;
  final String title;
  final String artist;
  final String? album;
  final double? duration;
  final String? cover;

  factory StreamTrack.fromJson(Map<String, dynamic> json) {
    return StreamTrack(
      id: json['id'] as String? ?? '',
      title: (json['title'] as String?) ?? 'Unknown',
      artist: (json['artist'] as String?) ?? 'Unknown',
      album: json['album'] as String?,
      duration: (json['duration'] as num?)?.toDouble(),
      cover: json['cover'] as String?,
    );
  }

  CatalogTrack toCatalogTrack({
    Uri? streamUrl,
    Map<String, String> streamHeaders = const {},
  }) {
    return CatalogTrack(
      id: 'ytm:$id',
      title: title,
      artist: artist,
      album: album ?? '',
      duration: duration ?? 0,
      ext: 'webm',
      hasCover: cover != null && cover!.isNotEmpty,
      streamUrl: streamUrl,
      streamHeaders: streamHeaders,
      coverUrl: cover,
    );
  }
}
class CatalogueAlbum {
  CatalogueAlbum({
    required this.id,
    required this.title,
    required this.artist,
    this.year,
    this.trackCount,
    this.cover,
  });

  final String id;
  final String title;
  final String artist;
  final int? year;
  final int? trackCount;
  final String? cover;

  factory CatalogueAlbum.fromJson(Map<String, dynamic> json) {
    return CatalogueAlbum(
      id: json['id'] as String? ?? '',
      title: (json['title'] as String?) ?? 'Unknown',
      artist: (json['artist'] as String?) ?? '',
      year: (json['year'] as num?)?.toInt(),
      trackCount: (json['trackCount'] as num?)?.toInt(),
      cover: json['cover'] as String?,
    );
  }
}

class CataloguePlaylist {
  CataloguePlaylist({
    required this.id,
    required this.title,
    this.author,
    this.trackCount,
    this.cover,
  });

  final String id;
  final String title;
  final String? author;
  final int? trackCount;
  final String? cover;

  factory CataloguePlaylist.fromJson(Map<String, dynamic> json) {
    return CataloguePlaylist(
      id: json['id'] as String? ?? '',
      title: (json['title'] as String?) ?? 'Unknown',
      author: json['author'] as String?,
      trackCount: (json['trackCount'] as num?)?.toInt(),
      cover: json['cover'] as String?,
    );
  }
}

class CatalogueStream {
  const CatalogueStream({required this.url, required this.mime});

  final String url;
  final String mime;
}

class CatalogueSearchResults {
  CatalogueSearchResults({
    required this.songs,
    required this.albums,
    required this.playlists,
  });

  final List<StreamTrack> songs;
  final List<CatalogueAlbum> albums;
  final List<CataloguePlaylist> playlists;

  factory CatalogueSearchResults.fromJson(Map<String, dynamic> json) {
    return CatalogueSearchResults(
      songs: (json['songs'] as List<dynamic>? ?? [])
          .map((e) => StreamTrack.fromJson(e as Map<String, dynamic>))
          .toList(),
      albums: (json['albums'] as List<dynamic>? ?? [])
          .map((e) => CatalogueAlbum.fromJson(e as Map<String, dynamic>))
          .toList(),
      playlists: (json['playlists'] as List<dynamic>? ?? [])
          .map((e) => CataloguePlaylist.fromJson(e as Map<String, dynamic>))
          .toList(),
    );
  }
}

class CatalogueAlbumDetail {
  CatalogueAlbumDetail({
    required this.album,
    this.description,
    this.durationText,
    required this.tracks,
  });

  final CatalogueAlbum album;
  final String? description;
  final String? durationText;
  final List<StreamTrack> tracks;

  factory CatalogueAlbumDetail.fromJson(Map<String, dynamic> json) {
    return CatalogueAlbumDetail(
      album: CatalogueAlbum.fromJson(
        json['album'] as Map<String, dynamic>? ?? <String, dynamic>{},
      ),
      description: json['description'] as String?,
      durationText: json['durationText'] as String?,
      tracks: (json['tracks'] as List<dynamic>? ?? [])
          .map((e) => StreamTrack.fromJson(e as Map<String, dynamic>))
          .toList(),
    );
  }
}

class CatalogueArtist {
  CatalogueArtist({
    required this.id,
    required this.name,
    this.description,
    this.subscribers,
    this.cover,
    required this.topTracks,
    required this.albums,
    required this.singles,
  });

  final String id;
  final String name;
  final String? description;
  final String? subscribers;
  final String? cover;
  final List<StreamTrack> topTracks;
  final List<CatalogueAlbum> albums;
  final List<CatalogueAlbum> singles;

  factory CatalogueArtist.fromJson(Map<String, dynamic> json) {
    List<CatalogueAlbum> albums(String key) =>
        (json[key] as List<dynamic>? ?? [])
            .map((e) => CatalogueAlbum.fromJson(e as Map<String, dynamic>))
            .toList();
    return CatalogueArtist(
      id: json['id'] as String? ?? '',
      name: (json['name'] as String?) ?? 'Unknown',
      description: json['description'] as String?,
      subscribers: json['subscribers'] as String?,
      cover: json['cover'] as String?,
      topTracks: (json['topTracks'] as List<dynamic>? ?? [])
          .map((e) => StreamTrack.fromJson(e as Map<String, dynamic>))
          .toList(),
      albums: albums('albums'),
      singles: albums('singles'),
    );
  }
}

class CataloguePlaylistDetail {
  CataloguePlaylistDetail({
    required this.playlist,
    required this.tracks,
    this.continuation,
  });

  final CataloguePlaylist playlist;
  final List<StreamTrack> tracks;
  final String? continuation;

  factory CataloguePlaylistDetail.fromJson(Map<String, dynamic> json) {
    return CataloguePlaylistDetail(
      playlist: CataloguePlaylist.fromJson(
        json['playlist'] as Map<String, dynamic>? ?? <String, dynamic>{},
      ),
      tracks: (json['tracks'] as List<dynamic>? ?? [])
          .map((e) => StreamTrack.fromJson(e as Map<String, dynamic>))
          .toList(),
      continuation: json['continuation'] as String?,
    );
  }
}

class MobilePlayerState {
  MobilePlayerState({
    required this.playing,
    required this.position,
    this.duration,
    required this.volume,
    this.track,
    required this.queue,
  });

  final bool playing;
  final double position;
  final double? duration;
  final int volume;
  final StreamTrack? track;
  final List<StreamTrack> queue;

  factory MobilePlayerState.fromJson(Map<String, dynamic> json) {
    final track = json['track'];
    return MobilePlayerState(
      playing: json['playing'] == true,
      position: (json['position'] as num?)?.toDouble() ?? 0,
      duration: (json['duration'] as num?)?.toDouble(),
      volume: (json['volume'] as num?)?.toInt() ?? 0,
      track: track is Map<String, dynamic>
          ? StreamTrack.fromJson(track)
          : null,
      queue: (json['queue'] as List<dynamic>? ?? [])
          .map((e) => StreamTrack.fromJson(e as Map<String, dynamic>))
          .toList(),
    );
  }
}

class MobileTaste {
  MobileTaste({required this.liked, required this.likedCount, required this.playedCount});

  final List<StreamTrack> liked;
  final int likedCount;
  final int playedCount;

  factory MobileTaste.fromJson(Map<String, dynamic> json) {
    final summary = json['summary'] as Map<String, dynamic>? ?? <String, dynamic>{};
    return MobileTaste(
      liked: (json['liked'] as List<dynamic>? ?? [])
          .map((e) => StreamTrack.fromJson(e as Map<String, dynamic>))
          .toList(),
      likedCount: (summary['liked'] as num?)?.toInt() ?? 0,
      playedCount: (summary['played'] as num?)?.toInt() ?? 0,
    );
  }
}

class PairingCredentials {
  PairingCredentials({
    required this.baseUrl,
    required this.token,
    this.alternates = const [],
  });

  final String baseUrl;
  final String token;
  final List<String> alternates;

  PairingCredentials copyWith({
    String? baseUrl,
    String? token,
    List<String>? alternates,
  }) => PairingCredentials(
    baseUrl: baseUrl ?? this.baseUrl,
    token: token ?? this.token,
    alternates: alternates ?? this.alternates,
  );
}
