//! Phase 2 — always-on listening.
//!
//! Instead of a hotkey gating each recording, always-on mode keeps the mic
//! open continuously and uses **voice-activity detection (VAD)** to carve the
//! audio stream into utterances: an utterance opens when energy rises above an
//! onset threshold and closes after a configurable run of silence (the
//! "hangover"). Each completed utterance is fed to the same STT → delivery
//! pipeline the hotkey path already uses (`server::process_recording_bg`).
//!
//! This module owns the **algorithmic core** — a pure [`VadSegmenter`] state
//! machine over a stream of per-frame RMS energies. It is deliberately free of
//! any audio-backend dependency so it can be unit-tested deterministically
//! (mic hardware is never reliable in CI). The continuous capture loop that
//! feeds real frames into the segmenter is wired in [`start_if_enabled`];
//! see the TODO there for the remaining cpal streaming work.
//!
//! Privacy: always-on is **opt-in** (`config.voice_server.always_on_enabled`,
//! default false) and pauses when the screen is locked (Phase 2 privacy hook).

use crate::openhuman::config::VoiceServerConfig as CfgVoiceServer;

const LOG_PREFIX: &str = "[voice::always_on]";

/// Tuning for the VAD segmenter, distilled from [`CfgVoiceServer`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VadConfig {
    /// Peak-RMS energy above which a frame counts as speech.
    pub onset_threshold: f32,
    /// How long energy must stay below `onset_threshold` before the current
    /// utterance is closed. Bridges natural mid-sentence pauses.
    pub hangover_ms: u32,
    /// Minimum voiced duration for a segment to be emitted; shorter blips
    /// (cough, door) are dropped.
    pub min_speech_ms: u32,
    /// Hard ceiling on a single utterance — forces a flush so a continuous
    /// noise source can't grow an unbounded recording.
    pub max_utterance_ms: u32,
}

impl VadConfig {
    /// Build VAD tuning from the persisted voice-server config.
    pub fn from_server_config(c: &CfgVoiceServer) -> Self {
        Self {
            onset_threshold: c.vad_onset_threshold,
            hangover_ms: c.vad_hangover_ms,
            min_speech_ms: c.vad_min_speech_ms,
            max_utterance_ms: (c.vad_max_utterance_secs * 1000.0).round().max(1.0) as u32,
        }
    }
}

/// An event emitted by the segmenter as the audio stream is consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadEvent {
    /// Energy crossed the onset threshold — an utterance has begun.
    SpeechStart,
    /// An utterance closed. `voiced_ms` is the accumulated speech duration
    /// (excluding the trailing silence); `emit` is false when it fell below
    /// `min_speech_ms` (drop it); `forced` is true when the close was caused
    /// by the `max_utterance_ms` ceiling rather than a silence hangover.
    SpeechEnd {
        voiced_ms: u32,
        emit: bool,
        forced: bool,
    },
}

#[derive(Debug, Clone, Copy)]
enum State {
    /// No active utterance — waiting for energy to cross the onset threshold.
    Silent,
    /// Inside an utterance.
    Speaking {
        /// Total elapsed time since the utterance opened (voiced + silence).
        total_ms: u32,
        /// Accumulated voiced time (frames above onset).
        voiced_ms: u32,
        /// Consecutive below-onset time since the last voiced frame.
        silence_run_ms: u32,
    },
}

/// Pure VAD state machine. Drive it by calling [`push_frame`](Self::push_frame)
/// with the RMS energy of each fixed-size audio frame; it returns at most one
/// [`VadEvent`] per frame.
#[derive(Debug)]
pub struct VadSegmenter {
    cfg: VadConfig,
    state: State,
}

impl VadSegmenter {
    pub fn new(cfg: VadConfig) -> Self {
        Self {
            cfg,
            state: State::Silent,
        }
    }

    /// True while inside an utterance (between `SpeechStart` and `SpeechEnd`).
    pub fn is_speaking(&self) -> bool {
        matches!(self.state, State::Speaking { .. })
    }

