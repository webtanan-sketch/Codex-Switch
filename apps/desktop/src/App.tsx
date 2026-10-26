import {useCallback, useEffect, useState} from "react";
import {invoke} from "@tauri-apps/api/core";
import {listen} from "@tauri-apps/api/event";
import type {Snapshot} from "@codex-manager/shared-types";
const date = (seconds: number) => new Intl.DateTimeFormat("fa-IR", {dateStyle:"medium", timeStyle:"short"}).format(seconds * 1000);
export default function App() {
  const [data,setData] = useState<Snapshot>();
  const [page,setPage] = useState("accounts");
  const [busy,setBusy] = useState(false);
  const [error,setError] = useState("");
  const [notice,setNotice] = useState("");
  const [name,setName] = useState("");
  const [login,setLogin] = useState<string>();
  const [codePath,setCodePath] = useState("");
  const [codexPath,setCodexPath] = useState("");
  const refresh = useCallback(async () => {
    const result = await invoke<Snapshot>("snapshot"); setData(result);
    setCodePath(result.settings.vscodePath ?? result.diagnostics.vscodePath ?? "");
    setCodexPath(result.settings.codexPath ?? result.diagnostics.codexPath ?? "");
  },[]);
  useEffect(() => {
    let disposed = false;
    void refresh().catch(() => setError("اتصال به برنامه برقرار نشد. نسخهٔ دسکتاپ را اجرا کنید. Desktop connection unavailable."));
    const registrations = [listen("profiles-updated", () => { void refresh().catch(() => {}); }), listen<string>("operation-error", e => setError(e.payload))];
    void Promise.all(registrations).then(stops => { if (disposed) stops.forEach(stop => stop()); });
    return () => { disposed = true; void Promise.all(registrations).then(stops => stops.forEach(stop => stop())); };
  },[refresh]);
  async function run(command: string, args?: Record<string,unknown>, success = "انجام شد") {
    setBusy(true); setError(""); setNotice("");
    try { await invoke(command,args); await refresh(); setNotice(success); return true; }
    catch(e) { setError(String(e)); return false; } finally { setBusy(false); }
  }
  async function startLogin() {
    setBusy(true); setError(""); try { setLogin(await invoke<string>("begin_login")); setNotice("ورود را در مرورگر تکمیل کنید و سپس ذخیرهٔ حساب را بزنید."); } catch(e) { setError(String(e)); } finally { setBusy(false); }
  }
  const state = data?.state;
  const active = state?.profiles.find(p => p.id === state.activeProfileId);
  return <div className="shell">
    <aside><div className="brand"><span className="brand-icon">C</span><div dir="ltr">CODEX<span>ACCOUNT MANAGER</span></div></div>
      <p className="side-label">فضای کار شما</p><nav aria-label="منوی اصلی">{[["accounts","◈","حساب‌ها","Accounts"],["settings","⚙","تنظیمات","Settings"],["recovery","↺","بازیابی","Recovery"]].map(([id,icon,label,en]) => <button key={id} aria-current={page === id ? "page" : undefined} onClick={() => setPage(id)}><b>{icon}</b>{label}<small>{en}</small></button>)}</nav>
      <div className="privacy"><span className="dot"/>ذخیره‌سازی محلی<small>رمزگذاری با حساب ویندوز · DPAPI</small></div>
    </aside>
    <main aria-busy={busy}><header><div><div className="eyebrow" dir="ltr">YOUR ACCOUNTS. ONE PLACE.</div><h1>{page === "accounts" ? "مدیریت حساب‌های Codex" : page === "settings" ? "تنظیمات و بررسی سیستم" : "بازیابی حساب‌ها"}</h1><p>حساب‌های کاری و شخصی، همیشه در دسترس شما.</p></div><span className="version">v0.1.0 · Preview</span></header>
      {error && <div className="alert error" role="alert"><b>عملیات تکمیل نشد</b><span dir="auto">{error}</span></div>}
      {notice && <div className="alert success" role="status">{notice}</div>}
      {data?.diagnostics.recoveryPending && <div className="alert error">یک تعویض ناتمام شناسایی شد. از بخش بازیابی استفاده کنید.</div>}
      {page === "accounts" && <>
        <section className="active-banner"><span className="active-symbol">↔</span><div><small>حساب فعال · Active account</small><h2>{active?.name ?? "هنوز حسابی انتخاب نشده"}</h2><p>{active ? "حساب انتخاب‌شده برای محیط اصلی Codex" : "حساب فعلی را وارد یا حساب جدیدی اضافه کنید."}</p></div>{active && <span className="badge"><i className="dot"/>فعال</span>}</section>
        <div className="section-title"><h2>حساب‌های ذخیره‌شده <span>{state?.profiles.length ?? 0}</span></h2><button className="primary" disabled={busy} onClick={() => { setPage("add"); setName(""); }}>＋ افزودن حساب</button></div>
        <div className="grid">{state?.profiles.map((p,i) => <article className={`card ${p.id === state.activeProfileId ? "selected" : ""}`} key={p.id}><div className="card-head"><span className={`avatar color-${i%3}`}>{p.name.slice(0,1)}</span><div><h3>{p.name}</h3><span className="email" dir="ltr">{p.identity.email ?? "حساب ChatGPT"}</span></div>{p.id === state.activeProfileId && <span className="dot"/>}</div><div className="card-meta"><span>{p.identity.plan ?? "ChatGPT"}</span><span>ذخیرهٔ امن · DPAPI</span></div><small className="sync">آخرین همگام‌سازی: {date(p.lastSyncedAt)}</small><div className="card-actions"><button disabled={busy || p.id === state.activeProfileId || data?.diagnostics.storage !== "file"} onClick={() => void run("switch_profile",{id:p.id},"حساب تغییر کرد. افزونهٔ همراه VS Code را تازه می‌کند.")}>{p.id === state.activeProfileId ? "حساب فعال" : "تعویض حساب"}</button><button disabled={busy} onClick={() => void run("open_vscode",{id:p.id},"پنجرهٔ مستقل VS Code باز شد. افزونه‌های Codex و همراه را در آن نصب کنید.")}>باز کردن VS Code ↗</button></div></article>)}</div>
        {!state?.profiles.length && <section className="empty"><h2>اولین حساب را اضافه کنید</h2><p>برای استفاده از حساب فعلی، «وارد کردن حساب فعلی» را انتخاب کنید.</p><button disabled={busy} onClick={() => setPage("add")}>شروع کنید ←</button></section>}
        <div className="hint">پنجرهٔ مستقل VS Code برای هر حساب با مسیر داده و محیط جدا باز می‌شود.</div>
      </>}
      {page === "add" && <section className="panel"><h2>افزودن حساب · Add account</h2><p>ورود جدید در مسیر جدا انجام می‌شود؛ حساب اصلی شما حفظ می‌شود.</p><label>نام حساب<input value={name} maxLength={80} onChange={e => setName(e.target.value)} placeholder="مثلاً وب‌تنان یا شخصی" /></label><div className="actions"><button className="primary" disabled={busy || !name.trim() || Boolean(login)} onClick={() => void startLogin()}>ورود با ChatGPT</button><button disabled={busy || !name.trim()} onClick={() => void run("import_current",{name},"حساب فعلی با موفقیت ذخیره شد.")}>وارد کردن حساب فعلی</button></div>{login && <div className="login-state"><p>ورود را در مرورگر/پنجرهٔ Codex تکمیل کنید. سپس:</p><button className="primary" disabled={busy || !name.trim()} onClick={() => void run("finish_login",{id:login,name},"حساب ذخیره شد؛ اکنون می‌توانید آن را انتخاب کنید.").then(ok => { if(ok) { setLogin(undefined); setPage("accounts"); } })}>ذخیرهٔ حساب</button></div>}<button className="text-button" onClick={() => setPage("accounts")}>بازگشت به حساب‌ها</button></section>}
      {page === "settings" && <section className="panel"><h2>بررسی سیستم · System check</h2><div className="checks">{[["Windows 10/11",data?.diagnostics.windows],["VS Code",Boolean(data?.diagnostics.vscodePath)],["Codex CLI",Boolean(data?.diagnostics.codexPath)],["auth.json",data?.diagnostics.authPresent],["File authentication",data?.diagnostics.storage === "file"],["Local IPC",data?.ipcReady]].map(([label,ok]) => <div key={String(label)}><span className={ok ? "ok" : "warn"}>{ok ? "✓" : "○"}</span><span dir="ltr">{label}</span></div>)}</div><label>مسیر اصلی Codex<input dir="ltr" readOnly value={data?.diagnostics.home ?? ""}/></label>{data?.diagnostics.storage !== "file" && <div className="alert"><p>روش ذخیرهٔ اعتبارنامه: <b>{data?.diagnostics.storage}</b>. تعویض حساب به حالت فایل نیاز دارد. یک نسخهٔ پشتیبان از تنظیمات ساخته می‌شود؛ سیاست مدیر سازمان ممکن است این تغییر را نپذیرد.</p><button disabled={busy} onClick={() => { if(window.confirm("ذخیرهٔ فایل‌محور فعال شود؟ سایر تنظیمات Codex حفظ می‌شوند.")) void run("configure_file_storage"); }}>فعال‌سازی ذخیرهٔ فایل‌محور</button></div>}<label>مسیر Code.exe<input dir="ltr" value={codePath} onChange={e => setCodePath(e.target.value)} placeholder="C:\...\Code.exe"/></label><label>مسیر codex.exe (باینری اصلی)<input dir="ltr" value={codexPath} onChange={e => setCodexPath(e.target.value)} placeholder="C:\...\codex.exe"/></label><button className="primary" disabled={busy} onClick={() => void run("save_settings",{settings:{vscodePath:codePath || null,codexPath:codexPath || null}})}>ذخیرهٔ تنظیمات</button>{data?.diagnostics.legacyMaintenancePresent && <p className="hint">پوشهٔ maintenance/v1 مربوط به نگهداری است و برای یافتن حساب‌ها جستجو نمی‌شود.</p>}{data?.diagnostics.legacyProfilesPresent && <p className="hint">پروفایل‌های قدیمی پیدا شدند. قالب آن‌ها هنوز تأیید نشده است؛ هر حساب را در ابزار قبلی فعال و سپس حساب فعلی را وارد کنید.</p>}</section>}
      {page === "recovery" && <section className="panel"><h2>بازیابی امن · Recovery</h2><p>فایل احراز هویت Codex با بازنشانی وضعیت برنامه حذف نمی‌شود.</p><div className="recovery-actions"><button disabled={busy || !state?.previousProfileId} onClick={() => void run("restore_previous")}>بازگشت به حساب قبلی</button><button disabled={busy} onClick={() => void run("recover")}>بازیابی تراکنش ناتمام</button><button disabled={busy} onClick={() => void run("repair")}>ترمیم فایل احراز هویت فعال</button><button disabled={busy} onClick={() => setPage("add")}>وارد کردن دوبارهٔ حساب فعلی</button><button disabled={busy} onClick={() => { if(window.confirm("وضعیت انتخاب حساب بازنشانی شود؟ حساب‌های ذخیره‌شده و ورود Codex حفظ می‌شوند.")) void run("reset_state"); }}>بازنشانی وضعیت برنامه</button></div></section>}
      <footer><span className={`dot ${data?.ipcReady ? "" : "offline"}`}/>{data?.ipcReady ? "ارتباط محلی آماده است" : "در انتظار ارتباط محلی"}<span dir="ltr">Built by Webtanan · Local by design</span></footer>
    </main>
  </div>;
}
