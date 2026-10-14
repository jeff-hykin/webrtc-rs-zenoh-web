# zenoh-web-rtc-srtp: patches on top of rtc-srtp 0.21.0

Fork of [rtc-srtp](https://crates.io/crates/rtc-srtp) 0.21.0 (webrtc-rs, MIT OR Apache-2.0), kept in
<https://github.com/jeff-hykin/webrtc-rs-zenoh-web/tree/main/rtc-srtp>. Every change is marked
`zenoh-web patch` in the source. Library name is unchanged (`rtc_srtp`).

1. **An SSRC's state can be freed** (`src/context/mod.rs` `Context::remove_ssrc`). Bug: a context kept
   the SRTP and SRTCP state (rollover counter, replay window) of every SSRC it ever saw, with no way to
   drop it, so each stream a long-lived connection stopped stayed in both maps. Change: `remove_ssrc`,
   which `zenoh-web-rtc` calls when a sender's or receiver's streams end.

Tests: `src/context/context_test.rs` (search `zenoh-web patch`).

Manifest: example and benchmark removed (`criterion` dev-dependency with them). Versioning: upstream
version + `-zw.N` (`0.21.0-zw.4`, which follows the fork's shared version).
