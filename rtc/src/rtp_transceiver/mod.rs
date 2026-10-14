//! RTP Media API
//!
//! This module provides the RTCRtpTransceiver, which represents a permanent pairing
//! of an [`RTCRtpSender`](rtp_sender::RTCRtpSender) and an [`RTCRtpReceiver`](rtp_receiver::RTCRtpReceiver),
//! along with shared state.
//!
//! # Overview
//!
//! A transceiver manages bidirectional media exchange for a single media type (audio or video).
//! It combines:
//! - An RTP sender for outgoing media
//! - An RTP receiver for incoming media  
//! - Shared state including direction, mid, and codec preferences
//!
//! # Examples
//!
//! ## Adding a transceiver from a track
//!
//! ```no_run
//! # use std::time::Instant;
//! # use rtc::peer_connection::RTCPeerConnectionBuilder;
//! # use rtc::media_stream::MediaStreamTrack;
//! # use rtc::rtp_transceiver::{RTCRtpTransceiverInit, RTCRtpTransceiverDirection};
//! # fn example(audio_track: MediaStreamTrack) -> Result<(), Box<dyn std::error::Error>> {
//! let mut peer_connection = RTCPeerConnectionBuilder::new().build(Instant::now())?;
//!
//! // Add a transceiver for sending audio
//! let init = RTCRtpTransceiverInit {
//!     direction: RTCRtpTransceiverDirection::Sendrecv,
//!     ..Default::default()
//! };
//!
//! let sender_id = peer_connection
//!     .add_transceiver_from_track(audio_track, Some(init))?;
//!
//! println!("Added sender with ID: {:?}", sender_id);
//! # Ok(())
//! # }
//! ```
//!
//! ## Adding a transceiver by media kind
//!
//! ```no_run
//! # use std::time::Instant;
//! # use rtc::peer_connection::RTCPeerConnectionBuilder;
//! # use rtc::rtp_transceiver::rtp_sender::RtpCodecKind;
//! # use rtc::rtp_transceiver::{RTCRtpTransceiverInit, RTCRtpTransceiverDirection};
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut peer_connection = RTCPeerConnectionBuilder::new().build(Instant::now())?;
//!
//! // Add a video transceiver for receiving only
//! let init = RTCRtpTransceiverInit {
//!     direction: RTCRtpTransceiverDirection::Recvonly,
//!     ..Default::default()
//! };
//!
//! let receiver_id = peer_connection
//!     .add_transceiver_from_kind(RtpCodecKind::Video, Some(init))?;
//!
//! println!("Added receiver with ID: {:?}", receiver_id);
//! # Ok(())
//! # }
//! ```
//!
//! ## Controlling transceiver direction
//!
//! ```no_run
//! # use std::time::Instant;
//! # use rtc::peer_connection::RTCPeerConnectionBuilder;
//! # use rtc::rtp_transceiver::RTCRtpTransceiverDirection;
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut peer_connection = RTCPeerConnectionBuilder::new().build(Instant::now())?;
//!
//! // Iterate through transceivers and change direction
//! for transceiver_id in peer_connection.get_transceivers() {
//!     // Access the transceiver through peer_connection's internal methods
//!     // Note: Direct transceiver access may be internal API
//!     // This demonstrates the concept - actual usage depends on your API design
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Setting codec preferences
//!
//! ```no_run
//! # use std::time::Instant;
//! # use rtc::peer_connection::RTCPeerConnectionBuilder;
//! # use rtc::rtp_transceiver::rtp_sender::RTCRtpCodecParameters;
//! # fn example(preferred_codecs: Vec<RTCRtpCodecParameters>) -> Result<(), Box<dyn std::error::Error>> {
//! let mut peer_connection = RTCPeerConnectionBuilder::new().build(Instant::now())?;
//!
//! // Codec preferences would be set through peer connection methods
//! // The exact API depends on your implementation
//! # Ok(())
//! # }
//! ```
//!
//! # Specification
//!
//! See [RTCRtpTransceiver](https://www.w3.org/TR/webrtc/#dom-rtcrtptransceiver) in the W3C WebRTC specification.

use crate::media_stream::MediaStreamId;
use crate::peer_connection::RTCPeerConnection;
use crate::rtp_transceiver::rtp_sender::RTCRtpCodecParameters;
use crate::rtp_transceiver::rtp_sender::rtp_encoding_parameters::RTCRtpEncodingParameters;
pub use direction::RTCRtpTransceiverDirection;
use log::trace;
use shared::error::Result;

