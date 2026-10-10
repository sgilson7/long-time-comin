// The screens between fights: the sentencing, the Holding with its shop
// and case file, the route map, the indictment read on a death, and the
// verdict. Each draws what the core's screen() says.

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

export function makeScreens(game, copy, changed) {
  return {
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
      const body = [
        copy.build_health.replace("{n}", b.health),
        copy.build_jumps.replace("{n}", b.jumps),
        copy.build_dashes.replace("{n}", b.dashes),
        copy.build_speed.replace("{n}", b.speed),
        b.gun,
      ];
      if (b.grapple) body.push(copy.build_grapple);
      if (b.parry) body.push(copy.build_parry);
      if (b.recess) body.push(copy.build_recess);
      el("m-build").replaceChildren(...body.map((t) => make("li", "", t)));

      const shop = el("upgrades");
      shop.replaceChildren(
        ...s.upgrades.map((u) => {
          const card = make("button", `upgrade ${u.owned ? "owned" : ""} ${u.can ? "can" : ""}`);
          card.dataset.id = u.id;
          const icon = make("img", "icon");
          icon.src = `art/icons/${u.icon}.png`;
          icon.alt = "";
          const label = u.owned ? copy.owned : u.can ? copy.buy.replace("{n}", u.cost) : u.why;
          card.append(icon, make("h4", "", u.name), make("p", "", u.text), make("span", "cost", label));
          card.disabled = !u.can;
          card.addEventListener("click", () => {
            if (game.buy(u.id)) changed();
          });
          return card;
        }),
      );

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

    map(s) {
      const m = s.map;
      if (!m) return;
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
          if (n.reward) node.append(make("span", "reward", n.reward));
          if (n.here) node.append(make("span", "here-tag", copy.here));
          return node;
        }),
      );
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
      el("d-earned").textContent = `${copy.earned.replace("{n}", r.clemency)} · ${copy.cleared.replace("{n}", r.cleared)}`;
    },

    won(s) {
      const v = s.verdict;
      if (!v) return;
      el("w-title").textContent = v.title;
      lines(el("w-lines"), v.lines, "line");
      Array.from(el("w-lines").children).forEach((p, k) => p.style.setProperty("--k", k));
      el("w-earned").textContent = `${copy.earned.replace("{n}", v.clemency)} · ${copy.cleared.replace("{n}", v.cleared)}`;
    },
  };
}
