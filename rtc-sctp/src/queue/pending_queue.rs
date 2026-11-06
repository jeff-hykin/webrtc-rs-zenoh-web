use crate::chunk::chunk_payload_data::ChunkPayloadData;

use std::collections::{BTreeMap, HashMap, VecDeque};

/// pendingBaseQueue
pub(crate) type PendingBaseQueue = VecDeque<ChunkPayloadData>;

/// A stream's priority until it is given one: the data channel's "normal" (RFC 8832 §5.1).
pub(crate) const DEFAULT_STREAM_PRIORITY: u16 = 256;

/// One priority's chunks: unordered ones go before ordered ones, as in upstream's single queue.
#[derive(Debug, Default)]
struct Lane {
    unordered_queue: PendingBaseQueue,
    ordered_queue: PendingBaseQueue,
}

impl Lane {
    fn front(&self) -> Option<&ChunkPayloadData> {
        self.unordered_queue.front().or_else(|| self.ordered_queue.front())
    }

    fn is_empty(&self) -> bool {
        self.unordered_queue.is_empty() && self.ordered_queue.is_empty()
    }
}

/// pendingQueue
///
/// zenoh-web patch: a strict priority scheduler across streams (RFC 8260 §3.4), the priority being the one the data
/// channel was opened with (RFC 8832's DATA_CHANNEL_OPEN, `RTCDataChannelInit.priority`): a higher priority's chunks go
/// first. Upstream's one queue (unordered first, then ordered) is each priority's lane, so with every stream at the
/// default it behaves as before. A message's fragments still go out back to back (`selected`): plain DATA chunks can't
/// interleave messages, so a higher priority's message waits at most for the one under way.
#[derive(Debug, Default)]
pub(crate) struct PendingQueue {
    /// priority -> its lane (only non-empty lanes are kept)
    lanes: BTreeMap<u16, Lane>,
    /// stream -> its priority, for streams not at `DEFAULT_STREAM_PRIORITY`
    priorities: HashMap<u16, u16>,
    queue_len: usize,
    n_bytes: usize,
    /// the message under way: (its lane's priority, whether it is unordered)
    selected: Option<(u16, bool)>,
}

impl PendingQueue {
    pub(crate) fn new() -> Self {
        PendingQueue::default()
    }

    fn priority_of(&self, stream_identifier: u16) -> u16 {
        self.priorities.get(&stream_identifier).copied().unwrap_or(DEFAULT_STREAM_PRIORITY)
    }

    pub(crate) fn push(&mut self, c: ChunkPayloadData) {
        self.n_bytes += c.user_data.len();
        let lane = self.lanes.entry(self.priority_of(c.stream_identifier)).or_default();
        if c.unordered {
            lane.unordered_queue.push_back(c);
        } else {
            lane.ordered_queue.push_back(c);
        }
        self.queue_len += 1;
    }

    /// Sets a stream's priority (higher goes first). Its chunks already queued move to the new priority's lane in order,
    /// so the stream's own messages keep their order; the rest of a message of it under way stays and finishes first (it
    /// goes next regardless, and split across lanes its fragments would interleave with others).
    pub(crate) fn set_stream_priority(&mut self, stream_identifier: u16, priority: u16) {
        let old = self.priority_of(stream_identifier);
        if priority == DEFAULT_STREAM_PRIORITY {
            self.priorities.remove(&stream_identifier);
        } else {
            self.priorities.insert(stream_identifier, priority);
        }
        if old == priority {
            return;
        }
        let selected = self.selected;
        let Some(lane) = self.lanes.get_mut(&old) else { return };
        let mut moved = Lane::default();
        for (unordered, from, to) in [(true, &mut lane.unordered_queue, &mut moved.unordered_queue), (false, &mut lane.ordered_queue, &mut moved.ordered_queue)] {
            // the fragments left of the message under way, when it is this queue's front
            let mut under_way = selected.is_some_and(|(lane, selected_unordered)| lane == old && selected_unordered == unordered);
            let mut rest = PendingBaseQueue::new();
            for c in from.drain(..) {
                if under_way {
                    under_way = !c.ending_fragment;
                    rest.push_back(c);
                } else if c.stream_identifier == stream_identifier {
                    to.push_back(c);
                } else {
                    rest.push_back(c);
                }
            }
            *from = rest;
        }
        if lane.is_empty() {
            self.lanes.remove(&old);
        }
        if !moved.is_empty() {
            let target = self.lanes.entry(priority).or_default();
            target.unordered_queue.extend(moved.unordered_queue);
            target.ordered_queue.extend(moved.ordered_queue);
        }
    }

    /// The lane to take from next: the message under way's, else the highest non-empty one.
    fn next_lane(&self) -> Option<u16> {
        match self.selected {
            Some((priority, _)) => Some(priority),
            None => self.lanes.keys().next_back().copied(),
        }
    }

    pub(crate) fn peek(&self) -> Option<&ChunkPayloadData> {
        let lane = self.lanes.get(&self.next_lane()?)?;
        match self.selected {
            Some((_, true)) => lane.unordered_queue.front(),
            Some((_, false)) => lane.ordered_queue.front(),
            None => lane.front(),
        }
    }

    pub(crate) fn pop(
        &mut self,
        beginning_fragment: bool,
        unordered: bool,
    ) -> Option<ChunkPayloadData> {
        let priority = self.next_lane()?;
        let selected = self.selected;
        if selected.is_none() && !beginning_fragment {
            return None;
        }
        let lane = self.lanes.get_mut(&priority)?;
        let from_unordered = selected.map_or(unordered, |(_, unordered)| unordered);
        let popped = if from_unordered { lane.unordered_queue.pop_front() } else { lane.ordered_queue.pop_front() };
        if lane.is_empty() {
            self.lanes.remove(&priority);
        }
        if let Some(p) = &popped {
            self.selected = (!p.ending_fragment).then_some((priority, from_unordered));
            self.n_bytes -= p.user_data.len();
            self.queue_len -= 1;
        }
        popped
    }

    /// zenoh-web patch: drop every not-yet-sent chunk of a stream the peer reset, so none of it
    /// gets a TSN after our answering reset and lands on a new channel that reuses the stream id.
    /// Returns the bytes dropped. The stream's priority goes with it (a new channel on the id sets its own).
    pub(crate) fn remove_stream(&mut self, stream_identifier: u16) -> usize {
        self.priorities.remove(&stream_identifier);
        if let Some((priority, unordered)) = self.selected
            && let Some(lane) = self.lanes.get(&priority)
        {
            let front = if unordered { lane.unordered_queue.front() } else { lane.ordered_queue.front() };
            if front.is_some_and(|c| c.stream_identifier == stream_identifier) {
                self.selected = None;
            }
        }
        let mut removed_bytes = 0;
        let mut removed_chunks = 0;
        for lane in self.lanes.values_mut() {
            for queue in [&mut lane.unordered_queue, &mut lane.ordered_queue] {
                queue.retain(|c| {
                    let keep = c.stream_identifier != stream_identifier;
                    if !keep {
                        removed_bytes += c.user_data.len();
                        removed_chunks += 1;
                    }
                    keep
                });
            }
        }
        self.lanes.retain(|_, lane| !lane.is_empty());
        self.n_bytes -= removed_bytes;
        self.queue_len -= removed_chunks;
        removed_bytes
    }

    pub(crate) fn get_num_bytes(&self) -> usize {
        self.n_bytes
    }

    pub(crate) fn len(&self) -> usize {
        self.queue_len
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
