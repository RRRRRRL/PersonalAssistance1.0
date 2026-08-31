<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";

  type Props = {
    onTranscription?: (text: string) => void;
    onSpeaking?: (speaking: boolean) => void;
    speak?: { text: string } | null;
  };

  let { onTranscription = () => {}, onSpeaking = () => {}, speak = null }: Props = $props();

  // ─── State ───────────────────────────────────────────────────────────────
  type VoicePhase = "idle" | "listening" | "processing" | "speaking" | "error";
  let phase = $state<VoicePhase>("idle");
  let transcript = $state("");
  let errorMessage = $state("");
  let audioLevel = $state(0);

  let mediaRecorder: MediaRecorder | null = null;
  let audioChunks: Blob[] = [];
  let audioContext: AudioContext | null = null;
  let analyser: AnalyserNode | null = null;
  let animFrameId: number | null = null;
  let streamRef: MediaStream | null = null;

  // ─── Voice settings (loaded on mount) ──────────────────────────────────────
  interface VoiceStatus {
    stt_ready: boolean;
    tts_ready: boolean;
    settings: { tts_voice: string; stt_model: string; tts_model: string };
  }
  let voiceStatus = $state<VoiceStatus | null>(null);

  onMount(async () => {
    try {
      voiceStatus = await invoke<VoiceStatus>("get_voice_settings");
    } catch (e) {
      console.error("Failed to load voice settings", e);
    }
  });

  onDestroy(() => {
    stopAudioVisualiser();
    streamRef?.getTracks().forEach((t) => t.stop());
  });

  // ─── Watch for speak prop changes to trigger TTS ────────────────────────────
  $effect(() => {
    if (speak && speak.text.trim()) {
      speakText(speak.text);
    }
  });

  // ─── Audio visualiser (mic level) ─────────────────────────────────────────
  function startAudioVisualiser(stream: MediaStream) {
    audioContext = new AudioContext();
    analyser = audioContext.createAnalyser();
    analyser.fftSize = 256;
    const source = audioContext.createMediaStreamSource(stream);
    source.connect(analyser);

    const dataArray = new Uint8Array(analyser.frequencyBinCount);
    function tick() {
      if (!analyser) return;
      analyser.getByteFrequencyData(dataArray);
      const avg = dataArray.reduce((a, b) => a + b, 0) / dataArray.length;
      audioLevel = Math.min(avg / 128, 1);
      animFrameId = requestAnimationFrame(tick);
    }
    tick();
  }

  function stopAudioVisualiser() {
    if (animFrameId !== null) {
      cancelAnimationFrame(animFrameId);
      animFrameId = null;
    }
    audioContext?.close().catch(() => {});
    audioContext = null;
    analyser = null;
    audioLevel = 0;
  }

  // ─── Recording ─────────────────────────────────────────────────────────────
  async function startListening() {
    if (phase !== "idle") return;
    transcript = "";
    errorMessage = "";
    audioChunks = [];

    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      streamRef = stream;
      startAudioVisualiser(stream);

      const mimeOptions = ["audio/webm;codecs=opus", "audio/webm", "audio/ogg;codecs=opus", "audio/mp4"];
      const mimeType = mimeOptions.find((m) => MediaRecorder.isTypeSupported(m)) ?? "";

      mediaRecorder = new MediaRecorder(stream, mimeType ? { mimeType } : undefined);
      mediaRecorder.ondataavailable = (e) => {
        if (e.data.size > 0) audioChunks.push(e.data);
      };
      mediaRecorder.onstop = handleRecordingStop;
      mediaRecorder.start();
      phase = "listening";
    } catch (e) {
      errorMessage = `Microphone access denied: ${String(e)}`;
      phase = "error";
    }
  }

  function stopListening() {
    if (phase !== "listening" || !mediaRecorder) return;
    mediaRecorder.stop();
    streamRef?.getTracks().forEach((t) => t.stop());
    streamRef = null;
    stopAudioVisualiser();
    phase = "processing";
  }

  async function handleRecordingStop() {
    if (audioChunks.length === 0) {
      errorMessage = "No audio recorded";
      phase = "error";
      return;
    }

    const blob = new Blob(audioChunks, { type: audioChunks[0]?.type || "audio/webm" });
    const buffer = await blob.arrayBuffer();
    const base64Audio = arrayBufferToBase64(buffer);

    try {
      const result = await invoke<{ text: string; language?: string }>("transcribe_audio", {
        audioBase64: base64Audio,
        mimeType: blob.type || "audio/webm",
      });

      transcript = result.text;
      if (result.text.trim()) {
        onTranscription(result.text);
      }
      phase = "idle";
    } catch (e) {
      errorMessage = `Transcription failed: ${String(e)}`;
      phase = "error";
    }
  }

  // ─── TTS playback ──────────────────────────────────────────────────────────
  async function speakText(text: string) {
    if (!text.trim()) return;
    phase = "processing";
    onSpeaking(true);

    try {
      const audioBase64 = await invoke<string>("synthesize_speech", { text });
      const audioBytes = base64ToArrayBuffer(audioBase64);
      const audioBlob = new Blob([audioBytes], { type: "audio/mpeg" });
      const url = URL.createObjectURL(audioBlob);
      const audio = new Audio(url);

      audio.onended = () => {
        URL.revokeObjectURL(url);
        phase = "idle";
        onSpeaking(false);
      };
      audio.onerror = () => {
        URL.revokeObjectURL(url);
        errorMessage = "Audio playback failed";
        phase = "error";
        onSpeaking(false);
      };

      phase = "speaking";
      await audio.play();
    } catch (e) {
      errorMessage = `TTS failed: ${String(e)}`;
      phase = "error";
      onSpeaking(false);
    }
  }

  // ─── Helpers ───────────────────────────────────────────────────────────────
  function arrayBufferToBase64(buffer: ArrayBuffer): string {
    const bytes = new Uint8Array(buffer);
    let binary = "";
    for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
    return btoa(binary);
  }

  function base64ToArrayBuffer(base64: string): ArrayBuffer {
    const binary = atob(base64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
    return bytes.buffer;
  }

  function resetError() {
    errorMessage = "";
    phase = "idle";
  }

  // ─── Derived ──────────────────────────────────────────────────────────────
  let orbScale = $derived(phase === "listening" ? 1 + audioLevel * 0.4 : phase === "speaking" ? 1.15 : 1);
  let orbColor = $derived(
    phase === "listening" ? "bg-rose-400" :
    phase === "processing" ? "bg-amber-400" :
    phase === "speaking" ? "bg-emerald-400" :
    phase === "error" ? "bg-red-400" : "bg-cyan-400"
  );
  let phaseLabel = $derived(
    phase === "listening" ? "Listening..." :
    phase === "processing" ? "Processing..." :
    phase === "speaking" ? "Speaking..." :
    phase === "error" ? "Error" : "Tap to speak"
  );
</script>

<div class="flex flex-col items-center gap-4">
  <!-- Orb -->
  <button
    type="button"
    onclick={() => {
      if (phase === "idle") startListening();
      else if (phase === "listening") stopListening();
      else if (phase === "error") resetError();
    }}
    class="group relative flex items-center justify-center outline-none"
    title={phaseLabel}
  >
    <!-- Pulse rings when listening -->
    {#if phase === "listening"}
      <span class="absolute h-20 w-20 animate-ping rounded-full bg-rose-400/30"></span>
      <span class="absolute h-16 w-16 animate-pulse rounded-full bg-rose-400/20"></span>
    {/if}
    {#if phase === "speaking"}
      <span class="absolute h-20 w-20 animate-pulse rounded-full bg-emerald-400/20"></span>
    {/if}

    <div
      class="relative h-14 w-14 rounded-full shadow-lg shadow-black/40 transition-all duration-200 {orbColor} group-hover:brightness-110"
      style="transform: scale({orbScale})"
    >
      {#if phase === "processing"}
        <div class="absolute inset-0 flex items-center justify-center">
          <svg class="h-6 w-6 animate-spin text-slate-900" viewBox="0 0 24 24" fill="none">
            <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="3" class="opacity-25"/>
            <path d="M4 12a8 8 0 018-8" stroke="currentColor" stroke-width="3" stroke-linecap="round"/>
          </svg>
        </div>
      {:else if phase === "listening"}
        <div class="absolute inset-0 flex items-center justify-center">
          <svg class="h-6 w-6 text-slate-900" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 1a3 3 0 00-3 3v8a3 3 0 006 0V4a3 3 0 00-3-3z"/>
            <path d="M19 10v2a7 7 0 01-14 0v-2" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            <line x1="12" y1="19" x2="12" y2="23" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
        </div>
      {:else if phase === "speaking"}
        <div class="absolute inset-0 flex items-center justify-center gap-0.5">
          <span class="inline-block h-3 w-0.5 animate-bounce rounded bg-slate-900" style="animation-delay:0ms"></span>
          <span class="inline-block h-4 w-0.5 animate-bounce rounded bg-slate-900" style="animation-delay:100ms"></span>
          <span class="inline-block h-2 w-0.5 animate-bounce rounded bg-slate-900" style="animation-delay:200ms"></span>
          <span class="inline-block h-5 w-0.5 animate-bounce rounded bg-slate-900" style="animation-delay:300ms"></span>
          <span class="inline-block h-3 w-0.5 animate-bounce rounded bg-slate-900" style="animation-delay:400ms"></span>
        </div>
      {:else}
        <div class="absolute inset-0 flex items-center justify-center">
          <svg class="h-6 w-6 text-slate-900" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 1a3 3 0 00-3 3v8a3 3 0 006 0V4a3 3 0 00-3-3z"/>
            <path d="M19 10v2a7 7 0 01-14 0v-2" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            <line x1="12" y1="19" x2="12" y2="23" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
        </div>
      {/if}
    </div>
  </button>

  <!-- Phase label -->
  <p class="text-xs font-medium {phase === 'error' ? 'text-red-300' : 'text-white/60'}">{phaseLabel}</p>

  <!-- Transcript -->
  {#if transcript}
    <div class="w-full rounded-xl border border-white/10 bg-white/5 px-3 py-2 text-xs text-white/80">
      <p class="mb-1 text-[10px] font-semibold uppercase tracking-wider text-white/40">Transcript</p>
      <p>{transcript}</p>
    </div>
  {/if}

  <!-- Error -->
  {#if errorMessage}
    <div class="w-full rounded-xl border border-red-500/30 bg-red-950/40 px-3 py-2 text-xs text-red-300">
      {errorMessage}
    </div>
  {/if}
</div>
