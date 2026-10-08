# Patches

| package | upstream | changes |
|---|---|---|
| `zenoh-web-rtc-sctp` | `rtc-sctp` | the SCTP fixes: [rtc-sctp/PATCHES.md](rtc-sctp/PATCHES.md) |
| `zenoh-web-rtc-datachannel` | `rtc-datachannel` | none, on the SCTP fork: [rtc-datachannel/PATCHES.md](rtc-datachannel/PATCHES.md) |
| `zenoh-web-rtc` | `rtc` | the data channel reliability and stream-id reuse fixes, on the two forks above: [rtc/PATCHES.md](rtc/PATCHES.md) |
| `zenoh-web-webrtc` | `webrtc` | the negotiated header extension ids for bound tracks, on the rtc fork: [webrtc/PATCHES.md](webrtc/PATCHES.md) |

Every change is marked `zenoh-web patch` in the source. Library names are upstream's (`rtc_sctp`,
`rtc_datachannel`, `rtc`, `webrtc`), so code only renames the package in its `Cargo.toml`:

```toml
webrtc = { package = "zenoh-web-webrtc", version = "=0.21.0-zw.3" }
```

## Versions

Upstream version + `-zw.N`; bump `N` for new fork fixes, reset it when rebasing on a new upstream.
All four crates share one version and depend on each other by exact version (and path, inside
this repository).

- `0.21.0-zw.1`: SCTP abandons fragmented partially-reliable messages whole, drops a reset
  stream's unsent chunks, doesn't re-run a repeated reset, caps the retransmission backoff;
  browser-opened data channels honor their reliability.
- `0.21.0-zw.2`: SCTP retransmission timeout as Chrome's dcsctp computes it; a bound track learns
  the negotiated header extension ids.
- `0.21.0-zw.3`: a closed data channel's stream id is freed only once both directions are reset (a
  channel reopened at once no longer gets the dying stream).

## Publishing

Each crate needs the previous ones on crates.io, so publish in order: `rtc-sctp`, `rtc-datachannel`,
`rtc`, `webrtc` (`cargo publish -p zenoh-web-rtc-sctp`, ...; `cargo package --list -p <package>`
first). Tests: `cargo test --workspace` (`rtc-sctp` keeps upstream's unit tests).

`rtc-sctp` and `rtc` also allow clippy's lints in their test builds (`cfg_attr(test, ...)` in
`lib.rs`): upstream's unit tests aren't clippy-clean; the libraries are.