pub(crate) mod direction;
pub(crate) mod fmtp;
pub(crate) mod internal;
pub mod rtp_receiver;
pub mod rtp_sender;

/// SSRC (Synchronization Source) identifier.
///
/// A synchronization source is a randomly chosen value meant to be globally unique
/// within a particular RTP session. It is used to identify a single stream of media.
///
/// # Specification
///
/// See [RFC 3550 Section 3](https://datatracker.ietf.org/doc/html/rfc3550#section-3).
#[allow(clippy::upper_case_acronyms)]
pub type SSRC = u32;

/// RTP payload type identifier.
///
/// Identifies the format of the RTP payload and determines its interpretation by the
/// application. Each codec in an RTP session will have a different payload type.
///
/// # Specification
///
/// See [RFC 3550 Section 3](https://datatracker.ietf.org/doc/html/rfc3550#section-3).
pub type PayloadType = u8;

/// RTP stream identifier.
///
/// Is used for unique identification of RTP stream
///
/// # Specification
///
/// See [RFC 8852 Section 3.1](https://datatracker.ietf.org/doc/html/rfc8852#section-3.1).
pub type RtpStreamId = String;

/// Repaired RTP stream identifier.
///
/// Is used to identify which stream is to be repaired using a redundancy RTP stream
///
/// # Specification
///
/// See [RFC 8852 Section 3.2](https://datatracker.ietf.org/doc/html/rfc8852#section-3.2).
pub type RepairedStreamId = String;

/// Internal identifier for an RTP transceiver.
pub type RTCRtpTransceiverId = usize;

impl From<RTCRtpSenderId> for RTCRtpTransceiverId {
    fn from(id: RTCRtpSenderId) -> Self {
        id.0
    }
}

impl From<RTCRtpReceiverId> for RTCRtpTransceiverId {
    fn from(id: RTCRtpReceiverId) -> Self {
        id.0
    }
}

/// Identifier for an `RTCRtpSender` within a peer connection.
///
/// Used to reference a specific RTP sender when calling methods like `remove_track`.
#[derive(Default, Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct RTCRtpSenderId(pub(crate) RTCRtpTransceiverId);

impl From<RTCRtpTransceiverId> for RTCRtpSenderId {
    fn from(id: RTCRtpTransceiverId) -> Self {
        Self(id)
    }
}

/// Identifier for an `RTCRtpReceiver` within a peer connection.
///
/// Used to reference a specific RTP receiver when handling incoming media.
#[derive(Default, Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct RTCRtpReceiverId(pub(crate) RTCRtpTransceiverId);

impl From<RTCRtpTransceiverId> for RTCRtpReceiverId {
    fn from(id: RTCRtpTransceiverId) -> Self {
        Self(id)
    }
}

/// Initialization parameters for creating an `RTCRtpTransceiver`.
///
/// Used with `add_transceiver_from_track` or `add_transceiver_from_kind` to configure
/// the transceiver's initial direction and encoding parameters.
///
/// # Specification
///
/// See [RTCRtpTransceiverInit](https://www.w3.org/TR/webrtc/#dom-rtcrtptransceiverinit)
#[derive(Default, Clone)]
pub struct RTCRtpTransceiverInit {
    /// The initial direction of the transceiver.
    pub direction: RTCRtpTransceiverDirection,
    /// The stream IDs associated with the transceiver's sender.
    pub streams: Vec<MediaStreamId>,
    /// The encoding parameters to use when sending media.
    pub send_encodings: Vec<RTCRtpEncodingParameters>,
}

/// RTCRtpTransceiver represents a permanent pairing of an RTP sender and RTP receiver that share a common mid.
/// The transceiver manages the direction of media flow and codec preferences.
///
/// # Specification
///
/// See [RTCRtpTransceiver](https://www.w3.org/TR/webrtc/#dom-rtcrtptransceiver) in the W3C WebRTC specification.
pub struct RTCRtpTransceiver<'a> {
    pub(crate) id: RTCRtpTransceiverId,
    /// zenoh-web patch: the transceiver's position, resolved from `id` when the handle was made
    /// (the mutable borrow keeps it valid: nothing can be added or removed meanwhile)
    pub(crate) index: usize,
    pub(crate) peer_connection: &'a mut RTCPeerConnection,
}

