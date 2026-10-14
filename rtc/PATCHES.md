# zenoh-web-rtc: patches on top of rtc 0.21.0

Fork of [rtc](https://crates.io/crates/rtc) 0.21.0 (webrtc-rs, MIT OR Apache-2.0), kept in
<https://github.com/jeff-hykin/webrtc-rs-zenoh-web/tree/main/rtc>. Library name is unchanged (`rtc`).

1. **Peer-opened data channels honor their reliability** (`src/peer_connection/handler/sctp.rs`, marked
   `zenoh-web patch`). Bug: the DCEP reliability (unordered, `maxRetransmits`, `maxPacketLifeTime`) was
   only applied to channels this side opened, so every channel a browser opened stayed ordered and fully
   reliable in the server's send direction. Change: apply the peer's DATA_CHANNEL_OPEN parameters to
   the stream when the open message arrives.
2. **A closed data channel's stream id is freed only once both directions are reset**
   (`src/peer_connection/handler/sctp.rs`, marked `zenoh-web patch`). Bug: closing a channel here freed its SCTP
   stream id at once, while RFC 8831 §6.7 allows reuse only after both directions are reset; a channel opened
   right after the close got the same id, and the old stream's reset then closed it (or the peer dropped its
   DCEP OPEN). Change: the local close no longer reports the stream closed; the peer's reset of its side does.
3. **A closed data channel leaves nothing behind** (`src/peer_connection/handler/datachannel.rs`,
   `src/data_channel/mod.rs` `close`, `src/data_channel/registry.rs` `remove`; marked `zenoh-web patch`).
   Bugs: a channel's stats accumulator stayed after it closed, and a channel closed before SCTP was up
   (never dialed, so no stream would ever close) stayed registered. Change: the stream's close removes
   the stats; closing an undialed channel removes it, its stats, and fires `OnClose`.
4. **Stopped transceivers are removed, and their streams' state freed** (`src/peer_connection/internal.rs`
   `remove_stopped_transceivers` / `stop_transceiver` / `release_streams`, `src/peer_connection/mod.rs`
   descriptions and `remove_track`, `src/peer_connection/sdp/mod.rs` rejected m-lines,
   `src/statistics/accumulator/mod.rs`; marked `zenoh-web patch`). Bug: a stopped transceiver stayed in
   the connection for its whole life, as did its streams' stats and SRTP state (each video resubscribe in
   zenoh-gateway left one of each). Changes, per W3C webrtc 4.4.1.6 and RFC 9429: a transceiver id is a
   stable counter, not the position (handles resolve the position when made); stopping frees the stats
   and SRTP state of its SSRCs (`remove_track` does the same for the sender's); a rejected m-line in a
   remote description stops its transceiver; once an answer is applied, stopped transceivers whose
   m-line is rejected or gone are removed. A stopped transceiver's m-line is written rejected (port 0);
   a rejected m-line with no transceiver left stays rejected in its place, and a new transceiver
   recycles it (under a new mid) when we offer. Candidates go on the first non-rejected section.
5. **Dependencies point at the forks**: `srtp` is `zenoh-web-rtc-srtp`, `sctp` is `zenoh-web-rtc-sctp`, `datachannel` is
   `zenoh-web-rtc-datachannel` (which uses the same SCTP fork, so their types match).

Manifest: dev-dependencies, examples and tests removed (not vendored).
Versioning: upstream version + `-zw.N` (`0.21.0-zw.4`, no change of its own since zw.3: the forks share one version); bump `N` for new fork fixes, reset it when rebasing on a new upstream.
