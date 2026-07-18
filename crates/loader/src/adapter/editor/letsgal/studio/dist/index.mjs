import { Extension, meta, settings } from "@avg-studio/sdk";

const ENGINE_PATH = __CRABGAL_ENGINE_PATH__;
const PROJECT_PATH = __CRABGAL_PROJECT_PATH__;
const EXTENSION_ID = "maincore.crabgal-preview";
const RUNTIME_SLOT = Symbol.for(`${EXTENSION_ID}.studio-runtime`);
const LAUNCH_SLOT = Symbol.for(`${EXTENSION_ID}.engine-launches`);
const DEFAULT_PORT = 39698;
const HEARTBEAT_MS = 1000;

export const manifest = {
  id: EXTENSION_ID,
  name: "crabgal 调试同步",
  description: "在 Studio 状态面板中启动 crabgal 并同步当前步进。",
  author: "maincore",
  version: "0.9.0",
  entry: "dist/index.mjs",
  sdkVersion: ">=1.0.0",
  minHostVersion: "1.7.0",
};

function bridgeUrl(port, path) {
  return `http://127.0.0.1:${port}/v1/${path}`;
}

function readOptions(ctx) {
  const rawPort = Number(ctx.settings.get("port") ?? DEFAULT_PORT);
  return {
    enabled: ctx.settings.get("enabled") ?? true,
    restartOnFragment: ctx.settings.get("restartOnFragment") ?? true,
    showDiagnostics: ctx.settings.get("showDiagnostics") ?? true,
    diagnosticsLines: Math.max(50, Number(ctx.settings.get("diagnosticsLines") ?? 200)),
    port: Number.isInteger(rawPort) && rawPort > 0 && rawPort <= 65535
      ? rawPort
      : DEFAULT_PORT,
  };
}

function nodeRequire(name) {
  const load = globalThis.require ?? globalThis.window?.require;
  if (!load) throw new Error("Studio 未开放本机进程能力");
  return load(name);
}

function userDataRoot() {
  const path = nodeRequire("path");
  const os = nodeRequire("os");
  const process = nodeRequire("process");
  if (process.platform === "darwin") {
    return path.join(os.homedir(), "Library", "Application Support", "letsgal-studio");
  }
  if (process.platform === "win32") {
    return path.join(process.env.APPDATA, "letsgal-studio");
  }
  return path.join(
    process.env.XDG_CONFIG_HOME || path.join(os.homedir(), ".config"),
    "letsgal-studio",
  );
}

function logPath() {
  return nodeRequire("path").join(userDataRoot(), "logs", "crabgal-preview.log");
}

function setConnected(runtime, connected) {
  if (runtime.connected === connected) return;
  runtime.connected = connected;
  updateDiagnostics(runtime);
}

async function post(runtime, path, payload = {}) {
  if (runtime.disposed || !runtime.options.enabled) return false;
  try {
    const response = await fetch(bridgeUrl(runtime.options.port, path), {
      method: "POST",
      headers: { "Content-Type": "text/plain;charset=UTF-8" },
      body: JSON.stringify(payload),
      signal: runtime.abort.signal,
      cache: "no-store",
    });
    setConnected(runtime, response.ok);
    return response.ok;
  } catch (error) {
    setConnected(runtime, false);
    if (error?.name !== "AbortError") runtime.lastError = String(error?.message ?? error);
    return false;
  }
}

