import type {Language} from "./i18n";

export function remainingPercent(used: number): number {
  return Math.max(0, Math.min(100, 100 - used));
}

export function windowLabel(minutes: number | null, kind: string, language: Language): string {
  const fa = language === "fa";
  const number = (value: number) => new Intl.NumberFormat(fa ? "fa-IR" : "en-US").format(value);
  if (minutes === 10080) return fa ? "محدودیت هفتگی" : "Weekly limit";
  if (minutes == null) return kind === "primary"
    ? (fa ? "بازهٔ کوتاه‌مدت" : "Short-term window")
    : (fa ? "بازهٔ بلندمدت" : "Long-term window");
  if (minutes % 1440 === 0) return fa ? `بازهٔ ${number(minutes / 1440)} روزه` : `${number(minutes / 1440)}-day window`;
  if (minutes % 60 === 0) return fa ? `بازهٔ ${number(minutes / 60)} ساعته` : `${number(minutes / 60)}-hour window`;
  return fa ? `بازهٔ ${number(minutes)} دقیقه‌ای` : `${number(minutes)}-minute window`;
}

export function resetCountdown(resetsAt: number | null, now: number, language: Language): string {
  const fa = language === "fa";
  if (resetsAt == null) return fa ? "زمان بازنشانی اعلام نشده" : "Reset time not reported";
  let seconds = Math.ceil(resetsAt - now);
  if (seconds <= 0) return fa ? "در انتظار تازه‌سازی از Codex" : "Waiting for a fresh Codex update";
  const parts: string[] = [];
  for (const [size, persian, english] of [[86400,"روز","d"],[3600,"ساعت","h"],[60,"دقیقه","m"],[1,"ثانیه","s"]] as const) {
    const count = Math.floor(seconds / size);
    seconds %= size;
    if (count) parts.push(`${new Intl.NumberFormat(fa ? "fa-IR" : "en-US").format(count)} ${fa ? persian : english}`);
    if (parts.length === 2) break;
  }
  return parts.join(fa ? " و " : " ");
}
