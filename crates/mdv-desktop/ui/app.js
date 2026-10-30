const { invoke } = window.__TAURI__.core;
const content = document.getElementById("content");
const name = document.getElementById("name");

async function show(path) {
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

// Links must not navigate the app window away from the reader.
content.addEventListener("click", (event) => {
  const link = event.target.closest("a");
  if (link && !link.getAttribute("href")?.startsWith("#")) event.preventDefault();
});

invoke("initial_file").then((path) => path && show(path));
