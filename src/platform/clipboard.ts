import { writeText } from "@tauri-apps/plugin-clipboard-manager";

export function writePlainText(text: string): Promise<void> {
  return writeText(text);
}
