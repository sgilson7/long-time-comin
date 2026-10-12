// The page draws what the core returns and decides nothing. It loads the data
// files, starts the core through the shim, and each display frame passes the
// clock, the held actions and the mouse movement in, then draws what comes
// out. The core says which mode the game is in; the page shows that screen.
import init, { Game, numbers, mesh } from "./pkg/ltc_shim.js";
import { makeRenderer } from "./gl.js";
import { makeInput } from "./input.js";
import { makeHud } from "./hud.js";
import { makeScreens } from "./screens.js";
import { setup as setupSound, wake, toggleMute, play } from "./sound.js";

const BUILD = "__BUILD__";
const SAVE_KEY = "long-time-comin.record";
const el = (id) => document.getElementById(id);
const file = (path) => fetch(`${path}?v=${BUILD}`).then((r) => r.text());

// Browser storage can be missing or refused; the game works without it.
const storage = {
  load() {
    try {
      return localStorage.getItem(SAVE_KEY) || "";
    } catch {
      return "";
    }
  },
  save(text) {
    try {
      localStorage.setItem(SAVE_KEY, text);
    } catch {
      // nothing to do: the record is simply not kept
    }
  },
};

function fillCopy(copy) {
  for (const node of document.querySelectorAll("[data-copy]")) {
    node.textContent = copy[node.dataset.copy];
  }
  document.title = copy.title;
  el("build").textContent = BUILD;
}

// The controls tables, from the core's Codex (its rows carry the numbers).
function fillControls(rows) {
  for (const table of document.querySelectorAll("table.controls")) {
    for (const [keys, does] of rows) {
      const row = table.insertRow();
      row.insertCell().textContent = keys;
      row.insertCell().textContent = does;
    }
  }
}