impl RTCRtpTransceiver<'_> {
    /// Returns the media stream identification tag (mid) for this transceiver.
    ///
    /// The mid uniquely identifies the media description in the SDP. When not already set,
    /// this value will be assigned during `create_offer` or `create_answer`.
    ///
    /// # Specification
    ///
    /// See [RTCRtpTransceiver.mid](https://www.w3.org/TR/webrtc/#dom-rtptransceiver-mid).
    pub fn mid(&self) -> &Option<String> {
        // peer_connection is mutable borrow, its rtp_transceivers won't be resized,
        // so, [self.index] here is safe.
        self.peer_connection.rtp_transceivers[self.index].mid()
    }

    /// Returns the id of this transceiver's [`RTCRtpSender`], if it has one.
    ///
    /// [`RTCRtpSender`]: crate::rtp_transceiver::rtp_sender::RTCRtpSender
    pub fn sender(&self) -> Option<RTCRtpSenderId> {
        // peer_connection is mutable borrow, its rtp_transceivers won't be resized,
        // so, [self.index] here is safe.
        if self.peer_connection.rtp_transceivers[self.index]
            .sender()
            .is_some()
        {
            Some(RTCRtpSenderId::from(self.id))
        } else {
            None
        }
    }

    /// Returns the id of this transceiver's [`RTCRtpReceiver`], if it has one.
    ///
    /// [`RTCRtpReceiver`]: crate::rtp_transceiver::rtp_receiver::RTCRtpReceiver
    pub fn receiver(&self) -> Option<RTCRtpReceiverId> {
        // peer_connection is mutable borrow, its rtp_transceivers won't be resized,
        // so, [self.index] here is safe.
        if self.peer_connection.rtp_transceivers[self.index]
            .receiver()
            .is_some()
        {
            Some(RTCRtpReceiverId::from(self.id))
        } else {
            None
        }
    }

    /// Returns the preferred direction of the transceiver.
    ///
    /// This indicates the direction that the application prefers for media flow.
    ///
    /// # Specification
    ///
    /// See [RTCRtpTransceiver.direction](https://www.w3.org/TR/webrtc/#dom-rtcrtptransceiver-direction).
    pub fn direction(&self) -> RTCRtpTransceiverDirection {
        // peer_connection is mutable borrow, its rtp_transceivers won't be resized,
        // so, [self.index] here is safe.
        self.peer_connection.rtp_transceivers[self.index].direction()
    }

    /// Sets the preferred direction of this transceiver.
    ///
    /// Changing the direction may trigger renegotiation to update the session description.
    ///
    /// # Specification
    ///
    /// See [RTCRtpTransceiver.direction](https://www.w3.org/TR/webrtc/#dom-rtcrtptransceiver-direction).
    pub fn set_direction(&mut self, direction: RTCRtpTransceiverDirection) {
        // peer_connection is mutable borrow, its rtp_transceivers won't be resized,
        // so, [self.index] here is safe.
        let previous_direction = self.peer_connection.rtp_transceivers[self.index].direction();
        self.peer_connection.rtp_transceivers[self.index].set_direction(direction);
        // Compare the effective direction after the setter (rather than the requested value) so
        // any future normalization inside `set_direction` cannot bypass this check.
        let current_direction = self.peer_connection.rtp_transceivers[self.index].direction();

        // https://www.w3.org/TR/webrtc/#dom-rtcrtptransceiver-direction
        // The direction setter is a no-op when the new direction equals the current one;
        // otherwise its final step updates the negotiation-needed flag for the connection,
        // which may fire a `negotiationneeded` event.
        if current_direction != previous_direction {
            trace!(
                "Changing direction of transceiver from {previous_direction} to {current_direction}"
            );
            self.peer_connection.on_transceiver_direction_changed();
        }
    }

    /// Returns the negotiated direction of the transceiver.
    ///
    /// This indicates the current direction as established by the most recent session description
    /// exchange. If this transceiver has never been negotiated or if it's stopped, this returns
    /// [`RTCRtpTransceiverDirection::Unspecified`].
    ///
    /// # Specification
    ///
    /// See [RTCRtpTransceiver.currentDirection](https://www.w3.org/TR/webrtc/#dom-rtcrtptransceiver-currentdirection).
    pub fn current_direction(&self) -> RTCRtpTransceiverDirection {
        // peer_connection is mutable borrow, its rtp_transceivers won't be resized,
        // so, [self.index] here is safe.
        self.peer_connection.rtp_transceivers[self.index].current_direction()
    }

    /// Irreversibly stops the transceiver.
    ///
    /// After calling this method, the transceiver will no longer send or receive media.
    /// This operation cannot be undone.
    ///
    /// # Errors
    ///
    /// Returns an error if the sender's interceptor bindings cannot be torn down.
    ///
    /// # Specification
    ///
    /// See [RTCRtpTransceiver.stop()](https://www.w3.org/TR/webrtc/#dom-rtcrtptransceiver-stop).
    pub fn stop(&mut self) -> Result<()> {
        // peer_connection is mutable borrow, its rtp_transceivers won't be resized,
        // so, [self.index] here is safe.
        self.peer_connection.stop_transceiver(self.index)
    }

    /// Sets the preferred codec list for this transceiver.
    ///
    /// This overrides the default codec preferences from the media engine. If an empty list is
    /// provided, the transceiver resets to use the default codecs from the media engine.
    ///
    /// # Errors
    ///
    /// Returns an error if any codec in the list is not supported by the media engine.
    ///
    /// # Specification
    ///
    /// See [RTCRtpTransceiver.setCodecPreferences()](https://www.w3.org/TR/webrtc/#dom-rtcrtptransceiver-setcodecpreferences).
    pub fn set_codec_preferences(&mut self, codecs: Vec<RTCRtpCodecParameters>) -> Result<()> {
        // peer_connection is mutable borrow, its rtp_transceivers won't be resized,
        // so, [self.index] here is safe.
        self.peer_connection.rtp_transceivers[self.index]
            .set_codec_preferences(codecs, &self.peer_connection.media_engine)
    }
}

