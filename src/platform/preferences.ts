import { invoke } from "@tauri-apps/api/core";

export interface AppPreferences {
  version: 2;
  scale: number;
  alwaysOnTop: boolean;
  currentPet: string;
}

export function getPreferences(): Promise<AppPreferences> {
  return invoke<AppPreferences>("get_preferences");
}

export function setPetAlwaysOnTop(enabled: boolean): Promise<void> {
  return invoke("set_pet_always_on_top", { enabled });
}

export function setPetScale(scale: number): Promise<void> {
  return invoke("set_pet_scale", { scale });
}

export function resetPetPosition(): Promise<void> {
  return invoke("reset_pet_position");
}

export function closeSettingsWindow(): Promise<void> {
  return invoke("close_settings_window");
}
