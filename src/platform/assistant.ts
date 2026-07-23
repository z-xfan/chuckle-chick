import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type AssistantPayload =
  | { mode: "panel" }
  | {
      mode: "bubble";
      message: string;
      durationMs: number;
      placement: "above" | "below";
    };

export interface PetInteractionPayload {
  animationName: string;
}

export function toggleQuickPanel(): Promise<boolean> {
  return invoke<boolean>("toggle_quick_panel");
}

export function getAssistantPayload(): Promise<AssistantPayload | null> {
  return invoke<AssistantPayload | null>("get_assistant_payload");
}

export function showSpeechBubble(message: string, durationMs?: number): Promise<boolean> {
  return invoke<boolean>("show_speech_bubble", { message, durationMs });
}

export function hideAssistantWindow(): Promise<void> {
  return invoke("hide_assistant_window");
}

export function requestPetInteraction(animationName: string): Promise<void> {
  return invoke("request_pet_interaction", { animationName });
}

export function listenAssistantPayload(
  listener: (payload: AssistantPayload) => void,
): Promise<UnlistenFn> {
  return listen<AssistantPayload>("assistant-payload", ({ payload }) => listener(payload));
}

export function listenPetInteraction(
  listener: (payload: PetInteractionPayload) => void,
): Promise<UnlistenFn> {
  return listen<PetInteractionPayload>("pet-direct-interaction", ({ payload }) =>
    listener(payload),
  );
}
