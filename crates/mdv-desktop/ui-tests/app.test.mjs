// Runs the real ui/app.js against stubbed DOM and Tauri APIs:
// node --test crates/mdv-desktop/ui-tests/app.test.mjs
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { setImmediate as flush } from "node:timers/promises";
import vm from "node:vm";

const source = readFileSync(new URL("../ui/app.js", import.meta.url), "utf8");

function loadApp(openExternal = async () => {}, storage = new Map()) {
  const elements = {};
  const element = () => {
    const listeners = {};
    const classes = new Set();
    const attributes = {};
    return {
      textContent: "",
      attributes,
      classList: { toggle: (name, on) => (on ? classes.add(name) : classes.delete(name)), contains: (name) => classes.has(name) },
      setAttribute: (key, value) => (attributes[key] = value),
      addEventListener: (type, listener) => (listeners[type] = listener),
      dispatch: (type, event) => listeners[type](event),
    };
  };
  const body = element();
  const localStorage = {
    getItem: (key) => storage.get(key) ?? null,
    setItem: (key, value) => storage.set(key, String(value)),
  };
  const calls = [];
  const invoke = (command, args) => {
    calls.push({ command, args });
    return command === "open_external" ? openExternal(args) : Promise.resolve(null);
  };
  vm.runInNewContext(source, {
    console,
    document: { body, getElementById: (id) => (elements[id] ??= element()) },
    localStorage,
    window: { __TAURI__: { core: { invoke } }, scrollTo() {} },
  });
  return { body, storage, width: elements.width, commands: () => calls.map((c) => c.command), calls: () => calls.filter((c) => c.command === "open_external"), status: elements.status, click: (link) => {
    const event = {
      prevented: false,
      preventDefault() { this.prevented = true; },
      target: { closest: () => link },
    };
    elements.content.dispatch("click", event);
    return event;
  } };
}

const link = (href) => ({ getAttribute: () => href });

test("clicks outside links are left alone", () => {
  const app = loadApp();
  assert.equal(app.click(null).prevented, false);
  assert.deepEqual(app.calls(), []);
});

test("in-page anchors keep their default scrolling", () => {
  const app = loadApp();
  assert.equal(app.click(link("#section")).prevented, false);
  assert.deepEqual(app.calls(), []);
});

test("other links stay in the reader and go to the backend", () => {
  const app = loadApp();
  assert.equal(app.click(link("https://a.dev")).prevented, true);
  assert.equal(app.calls().length, 1);
  assert.equal(app.calls()[0].args.url, "https://a.dev");
});

for (const [label, href] of [["empty", ""], ["missing", null]]) {
  test(`a link with a ${label} href cannot navigate the window`, () => {
    const app = loadApp();
    assert.equal(app.click(link(href)).prevented, true);
    assert.deepEqual(app.calls(), []);
  });
}

test("a refused link is reported, and the next link click clears it", async () => {
  const app = loadApp(async () => {
    throw "refused to open notes.md";
  });
  app.click(link("notes.md"));
  await flush();
  assert.equal(app.status.textContent, "refused to open notes.md");

  app.click(link(""));
  assert.equal(app.status.textContent, "");
});

test("documents start at reading width", () => {
  const app = loadApp();
  assert.equal(app.body.classList.contains("full-width"), false);
  assert.equal(app.width.attributes["aria-pressed"], "false");
});

test("the width button toggles full width and remembers the choice", () => {
  const app = loadApp();
  app.width.dispatch("click");
  assert.equal(app.body.classList.contains("full-width"), true);
  assert.equal(app.width.attributes["aria-pressed"], "true");
  assert.equal(app.storage.get("mdv.fullWidth"), "true");

  app.width.dispatch("click");
  assert.equal(app.body.classList.contains("full-width"), false);
  assert.equal(app.storage.get("mdv.fullWidth"), "false");
});

test("the remembered width is restored on startup", () => {
  const app = loadApp(undefined, new Map([["mdv.fullWidth", "true"]]));
  assert.equal(app.body.classList.contains("full-width"), true);
  assert.equal(app.width.attributes["aria-pressed"], "true");
});

test("a storage failure keeps reading width and still opens the initial file", () => {
  const broken = { get() { throw new Error("denied"); }, set() { throw new Error("denied"); } };
  const app = loadApp(undefined, broken);
  assert.equal(app.body.classList.contains("full-width"), false);
  assert.ok(app.commands().includes("initial_file"));
  app.width.dispatch("click");
  assert.equal(app.body.classList.contains("full-width"), true);
});