#[cfg(test)]
mod tests {
    use sansio::Protocol;
    use std::time::Instant;

    use crate::peer_connection::configuration::media_engine::MediaEngine;
    use crate::peer_connection::event::RTCPeerConnectionEvent;
    use crate::peer_connection::{RTCPeerConnection, RTCPeerConnectionBuilder};
    use crate::rtp_transceiver::RTCRtpTransceiverDirection;
    use crate::rtp_transceiver::rtp_sender::RtpCodecKind;

    fn build_pc() -> RTCPeerConnection {
        let mut media_engine = MediaEngine::default();
        media_engine
            .register_default_codecs()
            .expect("register default codecs");
        RTCPeerConnectionBuilder::new()
            .with_media_engine(media_engine)
            .build(Instant::now())
            .expect("build peer connection")
    }

    /// Drain every queued event and count the `OnNegotiationNeededEvent`s, driving only the
    /// public sans-I/O `Protocol` API (no private negotiation state is touched).
    fn count_negotiation_needed(pc: &mut RTCPeerConnection) -> usize {
        let mut count = 0;
        while let Some(event) = pc.poll_event() {
            if matches!(event, RTCPeerConnectionEvent::OnNegotiationNeededEvent) {
                count += 1;
            }
        }
        count
    }

    /// Complete a full offer/answer exchange so `offerer` settles back into a stable, idle
    /// negotiation state.
    fn negotiate(offerer: &mut RTCPeerConnection, answerer: &mut RTCPeerConnection) {
        let offer = offerer.create_offer(None).expect("create offer");
        offerer
            .set_local_description(Instant::now(), offer.clone())
            .expect("offerer set local");
        answerer
            .set_remote_description(Instant::now(), offer)
            .expect("answerer set remote");
        let answer = answerer.create_answer(None).expect("create answer");
        answerer
            .set_local_description(Instant::now(), answer.clone())
            .expect("answerer set local");
        offerer
            .set_remote_description(Instant::now(), answer)
            .expect("offerer set remote");
    }

