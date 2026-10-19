// The screens between fights: the sentencing, the Holding with its shop
// and case file, the route map, the interlude between zones, the
// indictment read on a death, and the verdict. Each draws what the core's
// screen() says.

const el = (id) => document.getElementById(id);

function make(tag, cls, textContent) {
  const node = document.createElement(tag);
  if (cls) node.className = cls;
  if (textContent !== undefined) node.textContent = textContent;
  return node;
}

function lines(container, list, cls) {
  container.replaceChildren(...list.map((t) => make("p", cls, t)));
}

function table(rows) {
  const t = make("table", "stats");
  for (const [k, v] of rows) {
    const r = t.insertRow();
    r.insertCell().textContent = k;
    r.insertCell().textContent = v;
  }
  return t;
}

export function makeScreens(game, copy, changed, startAt, tryChumbles) {
  let tab = copy.codex_tabs[0][0];
  // the upgrade picked on the Holding's tree, whose card is read below it
  let picked = "";

  // each tab of the Codex, built from the core's numbers
  const pages = {
    controls: (cx) => [table(cx.controls)],
    guns: (cx) =>
      cx.guns.map((g) => {
        const card = make("section", "codex-card");
        const art = make("img", "gun-art");
        art.src = `art/guns/${g.id}.png`;
        art.alt = "";
        card.append(art, make("h3", "", g.name), make("p", "", g.role));
        const t = make("table", "levels");
        const head = t.insertRow();
        for (const h of [copy.codex_level, copy.codex_damage, copy.codex_rate, copy.codex_dps, copy.codex_speed, copy.codex_range, copy.codex_spread]) {
          head.appendChild(make("th", "", h));
        }
        for (const l of g.levels) {
          const r = t.insertRow();
          for (const v of [l.level, l.damage, l.rate, l.dps, l.speed, l.range, l.spread]) r.insertCell().textContent = v;
        }
        card.append(t);
        for (const n of g.notes) card.append(make("p", "note", n));
        for (const l of g.levels) {
          for (const n of l.notes) card.append(make("p", "note", `${copy.codex_level} ${l.level}: ${n}`));
        }
        return card;
      }),
    enemies: (cx) =>
      cx.enemies.map((e) => {
        const card = make("section", "codex-card");
        card.append(make("h3", "", e.name));
        if (e.zone) card.append(make("p", "note", e.zone));
        card.append(
          table([
            [copy.codex_health, e.health],
            [copy.codex_size, e.size],
            [copy.codex_touch, e.touch],
            [copy.codex_clemency, e.clemency],
          ]),
        );
        card.append(make("p", "", e.moves), make("p", "note", e.order));
        for (const ph of e.phases) {
          if (ph.from) card.append(make("h4", "", ph.from));
          ph.attacks.forEach((a) => {
            const block = make("div", "attack");
            block.append(make("p", "windup", a.windup));
            const list = make("ul", "");
            for (const v of a.volleys) list.append(make("li", "", v));
            block.append(list);
            card.append(block);
          });
        }
        return card;
      }),
    upgrades: (cx) =>
      cx.upgrades.map((u) => {
        const card = make("section", "codex-card row");
        const icon = make("img", "icon");
        icon.src = `art/icons/${u.icon}.png`;
        icon.alt = "";
        const body = make("div", "");
        const cost = u.needs ? `${copy.codex_cost} ${u.cost} · ${copy.codex_needs} ${u.needs}` : `${copy.codex_cost} ${u.cost}`;
        body.append(make("h3", "", u.name), make("p", "note", cost), make("p", "", u.effect));
        card.append(icon, body);
        return card;
      }),
    rules: (cx) => {
      const list = make("ul", "rules");
      for (const r of cx.rules) list.append(make("li", "", r));
      return [list];
    },
  };

  return {
    codex(cx) {
      const tabs = el("codex-tabs");
      const draw = () => {
        tabs.replaceChildren(
          ...copy.codex_tabs.map(([id, label]) => {
            const b = make("button", id === tab ? "small tab on" : "small tab", label);
            b.addEventListener("click", () => {
              tab = id;
              draw();
            });
            return b;
          }),
        );
        el("codex-body").replaceChildren(...pages[tab](cx));
        el("codex-body").scrollTop = 0;
      };
      draw();
    },

    intro(s) {
      el("i-case").textContent = s.case;
      lines(el("i-lines"), s.sentencing, "line");
      Array.from(el("i-lines").children).forEach((p, k) => p.style.setProperty("--k", k));
    },

    holding(s) {
      el("m-clemency").textContent = s.clemency;
      el("m-deaths").textContent = s.deaths;
      el("m-years").textContent = s.years;
      el("m-years-label").textContent = `${copy.sentence} (${copy.years})`;
      el("m-counts").textContent = `${s.counts_read} / ${s.counts_total}`;
      const b = s.build;
      const body = s.build_lines.slice();
      el("m-bonus").textContent = s.sentence_bonus;
      if (b.grapple) body.push(copy.build_grapple);
      if (b.parry) body.push(copy.build_parry);
      if (b.recess) body.push(copy.build_recess);
      if (b.wall) body.push(copy.build_wall);
      if (b.pound) body.push(copy.build_pound);
      el("m-build").replaceChildren(...body.map((t) => make("li", "", t)));
      // the gun a run carries in: the Carbine, or one an Exhibit bought
      el("carry").hidden = s.carry.length <= 1;
      el("carry-guns").replaceChildren(
        ...s.carry.map((g) => {
          const button = make("button", `carry-gun ${g.chosen ? "chosen" : ""}`);
          const icon = make("img", "");
          icon.src = `art/guns/${g.id}.png`;
          icon.alt = "";
          button.append(icon, make("span", "", g.name));
          button.setAttribute("aria-pressed", g.chosen ? "true" : "false");
          button.addEventListener("click", () => {
            if (game.carry(g.id)) changed();
          });
          return button;
        }),
      );
      // the bars the Chumbles practice may begin from, behind the paw
      el("chumbles-bars").replaceChildren(
        ...s.chumbles_bars.map((name, bar) => {
          const button = make("button", "secret bar", name);
          button.addEventListener("click", () => tryChumbles(bar));
          return button;
        }),
      );

      el("start-rush").hidden = !s.rush;
      el("start-mission").hidden = !s.mission;
      el("mission-note").hidden = !s.mission;
      // the shop as a passive tree: each branch a wedge bursting out from
      // the ring round the hub, every place and line where the core puts it
      const t = s.tree;
      const svg = el("tree-lines");
      const ns = svg.namespaceURI;
      const shape = (tag, cls, attrs) => {
        const node = document.createElementNS(ns, tag);
        node.setAttribute("class", cls);
        for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
        return node;
      };
      svg.setAttribute("viewBox", t.view);
      svg.replaceChildren(
        ...t.wedges.map((w) => shape("path", `wedge ${w.branch}`, { d: w.d })),
        shape("circle", "ring", { cx: t.centre, cy: t.centre, r: t.ring }),
        ...t.edges.map((e) =>
          shape("line", `edge ${e.branch} ${e.lit ? "lit" : ""} ${e.open ? "open" : ""}`, { x1: e.x1, y1: e.y1, x2: e.x2, y2: e.y2 }),
        ),
        shape("circle", "hub", { cx: t.centre, cy: t.centre, r: t.hub }),
      );
      el("tree-labels").replaceChildren(
        ...t.wedges.map((w) => {
          const label = make("span", `branch-name ${w.branch}`, w.name);
          label.style.left = w.x;
          label.style.top = w.y;
          label.style.rotate = w.turn;
          return label;
        }),
      );
      el("tree-clemency").textContent = s.clemency;
      if (!s.upgrades.some((u) => u.id === picked)) picked = (s.upgrades.find((u) => u.can) || s.upgrades[0]).id;
      // the card under the tree reads the upgrade pointed at, or the one picked
      const read = (u) => {
        el("tree-name").textContent = u.name;
        el("tree-text").textContent = u.text;
        el("tree-needs").textContent = u.needs ? `${copy.codex_needs} ${u.needs}` : "";
        const buy = el("tree-buy");
        buy.textContent = u.owned ? copy.owned : u.can ? copy.buy.replace("{n}", u.cost) : u.why;
        buy.disabled = !u.can || u.id !== picked;
        buy.classList.toggle("can", u.can);
      };
      const chosen = () => s.upgrades.find((u) => u.id === picked);
      el("tree-buy").onclick = () => {
        if (game.buy(picked)) changed();
      };
      el("upgrades").replaceChildren(
        ...s.upgrades.map((u) => {
          const node = make("button", `upgrade ${u.branch} ${u.owned ? "owned" : ""} ${u.can ? "can" : ""} ${u.id === picked ? "picked" : ""}`);
          node.dataset.id = u.id;
          node.dataset.ring = u.ring;
          node.style.left = u.x;
          node.style.top = u.y;
          node.setAttribute("aria-label", u.name);
          node.setAttribute("aria-pressed", u.id === picked ? "true" : "false");
          const icon = make("img", "icon");
          icon.src = `art/icons/${u.icon}.png`;
          icon.alt = "";
          node.append(icon);
          node.addEventListener("mouseenter", () => read(u));
          node.addEventListener("focus", () => read(u));
          node.addEventListener("mouseleave", () => read(chosen()));
          node.addEventListener("blur", () => read(chosen()));
          node.addEventListener("click", () => {
            picked = u.id;
            changed();
          });
          return node;
        }),
      );
      read(chosen());

      const counts = el("m-indictment");
      if (s.counts.length) counts.replaceChildren(...s.counts.map((t) => make("li", "", t)));
      else counts.replaceChildren(make("li", "none", copy.no_counts));
      el("m-route").replaceChildren(
        ...s.zones.map((z) => {
          const li = make("li", z.built ? "built" : "sealed");
          li.append(make("strong", "", `${z.year} · ${z.name}`), make("span", "", ` · ${copy.boss_label}: ${z.boss}`));
          if (!z.built) li.append(make("em", "", ` · ${copy.sealed}`));
          return li;
        }),
      );
    },

    // the map: the dioramas are the core's; the page labels each where the
    // core says it lands, to the outside of its line, and lights it on hover
    atlas(s) {
      const tl = s.timeline;
      el("timeline-lines").replaceChildren(
        ...tl.lines.map((l) => {
          const button = make("button", `small line ${l.id} ${l.chosen ? "on" : ""}`, `${copy.timeline_choose}: ${l.name}`);
          button.disabled = !l.open;
          button.title = l.why;
          button.setAttribute("aria-pressed", l.chosen ? "true" : "false");
          button.addEventListener("click", () => {
            if (game.choose_timeline(l.id)) changed();
          });
          return button;
        }),
      );
      el("timeline-nodes").replaceChildren(
        ...tl.nodes.map((n) => {
          const node = make(
            "button",
            `year ${n.line} ${n.side} ${n.open ? "open" : ""} ${n.beaten ? "beaten" : ""} ${n.sealed ? "sealed" : ""}`,
          );
          node.style.left = `${n.x}%`;
          node.style.top = `${n.y}%`;
          node.append(make("strong", "", n.year), make("span", "", n.name));
          if (n.boss) node.append(make("em", "", n.boss));
          if (!n.open) node.append(make("small", "", n.why));
          node.disabled = n.sealed;
          node.addEventListener("mouseenter", () => game.point_at(n.zone));
          node.addEventListener("focus", () => game.point_at(n.zone));
          node.addEventListener("mouseleave", () => game.point_at(-1));
          node.addEventListener("blur", () => game.point_at(-1));
          node.addEventListener("click", () => {
            if (game.show_zone(n.zone)) changed();
          });
          return node;
        }),
      );
    },

    // a zone's walkthrough: its card, and the stop the camera is at
    tour(s) {
      const t = s.tour;
      if (!t) return;
      el("t-year").textContent = t.year;
      el("t-name").textContent = t.name;
      el("t-blurb").textContent = t.blurb;
      el("t-stops").replaceChildren(
        ...t.stops.map((p, k) => {
          const li = make("li", p.here ? "here" : "");
          const b = make("button", "small", p.name);
          b.addEventListener("click", () => {
            game.tour_to(k);
            changed();
          });
          li.append(b);
          return li;
        }),
      );
      el("t-creatures").textContent = t.creatures.join(" · ");
      el("t-boss").textContent = t.boss;
      el("t-boss-line").textContent = t.boss_line;
      el("tour-start").disabled = !t.open;
      el("t-why").textContent = t.why;
      const here = t.stops[t.at];
      el("t-stop-name").textContent = here.name;
      el("t-stop-line").textContent = here.line;
      // the bar fills over the stop: a fresh element restarts its fill
      const bar = make("i", "");
      bar.id = "t-bar";
      bar.style.setProperty("--ticks", t.stop_ticks);
      el("t-bar").replaceWith(bar);
      const caption = el("t-caption");
      caption.classList.remove("shown");
      void caption.offsetWidth;
      caption.classList.add("shown");
    },

    map(s) {
      const m = s.map;
      if (!m) return;
      el("map-heading").textContent = m.heading;
      const svg = el("edges");
      // the SVG namespace, read off the element so no URL is written here
      const ns = svg.namespaceURI;
      svg.replaceChildren(
        ...m.edges.map(([a, b]) => {
          const line = document.createElementNS(ns, "line");
          const [na, nb] = [m.nodes[a], m.nodes[b]];
          line.setAttribute("x1", na.x);
          line.setAttribute("y1", na.y);
          line.setAttribute("x2", nb.x);
          line.setAttribute("y2", nb.y);
          const main = !na.detour && !nb.detour;
          line.setAttribute("class", `${main ? "main" : "side"} ${na.visited && nb.visited ? "walked" : ""}`);
          return line;
        }),
      );
      el("nodes").replaceChildren(
        ...m.nodes.map((n) => {
          const node = make("div", `node ${n.kind} ${n.detour ? "side" : "main"} ${n.here ? "here" : ""} ${n.visited ? "visited" : ""} ${n.next ? "next" : ""}`);
          node.style.left = `${n.x}%`;
          node.style.top = `${n.y}%`;
          const dot = make("span", "dot");
          if (n.icon) {
            const img = make("img", "");
            img.src = `art/icons/${n.icon}.png`;
            img.alt = "";
            dot.append(img);
          }
          node.append(dot, make("span", "name", n.name));
          if (n.tier) node.append(make("span", "tier", n.tier));
          if (n.reward) node.append(make("span", "reward", n.reward));
          if (n.here) node.append(make("span", "here-tag", copy.here));
          return node;
        }),
      );
    },

    interlude(s) {
      const v = s.interlude;
      if (!v) return;
      el("l-art").src = `art/${v.art}.png`;
      el("l-title").textContent = v.title;
      lines(el("l-lines"), v.lines, "line");
      Array.from(el("l-lines").children).forEach((p, k) => p.style.setProperty("--k", k));
      el("l-newest").textContent = v.newest;
      el("l-newest").hidden = !v.newest;
      el("l-next").textContent = v.next;
      el("interlude-next").textContent = v.go;
    },

    dead(s) {
      const r = s.reading;
      if (!r) return;
      el("d-title").textContent = r.title;
      el("d-case").textContent = s.case;
      const earlier = r.counts.filter((c) => c !== r.newest);
      el("d-counts").replaceChildren(...earlier.map((t) => make("li", "", t)));
      el("d-newest").textContent = r.newest;
      el("d-served").textContent = r.time_served;
      el("d-grant").textContent = [r.grant, r.bonus].filter((t) => t).join(" ");
      el("d-earned").textContent = `${copy.earned.replace("{n}", r.clemency)} · ${copy.cleared.replace("{n}", r.cleared)}`;
    },

    won(s) {
      const v = s.verdict;
      if (!v) return;
      el("w-art").src = `art/${v.art}.png`;
      el("w-title").textContent = v.title;
      lines(el("w-lines"), v.lines, "line");
      el("w-newest").textContent = v.newest;
      el("w-newest").hidden = !v.newest;
      Array.from(el("w-lines").children).forEach((p, k) => p.style.setProperty("--k", k));
      el("w-earned").textContent = `${copy.earned.replace("{n}", v.clemency)} · ${copy.cleared.replace("{n}", v.cleared)}`;
    },
  };
}
