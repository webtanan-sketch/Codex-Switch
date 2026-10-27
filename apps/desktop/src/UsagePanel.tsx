import {useEffect, useState} from "react";
import {invoke} from "@tauri-apps/api/core";
import type {UsageSnapshot} from "@codex-manager/shared-types";
import type {Language} from "./i18n";
import {remainingPercent, resetCountdown, windowLabel} from "./usage-format";

export function UsagePanel({profileId, language, t}: {profileId: string; language: Language; t: (value: string) => string}) {
  const [usage, setUsage] = useState<UsageSnapshot>();
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const [refreshTick, setRefreshTick] = useState(0);
  const [now, setNow] = useState(() => Date.now() / 1000);
  useEffect(() => {
    let disposed = false;
    setLoading(true); setError("");
    void invoke<UsageSnapshot>("read_usage").then(value => {
      if (disposed) return;
      if (value.profileId !== profileId) { setError("حساب فعلی با حساب انتخاب‌شده هماهنگ نیست. حساب فعلی را دوباره وارد کنید."); return; }
      setUsage(value);
    }).catch(() => { if (!disposed) setError("دریافت محدودیت مصرف ممکن نشد. وضعیت ورود و مسیر Codex CLI را بررسی کنید."); })
      .finally(() => { if (!disposed) setLoading(false); });
    return () => { disposed = true; };
  }, [profileId, refreshTick]);
  useEffect(() => {
    const clock = setInterval(() => setNow(Date.now() / 1000), 1000);
    const poll = setInterval(() => setRefreshTick(value => value + 1), 300_000);
    return () => { clearInterval(clock); clearInterval(poll); };
  }, []);
  const locale = language === "fa" ? "fa-IR" : "en-US";
  const date = (seconds: number) => new Intl.DateTimeFormat(locale, {dateStyle:"medium",timeStyle:"short"}).format(seconds * 1000);
  const coolingDown = usage !== undefined && now < usage.checkedAt + 60;
  return <section className="usage-panel" aria-labelledby="usage-title" aria-busy={loading}>
    <div className="usage-heading"><h2 id="usage-title">{t("محدودیت مصرف حساب فعال")}</h2>
      <button disabled={loading || coolingDown} onClick={() => setRefreshTick(value => value + 1)}>{loading ? t("در حال دریافت…") : t("تازه‌سازی مصرف")}</button></div>
    {error && <p className="usage-error" role="status">{t(error)} {usage && t("آخرین اطلاعات دریافت‌شده نمایش داده می‌شود.")}</p>}
    {!usage && !error && <p role="status">{t("در حال دریافت اطلاعات از Codex…")}</p>}
    {usage && <>
      {usage.ordinaryUsageAllowed === false && <p className="usage-error">{t("Codex در حال حاضر محدودیت مصرف این حساب را اعلام کرده است.")}</p>}
      <div className="usage-windows">{usage.windows.map(window => {
        const remaining = remainingPercent(window.usedPercent);
        const label = windowLabel(window.durationMins, window.kind, language);
        return <div className="usage-window" key={window.kind}><h3>{label}</h3>
          <strong>{new Intl.NumberFormat(locale, {maximumFractionDigits:1,style:"percent"}).format(remaining / 100)} <small>{t("باقی‌مانده")}</small></strong>
          <progress max={100} value={remaining} aria-label={`${label} ${t("باقی‌مانده")}`}/>
          <p>{t("تا بازنشانی:")} {resetCountdown(window.resetsAt, now, language)}</p>
          {window.resetsAt != null && <small>{t("بازنشانی در:")} {date(window.resetsAt)}</small>}
        </div>;
      })}</div>
      <p className="usage-meta">{t("آخرین بررسی:")} {date(usage.checkedAt)} · {t("تازه‌سازی خودکار هر پنج دقیقه")}</p>
      {usage.windows.every(window => window.durationMins != null && window.durationMins >= 1440) &&
        <p className="usage-meta">{t("Codex در این پاسخ، محدودیت ساعتی جداگانه‌ای اعلام نکرده است.")}</p>}
    </>}
  </section>;
}