    #[test]
    fn test_set_direction_triggers_negotiation_on_change() {
        let mut offerer = build_pc();
        let mut answerer = build_pc();

        let tid = offerer
            .add_transceiver_from_kind(RtpCodecKind::Video, None)
            .expect("add offerer transceiver");
        answerer
            .add_transceiver_from_kind(RtpCodecKind::Video, None)
            .expect("add answerer transceiver");

        negotiate(&mut offerer, &mut answerer);
        // Discard events produced by the initial negotiation.
        let _ = count_negotiation_needed(&mut offerer);

        // A transceiver added with `add_transceiver_from_kind(.., None)` negotiates as recvonly.
        assert_eq!(
            offerer.rtp_transceivers[tid].direction(),
            RTCRtpTransceiverDirection::Recvonly,
        );
        offerer
            .rtp_transceiver(tid)
            .expect("transceiver exists")
            .set_direction(RTCRtpTransceiverDirection::Inactive);

        assert_eq!(
            count_negotiation_needed(&mut offerer),
            1,
            "changing the transceiver direction must trigger negotiation-needed",
        );
    }

    #[test]
    fn test_set_direction_noop_when_unchanged() {
        let mut offerer = build_pc();
        let mut answerer = build_pc();

        let tid = offerer
            .add_transceiver_from_kind(RtpCodecKind::Video, None)
            .expect("add offerer transceiver");
        answerer
            .add_transceiver_from_kind(RtpCodecKind::Video, None)
            .expect("add answerer transceiver");

        negotiate(&mut offerer, &mut answerer);
        let _ = count_negotiation_needed(&mut offerer);

        // Setting the current direction again must be a no-op.
        let current = offerer.rtp_transceivers[tid].direction();
        offerer
            .rtp_transceiver(tid)
            .expect("transceiver exists")
            .set_direction(current);

        assert_eq!(
            count_negotiation_needed(&mut offerer),
            0,
            "setting the same direction must not trigger negotiation-needed",
        );
    }

    /// A VP8 track sending on `ssrc`, as zenoh-gateway makes one per subscription.
    fn video_track(name: &str, ssrc: u32) -> crate::media_stream::MediaStreamTrack {
        use crate::rtp_transceiver::rtp_sender::{
            RTCRtpCodec, RTCRtpCodingParameters, RTCRtpEncodingParameters,
        };
        crate::media_stream::MediaStreamTrack::new(
            format!("stream-{name}"),
            format!("track-{name}"),
            format!("label-{name}"),
            RtpCodecKind::Video,
            vec![RTCRtpEncodingParameters {
                rtp_coding_parameters: RTCRtpCodingParameters {
                    ssrc: Some(ssrc),
                    ..Default::default()
                },
                codec: RTCRtpCodec {
                    mime_type: "video/VP8".to_owned(),
                    clock_rate: 90000,
                    ..Default::default()
                },
                ..Default::default()
            }],
        )
    }

    /// A local SRTP context holding state for `ssrc`, as one would after sending on it.
    fn srtp_context_with(ssrc: u32) -> srtp::context::Context {
        use shared::marshal::Marshal;
        let profile = srtp::protection_profile::ProtectionProfile::Aes128CmHmacSha1_80;
        let provider = crypto::default_provider().expect("a built-in crypto provider");
        let mut context = srtp::context::Context::new(
            &vec![0; profile.key_len()],
            &vec![0; profile.salt_len()],
            profile,
            None,
            None,
            provider.crypto(),
        )
        .expect("srtp context");
        let packet = rtp::Packet {
            header: rtp::Header {
                version: 2,
                ssrc,
                ..Default::default()
            },
            payload: bytes::Bytes::from_static(b"frame"),
        };
        context
            .encrypt_rtp(&packet.marshal().expect("marshal"))
            .expect("encrypt");
        context
    }

