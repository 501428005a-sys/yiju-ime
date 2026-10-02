// 把 demo.html 逐帧截成 PNG，再用 ffmpeg 合成 demo.gif。
// 用法：node render.mjs [浏览器路径] [ffmpeg 路径]   （需要 Node 22+，本机装的 Chrome / Edge）
// 只想看某几帧：node render.mjs --peek 2.0,5.6,7.6
import { spawn, spawnSync } from "node:child_process";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const peek = args[0] === "--peek" ? args.splice(0, 2)[1].split(",").map(Number) : null;
const browser = args[0] ?? "C:/Program Files/Google/Chrome/Application/chrome.exe";
const ffmpeg = args[1] ?? "ffmpeg";
const FPS = 12.5; // 80 ms 一帧，与打字节拍一致
const frames = join(here, "frames");
const profile = join(tmpdir(), "yiju-demo-profile");

rmSync(frames, { recursive: true, force: true });
mkdirSync(frames);

const port = 9333;
const chrome = spawn(browser, [
  "--headless=new", `--remote-debugging-port=${port}`, "--hide-scrollbars",
  "--force-device-scale-factor=1", `--user-data-dir=${profile}`, "about:blank",
], { stdio: "ignore" });

let target;
for (let i = 0; i < 50 && !target; i++) {
  await new Promise(r => setTimeout(r, 200));
  try {
    const list = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
    target = list.find(t => t.type === "page");
  } catch {}
}
const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise(r => ws.addEventListener("open", r));
let id = 0;
const pending = new Map();
ws.addEventListener("message", e => {
  const m = JSON.parse(e.data);
  if (pending.has(m.id)) { pending.get(m.id)(m.result); pending.delete(m.id); }
});
const send = (method, params = {}) => new Promise(r => { pending.set(++id, r); ws.send(JSON.stringify({ id, method, params })); });

await send("Emulation.setDeviceMetricsOverride", { width: 760, height: 470, deviceScaleFactor: 1, mobile: false });
await send("Page.enable");
await send("Page.navigate", { url: pathToFileURL(join(here, "demo.html")).href });
await new Promise(r => setTimeout(r, 1500));
const { result } = await send("Runtime.evaluate", { expression: "DURATION", returnByValue: true });
const times = peek ?? Array.from({ length: Math.ceil(result.value * FPS) }, (_, i) => i / FPS);

for (const [i, t] of times.entries()) {
  await send("Runtime.evaluate", { expression: `render(${t})` });
  const shot = await send("Page.captureScreenshot", { format: "png" });
  writeFileSync(join(frames, `f${String(i).padStart(4, "0")}.png`), Buffer.from(shot.data, "base64"));
}
ws.close();
chrome.kill();
console.log(`${times.length} frames`);

if (!peek) {
  const gif = join(here, "..", "..", "docs", "images", "demo.gif");
  mkdirSync(dirname(gif), { recursive: true });
  const r = spawnSync(ffmpeg, [
    "-y", "-loglevel", "error", "-framerate", String(FPS), "-i", join(frames, "f%04d.png"),
    "-filter_complex", "[0]split[a][b];[a]palettegen=max_colors=128:stats_mode=diff[p];[b][p]paletteuse=dither=none:diff_mode=rectangle",
    "-loop", "0", gif,
  ], { stdio: "inherit" });
  if (r.status !== 0) process.exit(r.status ?? 1);
  rmSync(frames, { recursive: true, force: true });
  console.log(gif);
}
try { rmSync(profile, { recursive: true, force: true }); } catch {}