async function start() {
  const names = ["weapons", "patterns", "enemies", "zone", "upgrades", "palette", "story", "copy", "controls", "sounds", "display"];
  const texts = await Promise.all(
    names.map((n) => file(`data/${n}.json`)).concat([file("data/shaders/scene.vert"), file("data/shaders/scene.frag")]),
  );
  const [weapons, patterns, enemies, zone, upgrades, palette, story, copyText, controlsText, soundsText, displayText, vertex, fragment] =
    texts;
  const display = JSON.parse(displayText);
  const copy = JSON.parse(copyText);
  fillCopy(copy);
  setupSound(JSON.parse(soundsText));
  await init();
  const N = JSON.parse(numbers());

  let game;
  try {
    game = new Game(weapons, patterns, enemies, zone, upgrades, palette, story, copyText, storage.load());
  } catch (e) {
    el("message").textContent = String(e);
    document.body.dataset.ready = "true";
    return;
  }

  const codex = JSON.parse(game.codex());
  fillControls(codex.controls);

  const canvas = el("view");
  const renderer = makeRenderer(canvas, N, { vertex, fragment, software: display.software }, (id) => mesh(id));
  if (!renderer) el("message").textContent = copy.no_webgl;
  const save = () => storage.save(game.saved());
  const input = makeInput(canvas, JSON.parse(controlsText), N.actions, () => wake(BUILD));
  const hud = makeHud(copy, play);
  let shownMode = null;
  const screens = makeScreens(
    game,
    copy,
    () => {
      save();
      showMode(JSON.parse(game.hud()).mode, true);
    },
    (zone) =>
      act(() => {
        game.start_run_at(zone);
        engage();
      })(),
  );

  // a button that changes the mode: wake the sound, act, keep the record
  const act = (fn) => () => {
    wake(BUILD);
    fn();
    if (document.activeElement) document.activeElement.blur();
    save();
  };
  const engage = () => {
    input.engage();
    hud.reset(JSON.parse(game.hud()));
  };
  el("begin").addEventListener("click", act(() => game.begin()));
  el("intro-next").addEventListener("click", act(() => game.finish_intro()));
  el("replay").addEventListener("click", act(() => game.replay_intro()));
  el("start-run").addEventListener(
    "click",
    act(() => {
      game.start_run();
      engage();
    }),
  );
  el("start-rush").addEventListener(
    "click",
    act(() => {
      game.start_rush();
      engage();
    }),
  );
  // a faint mark under the credits: the lullaby alone, for testing
  el("try-lullaby").title = copy.try_lullaby_title;
  el("try-lullaby").addEventListener(
    "click",
    act(() => {
      game.practice_lullaby();
      engage();
    }),
  );
  // the last mission opens on its brief, read on the interlude's screen
  el("start-mission").addEventListener(
    "click",
    act(() => game.start_mission()),
  );
  el("engage-btn").addEventListener("click", act(engage));
  el("resume").addEventListener(
    "click",
    act(() => {
      game.resume();
      engage();
    }),
  );
  el("map-close").addEventListener(
    "click",
    act(() => {
      game.resume();
      engage();
    }),
  );
  el("pause-map").addEventListener("click", act(() => game.toggle_map()));
  const abandon = el("abandon");
  abandon.addEventListener("click", () => {
    if (!abandon.classList.contains("armed")) {
      abandon.classList.add("armed");
      abandon.textContent = copy.abandon_confirm;
      return;
    }
    abandon.classList.remove("armed");
    abandon.textContent = copy.abandon;
    act(() => game.abandon())();
  });
  // the Codex opens over whatever is showing; in play it pauses first
  for (const b of document.querySelectorAll(".open-codex")) {
    b.addEventListener("click", () => {
      game.pause();
      screens.codex(codex);
      el("codex").hidden = false;
      b.blur();
    });
  }
  el("codex-close").addEventListener("click", () => {
    el("codex").hidden = true;
    el("codex-close").blur();
  });
  el("interlude-next").addEventListener(
    "click",
    act(() => {
      game.continue_run();
      engage();
    }),
  );
  el("dead-next").addEventListener("click", act(() => game.to_holding()));
  el("won-next").addEventListener("click", act(() => game.to_holding()));
  const reset = el("reset");
  reset.addEventListener("click", () => {
    if (!reset.classList.contains("armed")) {
      reset.classList.add("armed");
      reset.textContent = copy.reset_confirm;
      return;
    }
    reset.classList.remove("armed");
    reset.textContent = copy.reset;
    act(() => game.reset())();
  });
  // the test save asks twice, as the reset does: it replaces the record
  const testSave = el("test-save");
  testSave.addEventListener("click", () => {
    if (!testSave.classList.contains("armed")) {
      testSave.classList.add("armed");
      testSave.textContent = copy.test_save_confirm;
      return;
    }
    testSave.classList.remove("armed");
    testSave.textContent = copy.test_save;
    act(() => game.test_save())();
    showMode(JSON.parse(game.hud()).mode, true);
  });

  // losing the pointer in play pauses; Tab or M opens the map
  // only a lock that was held and then lost pauses: a browser that grants
  // the lock and drops it at once (headless Linux) plays on in the fallback
  let held = false;
  document.addEventListener("pointerlockchange", () => {
    const now = Boolean(document.pointerLockElement);
    if (held && !now && JSON.parse(game.hud()).mode === "play") game.pause();
    held = now;
  });
  window.addEventListener("keydown", (e) => {
    const mode = document.body.dataset.mode;
    if (e.code === "Tab" || e.code === "KeyM") {
      e.preventDefault();
      if (mode === "play" || mode === "map" || mode === "paused") game.toggle_map();
    }
    if (e.code === "Escape" && !el("codex").hidden) {
      el("codex").hidden = true;
      return;
    }
    if (e.code === "Escape" && mode === "map") game.resume();
    if (e.code === "Escape" && mode === "play") game.pause();
    if (e.code === "KeyN") toggleMute();
    wake(BUILD);
  });
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) game.pause();
  });

  function resize() {
    const scale = renderer && renderer.software ? display.software_scale : 1;
    const [w, h] = game.resize(canvas.clientWidth, canvas.clientHeight, (window.devicePixelRatio || 1) * scale);
    if (renderer) renderer.resize(w, h);
  }
  window.addEventListener("resize", resize);
  resize();

  const overlays = { title: "title", intro: "intro", holding: "holding", map: "map", paused: "paused", interlude: "interlude", dead: "dead", won: "won" };
  function showMode(mode, again) {
    if (mode === shownMode && !again) return;
    document.body.dataset.mode = mode;
    for (const [m, id] of Object.entries(overlays)) el(id).hidden = mode !== m;
    el("hud").hidden = !["play", "paused", "map"].includes(mode);
    const s = JSON.parse(game.screen());
    if (mode === "intro") screens.intro(s);
    if (mode === "holding") screens.holding(s);
    if (mode === "map") screens.map(s);
    if (mode === "interlude") screens.interlude(s);
    if (mode === "dead") screens.dead(s);
    if (mode === "won") screens.won(s);
    if (mode === "dead" || mode === "won" || mode === "holding" || mode === "interlude") input.release();
    if (mode === "dead" || mode === "won" || mode === "interlude") save();
    shownMode = mode;
  }

  function frame(now) {
    const { bits, dx, dy } = input.read();
    game.advance(now, bits, dx, dy);
    const h = JSON.parse(game.hud());
    showMode(h.mode, false);
    if (h.mode === "play") {
      hud.update(h);
      el("engage").hidden = input.engaged();
    }
    if (renderer) {
      if (game.scene_version() !== sceneVersion) {
        sceneVersion = game.scene_version();
        renderer.setScene(game.static_instances(), game.uniforms(), game.clear_color());
      }
      renderer.frame(game.view(), game.instances(), game.draws());
    }
    requestAnimationFrame(frame);
  }
  let sceneVersion = null;

  hud.reset(JSON.parse(game.hud()));
  document.body.dataset.ready = "true";
  requestAnimationFrame(frame);
}

start();
