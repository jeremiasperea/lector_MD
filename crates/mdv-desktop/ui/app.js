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
// else, such as a relative link, is refused and reported in the header.
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

invoke("initial_file").then((path) => path && show(path));