function processIsAlive(pid) {
  if (!pid) return false;
  try {
    nodeRequire("process").kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

async function runCrabgal(runtime) {
  if (runtime.launching) return;
  runtime.launching = true;
  runtime.lastError = "";
  updateDiagnostics(runtime);

  if (await post(runtime, "restart", { source: "studio-status-panel" })) {
    runtime.launching = false;
    setPanelOpen(runtime, false);
    return;
  }

  try {
    if (!ENGINE_PATH || !PROJECT_PATH) throw new Error("扩展缺少引擎或工程路径，请重新运行 cargo studio-sync");
    const launches = globalThis[LAUNCH_SLOT] ?? new Map();
    globalThis[LAUNCH_SLOT] = launches;
    const previous = launches.get(runtime.options.port);
    if (previous && processIsAlive(previous.pid)) {
      runtime.childPid = previous.pid;
      runtime.lastError = "crabgal 已启动，正在等待连接";
      return;
    }

    const fs = nodeRequire("fs");
    const path = nodeRequire("path");
    const childProcess = nodeRequire("child_process");
    const process = nodeRequire("process");
    const outputPath = logPath();
    fs.mkdirSync(path.dirname(outputPath), { recursive: true });
    const output = fs.openSync(outputPath, "a");
    let child;
    try {
      child = childProcess.spawn(
        ENGINE_PATH,
        ["studio", PROJECT_PATH, "--bridge-port", String(runtime.options.port)],
        {
          detached: true,
          stdio: ["ignore", output, output],
          env: { ...process.env },
        },
      );
    } finally {
      fs.closeSync(output);
    }
    child.unref();
    runtime.childPid = child.pid ?? null;
    launches.set(runtime.options.port, { pid: runtime.childPid, startedAt: Date.now() });
    child.once("error", (error) => {
      runtime.lastError = String(error?.message ?? error);
      runtime.childPid = null;
      updateDiagnostics(runtime);
    });
    child.once("exit", () => {
      if (runtime.childPid === child.pid) runtime.childPid = null;
      if (launches.get(runtime.options.port)?.pid === child.pid) launches.delete(runtime.options.port);
      updateDiagnostics(runtime);
    });
    setPanelOpen(runtime, false);
  } catch (error) {
    runtime.lastError = String(error?.message ?? error);
  } finally {
    runtime.launching = false;
    updateDiagnostics(runtime);
  }
}

function diagnosticsRoot() {
  return document.querySelector('[data-preview-root="true"]');
}

function previewRegion() {
  return diagnosticsRoot()?.children?.[1]?.firstElementChild ?? null;
}

function cleanLog(value) {
  return value.replace(/\u001b\[[0-?]*[ -/]*[@-~]/g, "").replace(/\r/g, "");
}

function readLogTail(runtime) {
  try {
    const fs = nodeRequire("fs");
    const text = cleanLog(fs.readFileSync(logPath(), "utf8"));
    const lines = text.split("\n");
    return lines.slice(Math.max(0, lines.length - runtime.options.diagnosticsLines)).join("\n").trim();
  } catch (error) {
    return runtime.lastError || `日志尚未创建。\n${String(error?.message ?? error)}`;
  }
}

function status(runtime) {
  if (!runtime.options.enabled) return { label: "已停用", tone: "idle" };
  if (runtime.connected) return { label: "已连接", tone: "ready" };
  if (runtime.lastError) return { label: "等待启动", tone: "waiting" };
  return { label: "未连接", tone: "idle" };
}

function updateDiagnostics(runtime, refreshLog = false) {
  const elements = runtime.diagnostics.elements;
  if (!elements) return;
  const current = status(runtime);
  elements.button.dataset.tone = current.tone;
  elements.status.dataset.tone = current.tone;
  elements.status.textContent = current.label;
  elements.details.textContent = [
    `127.0.0.1:${runtime.options.port}`,
    runtime.childPid ? `PID ${runtime.childPid}` : "独立窗口",
  ].join("  ·  ");
  elements.run.textContent = runtime.launching
    ? "正在启动…"
    : runtime.connected
      ? "同步当前步进"
      : "运行 CRABGAL";
  elements.run.disabled = runtime.launching;
  if (refreshLog && runtime.diagnostics.open) {
    elements.log.textContent = readLogTail(runtime) || "日志为空。";
    elements.log.scrollTop = elements.log.scrollHeight;
  }
}

function positionPanel(runtime) {
  if (!runtime.diagnostics.open || !runtime.diagnostics.panel) return;
  const region = previewRegion();
  if (!(region instanceof HTMLElement)) return;
  const rect = region.getBoundingClientRect();
  const width = Math.min(rect.width, Math.max(430, Math.round(rect.width * 0.62)));
  Object.assign(runtime.diagnostics.panel.style, {
    left: `${Math.round(rect.right - width)}px`,
    top: `${Math.round(rect.top)}px`,
    width: `${Math.round(width)}px`,
    height: `${Math.round(rect.height)}px`,
  });
}

function setPanelOpen(runtime, open) {
  const diagnostics = runtime.diagnostics;
  if (!diagnostics.panel || !diagnostics.elements) return;
  diagnostics.open = Boolean(open && runtime.options.showDiagnostics);
  diagnostics.panel.hidden = !diagnostics.open;
  diagnostics.elements.button.setAttribute("aria-expanded", String(diagnostics.open));
  if (diagnostics.open) {
    positionPanel(runtime);
    updateDiagnostics(runtime, true);
    diagnostics.logTimer ??= window.setInterval(() => updateDiagnostics(runtime, true), 700);
  } else if (diagnostics.logTimer) {
    window.clearInterval(diagnostics.logTimer);
    diagnostics.logTimer = null;
  }
}

function copyLog(runtime) {
  const text = runtime.diagnostics.elements?.log.textContent ?? "";
  navigator.clipboard?.writeText(text).catch(() => {});
}

function openLog(runtime) {
  try {
    nodeRequire("electron").shell.openPath(logPath());
  } catch (error) {
    runtime.lastError = String(error?.message ?? error);
    updateDiagnostics(runtime, true);
  }
}

function createDiagnostics(runtime) {
  const root = diagnosticsRoot();
  const toolbar = root?.firstElementChild;
  if (!(toolbar instanceof HTMLElement)) return false;

  const buttonHost = document.createElement("span");
  buttonHost.dataset.crabgalDiagnostics = "button";
  const buttonShadow = buttonHost.attachShadow({ mode: "open" });
  buttonShadow.innerHTML = `
    <style>
      :host { display: inline-flex; margin-left: 6px; }
      button { display: inline-flex; align-items: center; gap: 7px; height: 28px; padding: 0 10px;
        border: 1px solid rgba(255,255,255,.12); border-radius: 6px; color: rgba(255,255,255,.76);
        background: rgba(12,15,20,.46); font: 12px/1 system-ui,sans-serif; cursor: pointer;
        transition: background 120ms ease,color 120ms ease,border-color 120ms ease; }
      button:hover,button[aria-expanded="true"] { color: white; background: rgba(35,41,51,.9);
        border-color: rgba(255,255,255,.24); }
      i { width: 7px; height: 7px; border-radius: 50%; background: #89909c; }
      button[data-tone="ready"] i { background: #66d19e; box-shadow: 0 0 8px #66d19e; }
      button[data-tone="waiting"] i { background: #d6a84b; box-shadow: 0 0 8px #d6a84b; }
    </style>
    <button type="button" aria-label="打开 crabgal 状态" aria-expanded="false"><i></i><span>CRABGAL</span></button>`;

  const panel = document.createElement("section");
  panel.dataset.crabgalDiagnostics = "panel";
  panel.hidden = true;
  Object.assign(panel.style, { position: "fixed", zIndex: "2147483647" });
  const shadow = panel.attachShadow({ mode: "open" });
  shadow.innerHTML = `
    <style>
      :host { color: #e9edf3; font-family: system-ui,-apple-system,sans-serif; }
      * { box-sizing: border-box; }
      .panel { height: 100%; display: grid; grid-template-rows: auto 1fr; overflow: hidden;
        border-left: 1px solid rgba(255,255,255,.14); background: rgba(13,16,22,.96);
        box-shadow: -18px 0 48px rgba(0,0,0,.34); backdrop-filter: blur(18px); }
      header { display: grid; grid-template-columns: 1fr auto; gap: 14px; align-items: center;
        padding: 15px 16px 13px; border-bottom: 1px solid rgba(255,255,255,.1); }
      h2 { margin: 0 0 5px; font-size: 15px; font-weight: 620; }
      .meta { display: flex; align-items: center; gap: 9px; color: #929aa8; font-size: 11px; }
      .status::before { content: ""; display: inline-block; width: 7px; height: 7px; margin-right: 6px;
        border-radius: 50%; background: #89909c; }
      .status[data-tone="ready"]::before { background: #66d19e; }
      .status[data-tone="waiting"]::before { background: #d6a84b; }
      nav { display: flex; gap: 6px; }
      button { height: 29px; padding: 0 9px; border: 1px solid rgba(255,255,255,.1); border-radius: 5px;
        color: #c8ced8; background: rgba(255,255,255,.055); cursor: pointer; font: 12px/1 system-ui,sans-serif;
        transition: 120ms ease; }
      button:hover { color: white; border-color: rgba(255,255,255,.24); background: rgba(255,255,255,.11); }
      button.primary { padding: 0 13px; color: white; background: rgba(70,124,102,.66); }
      button.primary:hover { background: rgba(79,151,119,.82); }
      button:disabled { opacity: .55; cursor: default; }
      pre { min-height: 0; margin: 0; padding: 16px; overflow: auto; color: #cbd2dc;
        background: rgba(3,5,8,.42); font: 11.5px/1.58 ui-monospace,SFMono-Regular,Menlo,monospace;
        white-space: pre-wrap; overflow-wrap: anywhere; }
    </style>
    <div class="panel">
      <header>
        <div><h2>crabgal 状态</h2><div class="meta"><span class="status"></span><span class="details"></span></div></div>
        <nav><button class="primary" data-action="run">运行 CRABGAL</button><button data-action="copy">复制</button><button data-action="open">打开日志</button><button data-action="close">×</button></nav>
      </header>
      <pre aria-live="polite">正在读取日志…</pre>
    </div>`;

  toolbar.append(buttonHost);
  document.body.append(panel);
  const elements = {
    button: buttonShadow.querySelector("button"),
    status: shadow.querySelector(".status"),
    details: shadow.querySelector(".details"),
    run: shadow.querySelector('[data-action="run"]'),
    log: shadow.querySelector("pre"),
  };
  runtime.diagnostics = { ...runtime.diagnostics, buttonHost, panel, elements };
  elements.button.addEventListener("click", () => setPanelOpen(runtime, !runtime.diagnostics.open));
  elements.run.addEventListener("click", () => void runCrabgal(runtime));
  shadow.querySelector('[data-action="copy"]').addEventListener("click", () => copyLog(runtime));
  shadow.querySelector('[data-action="open"]').addEventListener("click", () => openLog(runtime));
  shadow.querySelector('[data-action="close"]').addEventListener("click", () => setPanelOpen(runtime, false));
  updateDiagnostics(runtime);
  return true;
}

function ensureDiagnostics(runtime) {
  const toolbar = diagnosticsRoot()?.firstElementChild;
  const current = runtime.diagnostics.buttonHost;
  for (const stale of document.querySelectorAll("[data-crabgal-diagnostics]")) {
    if (stale !== current && stale !== runtime.diagnostics.panel) stale.remove();
  }
  if (current?.isConnected && current.parentElement === toolbar) {
    current.style.display = runtime.options.showDiagnostics ? "" : "none";
    if (!runtime.options.showDiagnostics) setPanelOpen(runtime, false);
    return;
  }
  current?.remove();
  runtime.diagnostics.panel?.remove();
  runtime.diagnostics.buttonHost = null;
  runtime.diagnostics.panel = null;
  runtime.diagnostics.elements = null;
  if (createDiagnostics(runtime) && !runtime.options.showDiagnostics) {
    runtime.diagnostics.buttonHost.style.display = "none";
  }
}

function dispose(runtime) {
  if (runtime.disposed) return;
  runtime.disposed = true;
  runtime.abort.abort();
  window.clearInterval(runtime.heartbeat);
  window.clearInterval(runtime.diagnostics.logTimer);
  runtime.observer?.disconnect();
  for (const unsubscribe of runtime.unsubscribers.splice(0)) unsubscribe();
  runtime.diagnostics.buttonHost?.remove();
  runtime.diagnostics.panel?.remove();
}

function registerRuntime(ctx) {
  globalThis[RUNTIME_SLOT]?.dispose?.();
  const runtime = {
    disposed: false,
    connected: false,
    launching: false,
    childPid: null,
    lastError: "",
    options: readOptions(ctx),
    abort: new AbortController(),
    heartbeat: 0,
    observer: null,
    unsubscribers: [],
    diagnostics: { buttonHost: null, panel: null, elements: null, open: false, logTimer: null },
    dispose: null,
  };
  runtime.dispose = () => dispose(runtime);
  globalThis[RUNTIME_SLOT] = runtime;

  const applySettings = () => {
    runtime.options = readOptions(ctx);
    ensureDiagnostics(runtime);
    void post(runtime, "heartbeat", { source: "settings" });
  };
  for (const key of ["enabled", "restartOnFragment", "showDiagnostics", "diagnosticsLines", "port"]) {
    runtime.unsubscribers.push(ctx.settings.subscribe(key, applySettings));
  }
  runtime.unsubscribers.push(ctx.subscribe("fragment:entered", () => {
    if (runtime.options.restartOnFragment) void post(runtime, "restart", { source: "fragment:entered" });
  }));

  ensureDiagnostics(runtime);
  runtime.observer = new MutationObserver(() => ensureDiagnostics(runtime));
  runtime.observer.observe(document.documentElement, { childList: true, subtree: true });
  window.addEventListener("resize", () => positionPanel(runtime), { signal: runtime.abort.signal });
  window.addEventListener("pagehide", runtime.dispose, { once: true });
  runtime.heartbeat = window.setInterval(() => {
    ensureDiagnostics(runtime);
    void post(runtime, "heartbeat", { source: "timer" });
  }, HEARTBEAT_MS);
  void post(runtime, "heartbeat", { source: "register" });
}

export class CrabgalPreview extends Extension {
  static meta = meta({
    id: "debug-sync",
    label: "crabgal 调试同步",
    description: "从 Studio 状态面板启动 crabgal 并同步步进。",
    autonomous: true,
    exposeUI: false,
  });

  static settings = settings((s) => ({
    enabled: s.boolean("启用 crabgal 调试同步").default(true),
    restartOnFragment: s.boolean("进入片段时同步 crabgal").default(true).enabledWhen("enabled"),
    showDiagnostics: s.boolean("显示 CRABGAL 状态按钮").default(true).enabledWhen("enabled"),
    diagnosticsLines: s.number("状态面板日志行数").default(200).range(50, 1000).step(50)
      .enabledWhen("showDiagnostics"),
    port: s.number("本机调试端口").default(DEFAULT_PORT).range(1024, 65535).step(1)
      .enabledWhen("enabled"),
  }));

  static onRegister(ctx) {
    registerRuntime(ctx);
  }
}
