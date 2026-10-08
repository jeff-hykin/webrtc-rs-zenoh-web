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
3. **Dependencies point at the forks**: `sctp` is `zenoh-web-rtc-sctp`, `datachannel` is
   `zenoh-web-rtc-datachannel` (which uses the same SCTP fork, so their types match).

Manifest: dev-dependencies, examples and tests removed (not vendored).
Versioning: upstream version + `-zw.N` (`0.21.0-zw.3`: patch 2); bump `N` for new fork fixes, reset it when rebasing on a new upstream.
