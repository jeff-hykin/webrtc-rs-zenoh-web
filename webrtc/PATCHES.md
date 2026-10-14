# zenoh-web-webrtc: webrtc 0.21.0 on the rtc fork

Fork of [webrtc](https://crates.io/crates/webrtc) 0.21.0 (webrtc-rs, MIT OR Apache-2.0), kept in
<https://github.com/jeff-hykin/webrtc-rs-zenoh-web/tree/main/webrtc>. Library name is unchanged (`webrtc`).

Its `rtc` dependency is `zenoh-web-rtc` instead of `rtc`, so applications built on it (zenoh-web)
get the patched SCTP and data channel code.

1. **A bound track learns the negotiated header extension ids** (`src/media_stream/track_local/`:
   `TrackLocal::update_parameters`, implemented by `TrackLocalStaticRTP` / `TrackLocalStaticSample`;
   `src/peer_connection/mod.rs` `refresh_track_parameters`, run after every applied local or remote
   description; marked `zenoh-web patch`). Bug: `add_track` binds the track at once, before
   negotiation, so its context kept the media engine's provisional extension ids; after the answer
   the sender validates packets against the negotiated ids, and every packet carrying an extension
   written by URI (e.g. `SampleWriter::with_extension(PlayoutDelay)`) was refused with "extension
   not found": no video at all. Change: hand the track its sender's current parameters after each
   description is applied.

2. **`close()` frees the connection** (`src/peer_connection/mod.rs` `close`; marked `zenoh-web patch`).
   Bug: `RtpTransceiverImpl` holds `Arc<PeerConnectionRef>` and `PeerConnectionRef` holds the
   transceivers, so a connection that had negotiated a track was never freed (its handler, SCTP, DTLS
   and media buffers: ~2 MB a browser in zenoh-gateway), and its data channels' `poll()` never ended,
   their event senders living in the connection. Change: `close()` clears the transceivers and the
   track and data channel event senders.

3. **A closed data channel's event sender is freed** (`src/peer_connection/driver.rs`
   `deliver_data_channel_event` / `flush_pending_data_channel_events`, `src/data_channel/mod.rs`
   `Drop`; marked `zenoh-web patch`). Bug: the sender was removed only by the channel's `Drop`, and only
   if `try_lock` got the map, so a channel dropped while the driver held it, or one the application
   kept, stayed in the map for the life of the connection. Change: the driver drops the sender once it
   has handed the channel its `OnClose` (its `poll` then ends after the close); `Drop` falls back to a
   task when the map is busy.

4. **A removed transceiver is released** (`src/peer_connection/mod.rs` `prune_transceivers`, run after
   every applied description and by `get_transceivers`; `src/rtp_transceiver/mod.rs` `release`; marked
   `zenoh-web patch`). Bug: the wrapper kept every transceiver (and with it the connection, its
   sender's bound track and that track's RTCP event sender) for the life of the connection. Change:
   when the core removes a stopped transceiver, the wrapper drops it, unbinds its sender's track and
   drops both tracks' event senders.

Manifest: dev-dependencies, examples and tests removed (not vendored). Versioning: upstream version + `-zw.N` (`0.21.0-zw.4`: patch 2).
