"""The project's own browser interactions, run by check.py in every engine.

check.py has already loaded the page, waited for it to report ready, and let
it idle. These drive the game the way a person would and assert what the page
shows. Expected values are worked out by hand from the data files, never
copied from the game's own output:

- story.json's sentencing has 5 lines; its indictment has 10 counts.
- the Codex has 5 tabs (copy.json codex_tabs), 11 control rows
  (copy.json controls), 3 guns (weapons.json) and 7 enemies (enemies.json).
- the crater's gate is open, so its label shows; the first fight's gates
  stay dark and unlabelled until its enemies are defeated.
- upgrades.json lists 11 upgrades; the cheapest (Constitution I) costs 8, so
  a new record with 0 Clemency can buy none of them.
- zone.json lists 4 zones, 1 built: 3 are sealed.
- a run starts in the arrival node, named "The Crater" in zone.json; with
  no upgrades the body has 100 health (balance.rs BASE_HEALTH), 1 jump
  (BASE_JUMPS) and 2 dashes (DASH_CHARGES).
- the main path is the arrival, 5 main fights (route.main_fights), the rest
  and the boss: 8 main nodes on the map.
- abandoning a run counts as a death: the first count is read, the
  sentence is 1 year (Sifu's count: the first death adds 1), deaths 1.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]
COPY = json.loads((ROOT / "data" / "copy.json").read_text())
STORY = json.loads((ROOT / "data" / "story.json").read_text())


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
    check(len(cards) == 11 and len(buyable) == 0, f"11 upgrades, none affordable with 0 Clemency ({len(cards)}, {len(buyable)})")
    check(page.inner_text("#m-counts") == "0 / 10", f"no count read yet ({page.inner_text('#m-counts')!r})")
    sealed = page.query_selector_all("#m-route li.sealed")
    check(len(sealed) == 3, f"three zones sealed in this build ({len(sealed)})")

    click(page, "#holding .open-codex")
    check(page.is_visible("#codex"), "the Codex opens from the Holding")
    tabs = page.query_selector_all("#codex-tabs .tab")
    rows = page.query_selector_all("#codex-body table.stats tr")
    check(len(tabs) == 5 and len(rows) == 11, f"5 tabs, 11 control rows ({len(tabs)}, {len(rows)})")
    click(page, "#codex-tabs .tab:nth-child(2)")
    guns = page.query_selector_all("#codex-body .codex-card")
    check(len(guns) == 3, f"three guns ({len(guns)})")
    click(page, "#codex-tabs .tab:nth-child(3)")
    foes = page.query_selector_all("#codex-body .codex-card")
    check(len(foes) == 7, f"seven enemies ({len(foes)})")
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
