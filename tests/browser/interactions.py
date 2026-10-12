"""The project's own browser interactions, run by check.py in every engine.

check.py has already loaded the page, waited for it to report ready, and let
it idle. These drive the game the way a person would and assert what the page
shows. Expected values are worked out by hand from the data files, never
copied from the game's own output:

- story.json's sentencing has 5 lines; its indictment has 10 counts.
- the Codex has 5 tabs (copy.json codex_tabs), 17 control rows
  (copy.json controls), 5 guns (weapons.json) and 55 enemies (enemies.json:
  the 43 of the first four zones, then zone five's seven, the Recursion's
  two smaller orders, the Idiot God, and the last mission's ball of yarn
  and Chumbles).
- the crater's gate is open, so its label shows; the first fight's gates
  stay dark and unlabelled until its enemies are defeated.
- upgrades.json lists 32 upgrades in 4 branches (copy.json holding_branches);
  the cheapest (Constitution I) costs 8, so a new record with 0 Clemency can
  buy none of them.
- zone.json lists 5 zones, all built: none is sealed (the last mission's
  home is listed after them and not shown on the Holding's route).
- a run starts in the arrival node, named "The Crater" in zone.json; with
  no upgrades the body has 100 health (balance.rs BASE_HEALTH), 1 jump
  (BASE_JUMPS) and 2 dashes (DASH_CHARGES).
- the main path is the arrival, 5 main fights (route.main_fights), the rest
  and the boss: 8 main nodes on the map.
- abandoning a run counts as a death: the first count is read, the
  sentence is 1 year (Sifu's count: the first death adds 1), deaths 1.
- a record with zone two open (unlocked 1) offers one more start in the
  Holding; it starts on the ice, at "The Landing" of "The White Waste"
  (zone.json), whose arrival holds two rewards to choose from.
- a record with zone three open (unlocked 2) offers two; the second starts
  in "The Vestibule" of "The Glass House", and a death there is headed by
  story.json's death title for the house.
- the faint practice marks in the Holding start in the last zone a run
  crosses (the god's) and in the last mission's home (Chumbles), and
  abandoning either returns to the Holding with no death added.
- the test save (clicked twice, as it asks) owns all 32 upgrades and opens
  the boss rush, whose first arena is the Bailiff's, "The Court of Hours".
- it has put the Idiot God to sleep, so the last mission is open: its brief
  is story.json's mission brief, and after it the run starts in the crater.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]
COPY = json.loads((ROOT / "data" / "copy.json").read_text())
STORY = json.loads((ROOT / "data" / "story.json").read_text())
ZONES = json.loads((ROOT / "data" / "zone.json").read_text())["zones"]
LAST_ZONE = [z for z in ZONES if not z.get("mission")][-1]["name"]
HOME = [z for z in ZONES if z.get("mission")][0]["name"]


def click(page, selector):
    page.click(selector, timeout=5000)
    page.wait_for_timeout(250)


def mode(page):
    return page.evaluate("() => document.body.dataset.mode")


def run(page, check, engine):
    message = page.inner_text("#message")
    check(message in ("", COPY["no_webgl"]), f"no error message is shown ({message!r})")
    check(mode(page) == "title", f"the game opens on the title ({mode(page)})")
    check(page.text_content("#title h1") == COPY["title"], "the title from data/copy.json is drawn")

    click(page, "#begin")
    check(mode(page) == "intro", "a first visit reads the sentencing")
    lines = page.query_selector_all("#i-lines .line")
    check(len(lines) == 5, f"the sentencing has 5 lines ({len(lines)})")
    click(page, "#intro-next")

    check(mode(page) == "holding", "then the Holding")
    cards = page.query_selector_all("#upgrades .upgrade")
    buyable = page.query_selector_all("#upgrades .upgrade:not(:disabled)")
    check(len(cards) == 32 and len(buyable) == 0, f"32 upgrades, none affordable with 0 Clemency ({len(cards)}, {len(buyable)})")
    branches = page.query_selector_all("#upgrades .branch")
    check(len(branches) == 4, f"in four branches ({len(branches)})")
    check(page.inner_text("#m-counts") == "0 / 10", f"no count read yet ({page.inner_text('#m-counts')!r})")
    sealed = page.query_selector_all("#m-route li.sealed")
    check(len(sealed) == 0, f"no zone sealed in this build ({len(sealed)})")
    later = page.query_selector_all("#start-later button")
    check(len(later) == 0, f"a new record can start only in 2031 ({len(later)})")

    click(page, "#holding .open-codex")
    check(page.is_visible("#codex"), "the Codex opens from the Holding")
    tabs = page.query_selector_all("#codex-tabs .tab")
    rows = page.query_selector_all("#codex-body table.stats tr")
    check(len(tabs) == 5 and len(rows) == 17, f"5 tabs, 17 control rows ({len(tabs)}, {len(rows)})")
    click(page, "#codex-tabs .tab:nth-child(2)")
    guns = page.query_selector_all("#codex-body .codex-card")
    check(len(guns) == 5, f"five guns ({len(guns)})")
    click(page, "#codex-tabs .tab:nth-child(3)")
    foes = page.query_selector_all("#codex-body .codex-card")
    check(len(foes) == 55, f"fifty-five enemies ({len(foes)})")
    body = page.inner_text("#codex-body")
    check("{" not in body, "no unfilled placeholder in the Codex")
    click(page, "#codex-close")
    check(page.is_hidden("#codex"), "and closes")

    click(page, "#start-run")
    check(mode(page) == "play", f"starting a run plays ({mode(page)})")
    page.wait_for_timeout(500)
    check(page.inner_text("#h-node") == "The Crater", f"the run starts in the crater ({page.inner_text('#h-node')!r})")
    check(page.inner_text("#h-health-text") == "100 / 100", f"with 100 health ({page.inner_text('#h-health-text')!r})")
    jumps = page.query_selector_all("#h-jumps i")
    dashes = page.query_selector_all("#h-dash i")
    check(len(jumps) == 1 and len(dashes) == 2, f"one jump and two dashes ({len(jumps)}, {len(dashes)})")
    if not page.is_hidden("#engage"):
        click(page, "#engage-btn")
    labels = page.query_selector_all("#gate-labels .gate-label")
    check(len(labels) >= 1, f"the crater's open gate is labelled ({len(labels)})")

    # run north through the crater's gate into the first fight
    check(mode(page) == "play", f"still playing before the walk ({mode(page)})")
    page.keyboard.down("KeyW")
    try:
        # generous: on a software renderer (CI) the page runs at a few
        # frames a second and the simulation's catch-up is capped per frame
        page.wait_for_function("() => document.getElementById('h-node').textContent !== 'The Crater'", timeout=120000)
        moved = True
    except Exception:
        moved = False
    page.keyboard.up("KeyW")
    node = page.inner_text("#h-node")
    check(moved, f"walking north through the gate reaches the next arena ({node!r}, mode {mode(page)})")
    page.wait_for_timeout(300)
    labels = page.query_selector_all("#gate-labels .gate-label")
    check(len(labels) == 0, f"a fight's gates say nothing until it is won ({len(labels)})")
    if mode(page) != "play":
        click(page, "#resume")

    page.keyboard.press("Tab")
    page.wait_for_timeout(300)
    check(mode(page) == "map", f"Tab opens the map ({mode(page)})")
    mains = page.query_selector_all("#nodes .node.main")
    check(len(mains) == 8, f"the main path has 8 nodes ({len(mains)})")
    page.keyboard.press("Tab")
    page.wait_for_timeout(300)
    check(mode(page) == "play", f"Tab again closes it ({mode(page)})")

    page.keyboard.press("Escape")
    page.wait_for_timeout(300)
    check(mode(page) == "paused", f"Esc pauses ({mode(page)})")
    click(page, "#abandon")
    click(page, "#abandon")
    check(mode(page) == "dead", f"abandoning reads the indictment ({mode(page)})")
    check(page.inner_text("#d-newest") == STORY["counts"][0], "the first count is read")
    click(page, "#dead-next")
    check(mode(page) == "holding", "and returns to the Holding")
    check(page.inner_text("#m-counts") == "1 / 10", f"one count read ({page.inner_text('#m-counts')!r})")
    check(page.inner_text("#m-deaths") == "1" and page.inner_text("#m-years") == "1", "one death, one year added")

    page.reload()
    page.wait_for_selector("body[data-ready='true']", state="attached", timeout=20000)
    click(page, "#begin")
    check(mode(page) == "holding", "a returning player skips the sentencing")
    check(page.inner_text("#m-deaths") == "1", "and the record was kept")

    # a record that has beaten the Bailiff can start on the ice
    record = page.evaluate("() => localStorage.getItem('long-time-comin.record')")
    opened = json.loads(record)
    opened["unlocked"] = 1
    page.evaluate("(r) => localStorage.setItem('long-time-comin.record', r)", json.dumps(opened))
    page.reload()
    page.wait_for_selector("body[data-ready='true']", state="attached", timeout=20000)
    click(page, "#begin")
    later = page.query_selector_all("#start-later button")
    check(len(later) == 1, f"one more start, on the ice ({len(later)})")
    click(page, "#start-later button")
    check(mode(page) == "play", f"it plays ({mode(page)})")
    page.wait_for_timeout(500)
    check(page.text_content("#h-zone") == "The White Waste", f"in the White Waste ({page.text_content('#h-zone')!r})")
    check(page.inner_text("#h-node") == "The Landing", f"at the Landing ({page.inner_text('#h-node')!r})")
    page.keyboard.press("Escape")
    page.wait_for_timeout(300)
    if mode(page) == "play":
        page.keyboard.press("Escape")
        page.wait_for_timeout(300)
    click(page, "#abandon")
    click(page, "#abandon")
    check(mode(page) == "dead", f"abandoning on the ice reads the indictment ({mode(page)})")
    check(page.inner_text("#d-title") == STORY["death_titles"]["white_waste"], "headed as the ice's death")

    # a record that has beaten the Witness can start in the house
    record = page.evaluate("() => localStorage.getItem('long-time-comin.record')")
    opened = json.loads(record)
    opened["unlocked"] = 2
    page.evaluate("(r) => localStorage.setItem('long-time-comin.record', r)", json.dumps(opened))
    page.reload()
    page.wait_for_selector("body[data-ready='true']", state="attached", timeout=20000)
    click(page, "#begin")
    page.wait_for_selector("#start-later button", timeout=10000)
    later = page.query_selector_all("#start-later button")
    check(len(later) == 2, f"two more starts, on the ice and in the house ({len(later)})")
    click(page, "#start-later button:nth-child(2)")
    check(mode(page) == "play", f"it plays ({mode(page)})")
    page.wait_for_timeout(500)
    check(page.text_content("#h-zone") == "The Glass House", f"in the Glass House ({page.text_content('#h-zone')!r})")
    check(page.inner_text("#h-node") == "The Vestibule", f"at the Vestibule ({page.inner_text('#h-node')!r})")
    page.keyboard.press("Escape")
    page.wait_for_timeout(300)
    if mode(page) == "play":
        page.keyboard.press("Escape")
        page.wait_for_timeout(300)
    click(page, "#abandon")
    click(page, "#abandon")
    check(mode(page) == "dead", f"abandoning in the house reads the indictment ({mode(page)})")
    check(page.inner_text("#d-title") == STORY["death_titles"]["glass_house"], "headed as the house's death")

    # the test save, then the boss rush it opens
    click(page, "#dead-next")
    check(mode(page) == "holding", f"back in the Holding ({mode(page)})")
    # the faint practice mark: the god's song at once, the record untouched
    deaths = page.inner_text("#m-deaths")
    click(page, "#try-lullaby")
    check(mode(page) == "play", f"the lullaby practice plays ({mode(page)})")
    page.wait_for_timeout(500)
    check(page.text_content("#h-zone") == LAST_ZONE, f"inside the god ({page.text_content('#h-zone')!r})")
    page.keyboard.press("Escape")
    page.wait_for_timeout(300)
    if mode(page) == "play":
        page.keyboard.press("Escape")
        page.wait_for_timeout(300)
    click(page, "#abandon")
    click(page, "#abandon")
    check(mode(page) == "holding", f"abandoning the practice returns to the Holding ({mode(page)})")
    check(page.inner_text("#m-deaths") == deaths, f"and counts no death ({page.inner_text('#m-deaths')!r})")
    # the other faint mark: Chumbles, with everything bought
    click(page, "#try-chumbles")
    check(mode(page) == "play", f"the Chumbles practice plays ({mode(page)})")
    page.wait_for_timeout(500)
    check(page.text_content("#h-zone") == HOME, f"at home ({page.text_content('#h-zone')!r})")
    page.keyboard.press("Escape")
    page.wait_for_timeout(300)
    if mode(page) == "play":
        page.keyboard.press("Escape")
        page.wait_for_timeout(300)
    click(page, "#abandon")
    click(page, "#abandon")
    check(mode(page) == "holding", f"abandoning it returns to the Holding ({mode(page)})")
    check(page.inner_text("#m-deaths") == deaths, f"and counts no death ({page.inner_text('#m-deaths')!r})")
    check(page.is_hidden("#start-rush"), "no boss rush before the game is beaten")
    check(page.is_hidden("#start-mission"), "no last mission before the Idiot God sleeps")
    click(page, "#test-save")
    click(page, "#test-save")
    page.wait_for_timeout(300)
    owned = page.query_selector_all("#upgrades .upgrade.owned")
    check(len(owned) == 32, f"the test save owns every upgrade ({len(owned)})")
    check(page.is_visible("#start-rush"), "and opens the boss rush")
    check(page.is_visible("#start-mission"), "and the last mission")
    click(page, "#start-rush")
    check(mode(page) == "play", f"the rush plays ({mode(page)})")
    page.wait_for_timeout(500)
    check(page.inner_text("#h-node") == "The Court of Hours", f"from the Bailiff's court ({page.inner_text('#h-node')!r})")

    # the last mission: its brief, then the crater
    page.keyboard.press("Escape")
    page.wait_for_timeout(300)
    if mode(page) == "play":
        page.keyboard.press("Escape")
        page.wait_for_timeout(300)
    click(page, "#abandon")
    click(page, "#abandon")
    click(page, "#dead-next")
    click(page, "#start-mission")
    brief = STORY["mission"]["brief"]
    check(mode(page) == "interlude", f"the last mission opens on its brief ({mode(page)})")
    check(page.inner_text("#l-title") == brief["title"], f"titled {brief['title']} ({page.inner_text('#l-title')!r})")
    click(page, "#interlude-next")
    check(mode(page) == "play", f"then it plays ({mode(page)})")
    page.wait_for_timeout(500)
    check(page.inner_text("#h-node") == "The Crater", f"from the crater ({page.inner_text('#h-node')!r})")
