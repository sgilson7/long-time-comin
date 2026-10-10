#!/usr/bin/env python3
"""Screenshots of the built game, for looking at it: the title, the
Holding, the crater, a fight and the map. Not a check: it asserts nothing.

    .venv/bin/python tools/shots.py OUT_DIR [seconds_of_fight]
"""
import functools, http.server, pathlib, sys, threading
from playwright.sync_api import sync_playwright

ROOT = pathlib.Path(__file__).resolve().parents[1]
OUT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "shots")
FIGHT_S = float(sys.argv[2]) if len(sys.argv) > 2 else 6
OUT.mkdir(parents=True, exist_ok=True)


class H(http.server.SimpleHTTPRequestHandler):
    extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, ".wasm": "application/wasm", ".js": "text/javascript"}

    def log_message(self, *a):
        pass


srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(H, directory=str(ROOT / "dist")))
threading.Thread(target=srv.serve_forever, daemon=True).start()
base = f"http://127.0.0.1:{srv.server_address[1]}"

with sync_playwright() as p:
    import os
    soft = ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"]
    gpu = ["--use-angle=metal", "--enable-gpu", "--ignore-gpu-blocklist"]
    b = p.chromium.launch(args=soft if os.environ.get("SOFT") else gpu)
    pg = b.new_page(viewport={"width": 1280, "height": 720})
    logs = []
    pg.on("console", lambda m: logs.append(f"{m.type}: {m.text}"))
    pg.on("pageerror", lambda e: logs.append(f"pageerror: {e}"))
    pg.goto(base + "/index.html")
    pg.wait_for_selector("body[data-ready='true']", state="attached", timeout=30000)
    pg.wait_for_timeout(800)
    fps = pg.evaluate("() => new Promise(r => { let n = 0; const t0 = performance.now(); const f = () => { n++; if (performance.now() - t0 < 2000) requestAnimationFrame(f); else r(n / 2); }; requestAnimationFrame(f); })")
    print("frames a second on the title:", fps)
    pg.click("#begin")
    pg.wait_for_timeout(300)
    if pg.evaluate("() => document.body.dataset.mode") == "intro":
        pg.click("#intro-next")
    pg.click("#start-run")
    pg.wait_for_timeout(1500)
    fps = pg.evaluate("() => new Promise(r => { let n = 0; const t0 = performance.now(); const f = () => { n++; if (performance.now() - t0 < 3000) requestAnimationFrame(f); else r(n / 3); }; requestAnimationFrame(f); })")
    print("frames a second in the crater:", fps)
    pg.reload()
    pg.wait_for_selector("body[data-ready='true']", state="attached", timeout=30000)
    pg.evaluate("() => localStorage.clear()")
    pg.reload()
    pg.wait_for_selector("body[data-ready='true']", state="attached", timeout=30000)
    pg.screenshot(path=str(OUT / "1-title.png"))
    pg.click("#begin")
    pg.wait_for_timeout(5500)
    pg.screenshot(path=str(OUT / "2-intro.png"))
    pg.click("#intro-next")
    pg.wait_for_timeout(600)
    pg.screenshot(path=str(OUT / "3-holding.png"))
    pg.click("#start-run")
    pg.wait_for_timeout(1500)
    pg.screenshot(path=str(OUT / "4-crater.png"))
    pg.keyboard.down("KeyW")
    pg.wait_for_timeout(6500)
    pg.keyboard.up("KeyW")
    pg.wait_for_timeout(int(FIGHT_S * 1000))
    pg.screenshot(path=str(OUT / "5-fight.png"))
    # strafe and fire, turning, for a few seconds
    pg.mouse.move(640, 360)
    pg.mouse.down()
    for k in range(8):
        pg.keyboard.down("KeyA" if k % 2 == 0 else "KeyD")
        pg.mouse.move(640 + (40 if k % 2 else -40), 360)
        pg.wait_for_timeout(700)
        pg.keyboard.up("KeyA" if k % 2 == 0 else "KeyD")
        if k % 3 == 0:
            pg.keyboard.press("Space")
        pg.screenshot(path=str(OUT / f"6-fight-{k}.png"))
    pg.mouse.up()
    pg.keyboard.press("Tab")
    pg.wait_for_timeout(400)
    pg.screenshot(path=str(OUT / "7-map.png"))
    print(pg.evaluate("() => document.body.dataset.mode"))
    print("\n".join(logs[-30:]))
    b.close()
srv.shutdown()