    /// Abort any in-flight utterance and return to the idle state without
    /// emitting an event. Used by the privacy hook (screen lock) and on
    /// stream teardown.
    pub fn reset(&mut self) {
        self.state = State::Silent;
    }

    /// Feed one frame's RMS energy and its duration in milliseconds.
    pub fn push_frame(&mut self, rms: f32, frame_ms: u32) -> Option<VadEvent> {
        let above = rms >= self.cfg.onset_threshold;
        match self.state {
            State::Silent => {
                if above {
                    self.state = State::Speaking {
                        total_ms: frame_ms,
                        voiced_ms: frame_ms,
                        silence_run_ms: 0,
                    };
                    Some(VadEvent::SpeechStart)
                } else {
                    None
                }
            }
            State::Speaking {
                mut total_ms,
                mut voiced_ms,
                mut silence_run_ms,
            } => {
                total_ms = total_ms.saturating_add(frame_ms);
                if above {
                    voiced_ms = voiced_ms.saturating_add(frame_ms);
                    silence_run_ms = 0;
                } else {
                    silence_run_ms = silence_run_ms.saturating_add(frame_ms);
                }

                // Close on a silence hangover.
                if silence_run_ms >= self.cfg.hangover_ms {
                    self.state = State::Silent;
                    let emit = voiced_ms >= self.cfg.min_speech_ms;
                    return Some(VadEvent::SpeechEnd {
                        voiced_ms,
                        emit,
                        forced: false,
                    });
                }
                // Close on the hard utterance ceiling.
                if total_ms >= self.cfg.max_utterance_ms {
                    self.state = State::Silent;
                    let emit = voiced_ms >= self.cfg.min_speech_ms;
                    return Some(VadEvent::SpeechEnd {
                        voiced_ms,
                        emit,
                        forced: true,
                    });
                }

                self.state = State::Speaking {
                    total_ms,
                    voiced_ms,
                    silence_run_ms,
                };
                None
            }
        }
    }
}

