# webrtc-rs for zenoh-web

[webrtc-rs](https://github.com/webrtc-rs/webrtc) 0.21.0 crates with small fixes, published under
new names so every crate depending on [zenoh-web](https://github.com/jeff-hykin/zenoh-web) gets the
fixed code (a `[patch.crates-io]` section only applies in the top-level workspace, so dependents
would silently build the unpatched upstream crates). What changed and why: [PATCHES.md](PATCHES.md).
License files are upstream's (MIT OR Apache-2.0, © WebRTC.rs).