    /// zenoh-web patch: zenoh-gateway's pattern over one long-lived connection. Each round the
    /// browser adds a recvonly transceiver and offers, the gateway adds a track and answers;
    /// then the browser stops the transceiver and offers again (its m-line rejected). Once that
    /// negotiation completes, both sides must have removed the stopped transceiver, and the
    /// gateway the stats and SRTP state of the stream it sent.
    #[test]
    fn stopped_transceivers_are_removed_once_negotiation_completes() {
        let mut browser = build_pc();
        let mut gateway = build_pc();

        for round in 0..10 {
            let browser_tid = browser
                .add_transceiver_from_kind(RtpCodecKind::Video, None)
                .expect("add browser transceiver");
            let sender_id = gateway
                .add_track(video_track(&format!("{round}"), 1000 + round))
                .expect("add track");
            negotiate(&mut browser, &mut gateway);

            let ssrc = gateway
                .rtp_sender(sender_id)
                .expect("the subscription's sender")
                .track()
                .ssrcs()
                .next()
                .expect("an ssrc");
            assert_eq!(gateway.pipeline_context.stats.outbound_rtp_streams.len(), 1);
            gateway.pipeline_context.srtp_handler_context.local_srtp_context =
                Some(srtp_context_with(ssrc));

            browser
                .rtp_transceiver(browser_tid)
                .expect("browser transceiver")
                .stop()
                .expect("stop");
            negotiate(&mut browser, &mut gateway);

            assert_eq!(browser.rtp_transceivers.len(), 0, "round {round}: browser");
            assert_eq!(gateway.rtp_transceivers.len(), 0, "round {round}: gateway");
            assert!(gateway.rtp_sender(sender_id).is_none(), "a removed sender is gone");
            assert!(
                gateway.pipeline_context.stats.outbound_rtp_streams.is_empty(),
                "round {round}: the stopped sender's stats must go"
            );
            let srtp = gateway
                .pipeline_context
                .srtp_handler_context
                .local_srtp_context
                .as_ref()
                .unwrap();
            assert_eq!(
                srtp.ssrc_state_counts(),
                (0, 0),
                "round {round}: the stopped sender's SRTP state must go"
            );
        }
    }

    /// zenoh-web patch: the rejected m-line of a removed transceiver keeps its place in later
    /// descriptions, and the next new transceiver recycles it (RFC 9429 5.2.2) under a new mid
    /// rather than growing the description.
    #[test]
    fn a_removed_transceivers_m_line_is_recycled() {
        let mut offerer = build_pc();
        let mut answerer = build_pc();

        let first = offerer
            .add_transceiver_from_kind(RtpCodecKind::Video, None)
            .expect("add");
        offerer
            .add_transceiver_from_kind(RtpCodecKind::Video, None)
            .expect("add");
        negotiate(&mut offerer, &mut answerer);

        offerer.rtp_transceiver(first).expect("first").stop().expect("stop");
        negotiate(&mut offerer, &mut answerer);
        assert_eq!(offerer.rtp_transceivers.len(), 1);
        assert_eq!(answerer.rtp_transceivers.len(), 1);

        // the answerer offers now: the removed transceiver's m-line stays, rejected
        let offer = answerer.create_offer(None).expect("offer");
        let media = &offer.parsed.as_ref().unwrap().media_descriptions;
        assert_eq!(media.len(), 2);
        assert_eq!(media[0].media_name.port.value, 0, "still rejected, in its place");
        answerer
            .set_local_description(Instant::now(), offer.clone())
            .expect("set local");
        offerer
            .set_remote_description(Instant::now(), offer)
            .expect("set remote");
        let answer = offerer.create_answer(None).expect("answer");
        offerer
            .set_local_description(Instant::now(), answer.clone())
            .expect("set local");
        answerer
            .set_remote_description(Instant::now(), answer)
            .expect("set remote");

        // a new transceiver takes the rejected slot
        offerer
            .add_transceiver_from_kind(RtpCodecKind::Video, None)
            .expect("add");
        let offer = offerer.create_offer(None).expect("offer");
        let media = &offer.parsed.as_ref().unwrap().media_descriptions;
        assert_eq!(media.len(), 2, "the rejected m-line is recycled, not appended to");
        assert_ne!(media[0].media_name.port.value, 0);
        let mids: Vec<_> = media
            .iter()
            .map(|m| crate::peer_connection::sdp::get_mid_value(m).unwrap().clone())
            .collect();
        assert_ne!(mids[0], mids[1]);
        assert_ne!(mids[0], "0", "a recycled m-line takes a new mid");
        negotiate_with(&mut offerer, &mut answerer, offer);
        assert_eq!(offerer.rtp_transceivers.len(), 2);
        assert_eq!(answerer.rtp_transceivers.len(), 2);
    }

    fn negotiate_with(
        offerer: &mut RTCPeerConnection,
        answerer: &mut RTCPeerConnection,
        offer: crate::peer_connection::sdp::RTCSessionDescription,
    ) {
        offerer
            .set_local_description(Instant::now(), offer.clone())
            .expect("offerer set local");
        answerer
            .set_remote_description(Instant::now(), offer)
            .expect("answerer set remote");
        let answer = answerer.create_answer(None).expect("create answer");
        answerer
            .set_local_description(Instant::now(), answer.clone())
            .expect("answerer set local");
        offerer
            .set_remote_description(Instant::now(), answer)
            .expect("offerer set remote");
    }
}