/// Start the always-on capture loop if `always_on_enabled` is set in config.
///
/// No-op when disabled (the common, privacy-preserving default).
///
/// TODO(phase-2): open a continuous cpal input stream, downmix to 16 kHz mono,
/// slice into fixed frames (e.g. 20 ms), feed each frame's RMS to a
/// [`VadSegmenter`], buffer samples between `SpeechStart` and an emitted
/// `SpeechEnd`, then hand the buffered WAV to the existing
/// `server::process_recording_bg` STT→delivery pipeline. Pause the segmenter
/// (`reset`) when the screen locks. The segmenter below is already complete and
/// unit-tested; this is the remaining audio-plumbing layer.
pub async fn start_if_enabled(app_config: &crate::openhuman::config::Config) {
    if !app_config.voice_server.always_on_enabled {
        log::info!("{LOG_PREFIX} disabled in config; not opening continuous mic");
        return;
    }
    let cfg = VadConfig::from_server_config(&app_config.voice_server);
    log::info!(
        "{LOG_PREFIX} enabled — onset={:.4} hangover={}ms min_speech={}ms max_utt={}ms \
         (continuous capture loop not yet wired; see TODO)",
        cfg.onset_threshold,
        cfg.hangover_ms,
        cfg.min_speech_ms,
        cfg.max_utterance_ms
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> VadConfig {
        VadConfig {
            onset_threshold: 0.01,
            hangover_ms: 100,
            min_speech_ms: 60,
            max_utterance_ms: 1000,
        }
    }

    /// Drive `n` frames of constant `rms` at `frame_ms` each, collecting events.
    fn drive(seg: &mut VadSegmenter, rms: f32, frame_ms: u32, n: u32) -> Vec<VadEvent> {
        (0..n)
            .filter_map(|_| seg.push_frame(rms, frame_ms))
            .collect()
    }

    #[test]
    fn silence_emits_nothing() {
        let mut seg = VadSegmenter::new(cfg());
        assert!(drive(&mut seg, 0.0, 20, 50).is_empty());
        assert!(!seg.is_speaking());
    }

    #[test]
    fn onset_then_hangover_emits_one_utterance() {
        let mut seg = VadSegmenter::new(cfg());
        // First loud frame opens the utterance.
        assert_eq!(seg.push_frame(0.2, 20), Some(VadEvent::SpeechStart));
        assert!(seg.is_speaking());
        // More speech, no event yet.
        assert!(drive(&mut seg, 0.2, 20, 5).is_empty());
        // Silence shorter than hangover: still open.
        assert!(seg.push_frame(0.0, 20).is_none()); // 20ms silence
        assert!(seg.push_frame(0.0, 20).is_none()); // 40ms
        assert!(seg.push_frame(0.0, 20).is_none()); // 60ms
        assert!(seg.push_frame(0.0, 20).is_none()); // 80ms
                                                    // Crossing the 100ms hangover closes it.
        let ev = seg.push_frame(0.0, 20).unwrap(); // 100ms
        match ev {
            VadEvent::SpeechEnd { emit, forced, .. } => {
                assert!(emit, "120ms voiced should clear the 60ms min");
                assert!(!forced);
            }
            other => panic!("expected SpeechEnd, got {other:?}"),
        }
        assert!(!seg.is_speaking());
    }

    #[test]
    fn short_blip_is_dropped() {
        let mut seg = VadSegmenter::new(cfg());
        // One 20ms loud frame (below the 60ms min), then silence to close.
        assert_eq!(seg.push_frame(0.2, 20), Some(VadEvent::SpeechStart));
        let mut ev = None;
        for _ in 0..5 {
            if let Some(e) = seg.push_frame(0.0, 20) {
                ev = Some(e);
                break;
            }
        }
        match ev.expect("utterance should close") {
            VadEvent::SpeechEnd {
                voiced_ms, emit, ..
            } => {
                assert_eq!(voiced_ms, 20);
                assert!(!emit, "20ms < 60ms min_speech ⇒ dropped");
            }
            other => panic!("expected SpeechEnd, got {other:?}"),
        }
    }

    #[test]
    fn mid_utterance_pause_does_not_split() {
        let mut seg = VadSegmenter::new(cfg());
        seg.push_frame(0.2, 20);
        // 80ms pause (< 100ms hangover) then speech resumes — one utterance.
        for _ in 0..4 {
            assert!(seg.push_frame(0.0, 20).is_none());
        }
        assert!(
            seg.is_speaking(),
            "pause under hangover keeps utterance open"
        );
        assert!(drive(&mut seg, 0.2, 20, 3).is_empty());
        assert!(seg.is_speaking());
    }

    #[test]
    fn max_utterance_forces_flush() {
        let mut seg = VadSegmenter::new(cfg()); // max 1000ms
        seg.push_frame(0.2, 20);
        // Keep talking past the ceiling; silence never triggers the close.
        let mut forced_seen = false;
        for _ in 0..60 {
            if let Some(VadEvent::SpeechEnd { forced, emit, .. }) = seg.push_frame(0.2, 20) {
                assert!(forced, "loud-throughout close must be the ceiling");
                assert!(emit);
                forced_seen = true;
                break;
            }
        }
        assert!(forced_seen, "should force-flush at max_utterance_ms");
        assert!(!seg.is_speaking());
    }

    #[test]
    fn reset_aborts_without_event() {
        let mut seg = VadSegmenter::new(cfg());
        seg.push_frame(0.2, 20);
        assert!(seg.is_speaking());
        seg.reset();
        assert!(!seg.is_speaking());
        // After reset, a fresh onset starts a new utterance.
        assert_eq!(seg.push_frame(0.2, 20), Some(VadEvent::SpeechStart));
    }

    #[test]
    fn from_server_config_maps_seconds_to_ms() {
        let mut c = CfgVoiceServer::default();
        c.vad_max_utterance_secs = 2.5;
        c.vad_hangover_ms = 750;
        let v = VadConfig::from_server_config(&c);
        assert_eq!(v.max_utterance_ms, 2500);
        assert_eq!(v.hangover_ms, 750);
        assert_eq!(v.onset_threshold, c.vad_onset_threshold);
    }
}
