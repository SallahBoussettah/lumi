import { invoke } from "@tauri-apps/api/core";

export async function listAudioDevices(): Promise<string[]> {
  return invoke("list_audio_devices");
}

export async function startRecording(): Promise<string> {
  return invoke("start_recording");
}

export async function stopRecording(): Promise<string> {
  return invoke("stop_recording");
}

export async function getAudioLevel(): Promise<number> {
  return invoke("get_audio_level");
}

export async function isRecording(): Promise<boolean> {
  return invoke("is_recording");
}

export async function getDbStats(): Promise<{
  conversations: number;
  memories: number;
  action_items: number;
  screenshots: number;
}> {
  return invoke("get_db_stats");
}
