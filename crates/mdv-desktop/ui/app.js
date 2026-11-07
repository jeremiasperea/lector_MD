const { invoke } = window.__TAURI__.core;
const content = document.getElementById("content");
const name = document.getElementById("name");
const status = document.getElementById("status");

async function show(path) {
  status.textContent = "";
  try {
    const doc = await invoke("open_file", { path });
    content.innerHTML = doc.html; // Sanitized by mdv-core.
    name.textContent = doc.name;
    window.scrollTo(0, 0);
  } catch (error) {
    content.textContent = String(error);
    name.textContent = "";
  }
}

document.getElementById("open").addEventListener("click", async () => {
  const path = await invoke("pick_file");
  if (path) await show(path);
});

// Links must not navigate the app window away from the reader: in-page anchors
// scroll as usual, and the backend opens only browser and mail links. Anything
// else, such as a relative link, is refused and reported in the header. A link
// with an empty or missing href is ignored without a message.
content.addEventListener("click", (event) => {
  const link = event.target.closest("a");
  if (!link) return;
  const href = link.getAttribute("href");
  if (href?.startsWith("#")) return;
  event.preventDefault();
  status.textContent = "";
  if (href) {
    invoke("open_external", { url: href }).catch((error) => {
      status.textContent = String(error);
    });
  }
});

// Reading width keeps lines comfortable; full width uses the whole window.
// The choice persists across launches when storage is available; a storage
// failure only loses the preference, never blocks opening the document.
const WIDTH_KEY = "mdv.fullWidth";
const widthButton = document.getElementById("width");

function applyFullWidth(fullWidth) {
  document.body.classList.toggle("full-width", fullWidth);
  widthButton.setAttribute("aria-pressed", String(fullWidth));
}

function saveFullWidth(fullWidth) {
  try {
    localStorage.setItem(WIDTH_KEY, String(fullWidth));
  } catch {}
}

function savedFullWidth() {
  try {
    return localStorage.getItem(WIDTH_KEY) === "true";
  } catch {
    return false;
  }
}

widthButton.addEventListener("click", () => {
  const fullWidth = !document.body.classList.contains("full-width");
  applyFullWidth(fullWidth);
  saveFullWidth(fullWidth);
});
applyFullWidth(savedFullWidth());

invoke("initial_file").then((path) => path && show(path));
