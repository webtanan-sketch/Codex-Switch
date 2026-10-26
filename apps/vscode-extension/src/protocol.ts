import {createHash} from "node:crypto";
export interface ProfileEvent { version: 1; type: "STATUS" | "PROFILE_CHANGED"; homeKey: string; profileId: string | null; profileName: string | null; timestamp: number }
const fields = new Set(["version","type","homeKey","profileId","profileName","timestamp"]);
export function parseEvent(line: string): ProfileEvent | undefined {
  if(Buffer.byteLength(line,"utf8") > 4096) return;
  let value: unknown;
  try { value = JSON.parse(line); } catch { return; }
  if(!value || typeof value !== "object" || Array.isArray(value)) return;
  const event = value as Record<string,unknown>;
  if(Object.keys(event).length !== fields.size || Object.keys(event).some(k => !fields.has(k))) return;
  if(event.version !== 1 || !["STATUS","PROFILE_CHANGED"].includes(String(event.type))) return;
  if(typeof event.homeKey !== "string" || !/^[a-f0-9]{64}$/.test(event.homeKey)) return;
  if(event.profileId !== null && (typeof event.profileId !== "string" || !/^[a-f0-9-]{36}$/.test(event.profileId))) return;
  if(event.profileName !== null && (typeof event.profileName !== "string" || event.profileName.length > 160 || /[\x00-\x1f\x7f]/.test(event.profileName))) return;
  if(typeof event.timestamp !== "number" || !Number.isSafeInteger(event.timestamp) || event.timestamp < 0) return;
  return event as unknown as ProfileEvent;
}
export function homeKey(path: string): string {
  return createHash("sha256").update(path.replace(/\//g,"\\").replace(/\\+$/,"").toLowerCase()).digest("hex");
}
export class LineDecoder {
  private pending = "";
  push(chunk: string): ProfileEvent[] {
    this.pending += chunk;
    if(Buffer.byteLength(this.pending,"utf8") > 32768) { this.pending = ""; throw new Error("IPC_FRAME_LIMIT"); }
    const lines = this.pending.split("\n"); this.pending = lines.pop() ?? "";
    return lines.map(parseEvent).filter((e): e is ProfileEvent => Boolean(e));
  }
}
