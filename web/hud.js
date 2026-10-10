// The heads-up display: what the core's hud() says, written into the page.
// Every number, string and position comes from the core.

const el = (id) => document.getElementById(id);

function text(id, value) {
  const node = el(id);
  if (node && node.textContent !== value) node.textContent = value;
}

// A row of pips, `filled` of `total` lit.
function pips(id, total, filled) {
  const node = el(id);
  while (node.children.length < total) node.appendChild(document.createElement("i"));
  while (node.children.length > total) node.lastChild.remove();
  Array.from(node.children).forEach((p, k) => {
    p.className = k < filled ? "on" : "";
  });
}

export function makeHud(copy, play) {
  let lastSeq = 0;
  const callout = el("callout");
  const word = el("c-word");
  const labels = el("gate-labels");
  const foes = el("foes");
  const reticle = el("reticle");
  const arc = el("hit-arc");
  const cue = el("jump-cue");
  const banner = el("story-banner");
  let lastStory = 0;

  return {
    // Forget what was heard, so a new arena's first note is not missed and
    // an old one is not replayed.
    reset(h) {
      if (h.story) lastStory = h.story.seq;
      if (!h.notes) return;
      lastSeq = h.notes.reduce((a, n) => (n.seq > a ? n.seq : a), lastSeq);
    },
    update(h) {
      text("h-zone", h.zone);
      text("h-year", h.year);
      text("h-node", h.node);
      text("h-enemies", h.cleared ? "" : copy.hud_enemies.replace("{n}", h.enemies));
      text("h-clemency", copy.hud_clemency.replace("{n}", h.clemency));
      el("h-health").style.width = `${h.health_pct}%`;
      text("h-health-text", `${h.health} / ${h.max_health}`);
      el("bottom-left").classList.toggle("low", h.low);
      pips("h-jumps", h.jumps, h.jumps_left);
      pips("h-dash", h.dashes, h.dash_left);
      el("h-dash").style.setProperty("--recharge", `${h.dash_pct}%`);
      el("h-hook").hidden = !h.grapple;
      el("h-hook").classList.toggle("ready", h.grapple_ready);
      el("h-parry").hidden = !h.parry;
      el("h-parry").classList.toggle("ready", h.parry_ready);
      el("h-parry").classList.toggle("active", h.parrying);
      el("h-wall").hidden = !h.wall;
      el("h-wall").classList.toggle("ready", h.wall_ready);
      el("h-wall").classList.toggle("active", h.walling);
      text("h-gun", h.gun);
      text("h-gun-level", h.gun_level);
      text("h-gun-trait", h.gun_trait);
      const art = `art/guns/${h.gun_id}.png`;
      if (el("h-gun-art").getAttribute("src") !== art) el("h-gun-art").setAttribute("src", art);
      el("bottom-right").style.setProperty("--gun", h.gun_colour);
      el("vignette").style.setProperty("--hurt", h.hurt);
      reticle.className = `${h.reticle} ${reticle.classList.contains("hitmark") ? "hitmark" : ""}`;

      const boss = el("boss");
      boss.hidden = !h.boss;
      if (h.boss) {
        text("b-name", h.boss.name);
        text("b-title", h.boss.title);
        el("b-fill").style.width = `${h.boss.pct}%`;
      }

      el("prompt").hidden = !h.prompt;
      if (h.prompt) text("prompt", h.prompt);
      text("hint", h.hint);

      while (labels.children.length < h.gates.length) labels.appendChild(document.createElement("div"));
      while (labels.children.length > h.gates.length) labels.lastChild.remove();
      h.gates.forEach((g, k) => {
        const node = labels.children[k];
        node.className = `gate-label ${g.main ? "main" : "side"} ${g.open ? "open" : "shut"}`;
        const body = g.reward ? `${g.path} · ${g.label} · ${g.reward}` : `${g.path} · ${g.label}`;
        if (node.textContent !== body) node.textContent = body;
        node.style.left = `${g.x}%`;
        node.style.top = `${g.y}%`;
      });

      while (foes.children.length < h.foes.length) {
        const node = document.createElement("div");
        node.appendChild(document.createElement("span"));
        foes.appendChild(node);
      }
      while (foes.children.length > h.foes.length) foes.lastChild.remove();
      h.foes.forEach((f, k) => {
        const node = foes.children[k];
        const threat = f.threat ? " threat" : "";
        node.className = f.edge ? `foe edge${threat}` : f.hurt ? `foe hurt${threat}` : `foe${threat}`;
        node.style.setProperty("--c", f.colour);
        node.style.left = `${f.x}%`;
        node.style.top = `${f.y}%`;
        node.style.setProperty("--a", `${f.angle}deg`);
        node.firstChild.style.width = `${f.pct}%`;
      });

      // the arc toward the last hit, fading
      arc.hidden = !h.hit;
      if (h.hit) {
        arc.style.setProperty("--a", `${h.hit.angle}deg`);
        arc.style.setProperty("--fade", h.hit.fade);
      }
      cue.hidden = !h.jump_cue;
      if (h.jump_cue) text("jump-cue", h.jump_text);
      const perks = el("h-perks");
      if (perks.dataset.list !== h.perks.join("|")) {
        perks.dataset.list = h.perks.join("|");
        perks.replaceChildren(
          ...h.perks.map((p) => {
            const chip = document.createElement("span");
            chip.className = "ability ready";
            chip.textContent = p;
            return chip;
          }),
        );
      }
      // a line of the indictment read during play
      if (h.story && h.story.seq > lastStory) {
        lastStory = h.story.seq;
        banner.querySelector(".heading").textContent = h.story.heading;
        banner.querySelector("p").textContent = h.story.text;
        banner.classList.remove("show");
        void banner.offsetWidth;
        banner.classList.add("show");
      }

      for (const n of h.notes) {
        if (n.seq <= lastSeq) continue;
        lastSeq = n.seq;
        play(n.sound);
        if (n.sound === "hit" || n.sound === "kill") {
          reticle.classList.remove("hitmark");
          void reticle.offsetWidth;
          reticle.classList.add("hitmark");
        }
        if (!n.text) continue;
        word.textContent = n.text;
        callout.classList.remove("pop");
        // restart the animation: reading the layout makes the removal count
        void callout.offsetWidth;
        callout.classList.add("pop");
      }
    },
  };
}
